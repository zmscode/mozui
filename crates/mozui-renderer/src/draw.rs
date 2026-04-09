use mozui_icons::{IconName, IconWeight};
use mozui_style::{Color, Corners, Fill, Rect, Shadow};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Border {
    pub width: f32,
    pub color: Color,
}

/// Decoded RGBA image data ready for GPU upload.
#[derive(Debug, Clone)]
pub struct ImageData {
    pub pixels: Vec<u8>, // RGBA, len = width * height * 4
    pub width: u32,
    pub height: u32,
    /// Unique ID for caching. Automatically assigned on creation.
    pub id: u64,
}

static NEXT_IMAGE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl ImageData {
    pub fn new(pixels: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            pixels,
            width,
            height,
            id: NEXT_IMAGE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// How an image should fit within its bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObjectFit {
    /// Scale to fill bounds, preserving aspect ratio, cropping if needed.
    #[default]
    Cover,
    /// Scale to fit within bounds, preserving aspect ratio (letterboxed).
    Contain,
    /// Stretch to fill bounds exactly (may distort).
    Fill,
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
    Image {
        bounds: Rect,
        data: Arc<ImageData>,
        corner_radii: Corners,
        opacity: f32,
        object_fit: ObjectFit,
    },
}

pub struct DrawList {
    commands: Vec<DrawCommand>,
    /// Stack of cumulative Y offsets for scroll containers.
    offset_stack: Vec<f32>,
    /// Current cumulative Y offset.
    current_offset_y: f32,
    /// Stack of (x, y) offset pairs for 2D repositioning (e.g. popovers, hover cards).
    offset_xy_stack: Vec<(f32, f32)>,
    /// Current cumulative X offset from push_offset.
    current_offset_x: f32,
    /// Stack of clip rects. Commands outside the top clip rect are discarded.
    clip_stack: Vec<Rect>,
    /// Stack of opacity multipliers. The effective opacity is the product of all values.
    opacity_stack: Vec<f32>,
    /// Current effective opacity (product of all stack entries).
    current_opacity: f32,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            offset_stack: Vec::new(),
            current_offset_y: 0.0,
            offset_xy_stack: Vec::new(),
            current_offset_x: 0.0,
            clip_stack: Vec::new(),
            opacity_stack: Vec::new(),
            current_opacity: 1.0,
        }
    }

    pub fn push(&mut self, mut command: DrawCommand) {
        // Apply current offsets to the command's bounds
        let has_y = self.current_offset_y != 0.0;
        let has_x = self.current_offset_x != 0.0;
        if has_x || has_y {
            match &mut command {
                DrawCommand::Rect { bounds, .. }
                | DrawCommand::Text { bounds, .. }
                | DrawCommand::Icon { bounds, .. }
                | DrawCommand::Image { bounds, .. } => {
                    bounds.origin.x += self.current_offset_x;
                    bounds.origin.y += self.current_offset_y;
                }
            }
        }

        // Clip check: discard commands entirely outside the clip rect
        if let Some(clip) = self.clip_stack.last() {
            let cmd_bounds = match &command {
                DrawCommand::Rect { bounds, .. }
                | DrawCommand::Text { bounds, .. }
                | DrawCommand::Icon { bounds, .. }
                | DrawCommand::Image { bounds, .. } => bounds,
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

        // Apply opacity to command colors
        if self.current_opacity < 1.0 {
            let opacity = self.current_opacity;
            match &mut command {
                DrawCommand::Rect {
                    background,
                    border,
                    shadow,
                    ..
                } => {
                    match background {
                        Fill::Solid(c) => c.a *= opacity,
                        Fill::LinearGradient { stops, .. } => {
                            for (_, c) in stops.iter_mut() {
                                c.a *= opacity;
                            }
                        }
                        Fill::RadialGradient { stops, .. } => {
                            for (_, c) in stops.iter_mut() {
                                c.a *= opacity;
                            }
                        }
                    }
                    if let Some(b) = border {
                        b.color.a *= opacity;
                    }
                    if let Some(s) = shadow {
                        s.color.a *= opacity;
                    }
                }
                DrawCommand::Text { color, .. } => color.a *= opacity,
                DrawCommand::Icon { color, .. } => color.a *= opacity,
                DrawCommand::Image {
                    opacity: img_opacity,
                    ..
                } => *img_opacity *= opacity,
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

    /// Push an opacity multiplier. All subsequent commands will have their
    /// alpha values multiplied by this factor. Nests multiplicatively.
    pub fn push_opacity(&mut self, opacity: f32) {
        self.opacity_stack.push(self.current_opacity);
        self.current_opacity *= opacity.clamp(0.0, 1.0);
    }

    /// Pop the most recent opacity multiplier, restoring the previous one.
    pub fn pop_opacity(&mut self) {
        if let Some(prev) = self.opacity_stack.pop() {
            self.current_opacity = prev;
        }
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

    /// Push a 2D offset. All subsequent `push` calls will have their
    /// X and Y coordinates shifted by this amount (cumulative with parent offsets).
    /// Used by hover cards, popovers, and other floating elements that
    /// need to reposition child content from layout coords to screen coords.
    pub fn push_offset(&mut self, dx: f32, dy: f32) {
        self.offset_xy_stack
            .push((self.current_offset_x, self.current_offset_y));
        self.current_offset_x += dx;
        self.current_offset_y += dy;
    }

    /// Pop the most recent 2D offset, restoring the previous one.
    pub fn pop_offset(&mut self) {
        if let Some((prev_x, prev_y)) = self.offset_xy_stack.pop() {
            self.current_offset_x = prev_x;
            self.current_offset_y = prev_y;
        }
    }

    pub fn commands(&self) -> impl Iterator<Item = &DrawCommand> {
        self.commands.iter()
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.offset_stack.clear();
        self.current_offset_y = 0.0;
        self.offset_xy_stack.clear();
        self.current_offset_x = 0.0;
        self.clip_stack.clear();
        self.opacity_stack.clear();
        self.current_opacity = 1.0;
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Number of draw commands in the list.
    pub fn len(&self) -> usize {
        self.commands.len()
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
