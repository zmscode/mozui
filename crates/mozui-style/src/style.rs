use crate::color::Color;
use crate::fill::Fill;
use crate::geometry::Corners;
use crate::shadow::Shadow;

#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    // Sizing
    pub width: Option<f32>,
    pub height: Option<f32>,

    // Visual
    pub background: Option<Fill>,
    pub corner_radii: Corners,
    pub border_width: f32,
    pub border_color: Color,
    pub shadow: Option<Shadow>,
    pub opacity: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            background: None,
            corner_radii: Corners::ZERO,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            shadow: None,
            opacity: 1.0,
        }
    }
}
