use mozui_icons::{IconName, IconWeight};
use mozui_style::{Color, Corners, Fill, Rect, Shadow};

#[derive(Debug, Clone)]
pub struct Border {
    pub width: f32,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub enum DrawCommand {
    Rect {
        bounds: Rect,
        background: Fill,
        corner_radii: Corners,
        border: Option<Border>,
        shadow: Option<Shadow>,
    },
    Text {
        text: String,
        bounds: Rect,
        font_size: f32,
        color: Color,
        weight: u32,
        italic: bool,
    },
    Icon {
        name: IconName,
        weight: IconWeight,
        bounds: Rect,
        color: Color,
        size_px: f32,
    },
}

pub struct DrawList {
    commands: Vec<DrawCommand>,
    /// Stack of cumulative Y offsets for scroll containers.
    offset_stack: Vec<f32>,
    /// Current cumulative Y offset.
    current_offset_y: f32,
    /// Stack of clip rects. Commands outside the top clip rect are discarded.
    clip_stack: Vec<Rect>,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            offset_stack: Vec::new(),
            current_offset_y: 0.0,
            clip_stack: Vec::new(),
        }
    }

    pub fn push(&mut self, mut command: DrawCommand) {
        // Apply current scroll offset to the command's bounds
        if self.current_offset_y != 0.0 {
            match &mut command {
                DrawCommand::Rect { bounds, .. }
                | DrawCommand::Text { bounds, .. }
                | DrawCommand::Icon { bounds, .. } => {
                    bounds.origin.y += self.current_offset_y;
                }
            }
        }

        // Clip check: discard commands entirely outside the clip rect
        if let Some(clip) = self.clip_stack.last() {
            let cmd_bounds = match &command {
                DrawCommand::Rect { bounds, .. }
                | DrawCommand::Text { bounds, .. }
                | DrawCommand::Icon { bounds, .. } => bounds,
            };
            let clip_bottom = clip.origin.y + clip.size.height;
            let clip_right = clip.origin.x + clip.size.width;
            let cmd_bottom = cmd_bounds.origin.y + cmd_bounds.size.height;
            let cmd_right = cmd_bounds.origin.x + cmd_bounds.size.width;

            // Discard if entirely outside clip rect
            if cmd_bounds.origin.y >= clip_bottom
                || cmd_bottom <= clip.origin.y
                || cmd_bounds.origin.x >= clip_right
                || cmd_right <= clip.origin.x
            {
                return;
            }
        }

        self.commands.push(command);
    }

    /// Push a clip rect. Commands whose bounds fall entirely outside
    /// this rect will be discarded until the matching `pop_clip()`.
    /// The clip rect is specified in layout coordinates and will have
    /// the current scroll offset applied automatically.
    pub fn push_clip(&mut self, mut clip: Rect) {
        // Apply scroll offset so clip is in the same space as draw commands
        clip.origin.y += self.current_offset_y;

        // Intersect with parent clip rect if any
        let effective = if let Some(parent) = self.clip_stack.last() {
            intersect_rects(parent, &clip)
        } else {
            clip
        };
        self.clip_stack.push(effective);
    }

    /// Pop the most recent clip rect.
    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    /// Push a vertical scroll offset. All subsequent `push` calls will have
    /// their Y coordinates shifted by this amount (cumulative with parent offsets).
    pub fn push_scroll_offset(&mut self, offset_y: f32) {
        self.offset_stack.push(self.current_offset_y);
        self.current_offset_y += offset_y;
    }

    /// Pop the most recent scroll offset, restoring the previous one.
    pub fn pop_scroll_offset(&mut self) {
        if let Some(prev) = self.offset_stack.pop() {
            self.current_offset_y = prev;
        }
    }

    pub fn commands(&self) -> impl Iterator<Item = &DrawCommand> {
        self.commands.iter()
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.offset_stack.clear();
        self.current_offset_y = 0.0;
        self.clip_stack.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Compute the intersection of two rectangles.
fn intersect_rects(a: &Rect, b: &Rect) -> Rect {
    let x = a.origin.x.max(b.origin.x);
    let y = a.origin.y.max(b.origin.y);
    let right = (a.origin.x + a.size.width).min(b.origin.x + b.size.width);
    let bottom = (a.origin.y + a.size.height).min(b.origin.y + b.size.height);
    Rect::new(x, y, (right - x).max(0.0), (bottom - y).max(0.0))
}
