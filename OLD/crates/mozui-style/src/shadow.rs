use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
    /// If true, the shadow is drawn inside the element (inset/recessed effect).
    pub inset: bool,
}

impl Shadow {
    pub fn new(offset_x: f32, offset_y: f32, blur: f32, spread: f32, color: Color) -> Self {
        Self {
            offset_x,
            offset_y,
            blur,
            spread,
            color,
            inset: false,
        }
    }

    /// Create an inset (inner) shadow for recessed/pressed-in effects.
    pub fn inset(offset_x: f32, offset_y: f32, blur: f32, spread: f32, color: Color) -> Self {
        Self {
            offset_x,
            offset_y,
            blur,
            spread,
            color,
            inset: true,
        }
    }
}
