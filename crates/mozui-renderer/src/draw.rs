use mozui_icons::IconName;
use mozui_style::{Color, Corners, Fill, Rect};

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
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            offset_stack: Vec::new(),
            current_offset_y: 0.0,
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
        self.commands.push(command);
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
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
