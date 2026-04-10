use crate::color::Color;
use crate::geometry::Point;

#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid(Color),
    LinearGradient {
        angle: f32,
        stops: Vec<(f32, Color)>,
    },
    RadialGradient {
        center: Point,
        radius: f32,
        stops: Vec<(f32, Color)>,
    },
}

impl From<Color> for Fill {
    fn from(color: Color) -> Self {
        Fill::Solid(color)
    }
}

impl From<&str> for Fill {
    fn from(hex: &str) -> Self {
        Fill::Solid(Color::hex(hex))
    }
}
