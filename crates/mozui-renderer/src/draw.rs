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
}

pub struct DrawList {
    commands: Vec<DrawCommand>,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, command: DrawCommand) {
        self.commands.push(command);
    }

    pub fn commands(&self) -> impl Iterator<Item = &DrawCommand> {
        self.commands.iter()
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
