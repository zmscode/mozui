#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Size = Size {
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        origin: Point::ZERO,
        size: Size::ZERO,
    };

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.x <= self.origin.x + self.size.width
            && point.y >= self.origin.y
            && point.y <= self.origin.y + self.size.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.origin.x < other.origin.x + other.size.width
            && self.origin.x + self.size.width > other.origin.x
            && self.origin.y < other.origin.y + other.size.height
            && self.origin.y + self.size.height > other.origin.y
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.origin.x.min(other.origin.x);
        let y = self.origin.y.min(other.origin.y);
        let right = (self.origin.x + self.size.width).max(other.origin.x + other.size.width);
        let bottom = (self.origin.y + self.size.height).max(other.origin.y + other.size.height);
        Rect::new(x, y, right - x, bottom - y)
    }

    pub fn inset(&self, amount: f32) -> Rect {
        Rect::new(
            self.origin.x + amount,
            self.origin.y + amount,
            (self.size.width - amount * 2.0).max(0.0),
            (self.size.height - amount * 2.0).max(0.0),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Corners {
    pub const ZERO: Corners = Corners {
        top_left: 0.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: 0.0,
    };

    pub fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    pub fn top(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            ..Self::ZERO
        }
    }

    pub fn bottom(radius: f32) -> Self {
        Self {
            bottom_left: radius,
            bottom_right: radius,
            ..Self::ZERO
        }
    }

    pub fn to_array(self) -> [f32; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }
}

impl From<f32> for Corners {
    fn from(radius: f32) -> Self {
        Corners::uniform(radius)
    }
}

// ---------------------------------------------------------------------------
// Layout geometry helpers (matching gpui-component)
// ---------------------------------------------------------------------------

/// Placement direction for popovers, tooltips, sheets, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Placement {
    #[default]
    Bottom,
    Top,
    Left,
    Right,
}

impl Placement {
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    pub fn opposite(&self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn axis(&self) -> Axis {
        match self {
            Self::Top | Self::Bottom => Axis::Vertical,
            Self::Left | Self::Right => Axis::Horizontal,
        }
    }
}

/// Anchor corner for positioned elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Anchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Anchor {
    pub fn is_top(&self) -> bool {
        matches!(self, Self::TopLeft | Self::TopCenter | Self::TopRight)
    }

    pub fn is_bottom(&self) -> bool {
        matches!(
            self,
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight
        )
    }

    pub fn swap_vertical(&self) -> Self {
        match self {
            Self::TopLeft => Self::BottomLeft,
            Self::TopCenter => Self::BottomCenter,
            Self::TopRight => Self::BottomRight,
            Self::BottomLeft => Self::TopLeft,
            Self::BottomCenter => Self::TopCenter,
            Self::BottomRight => Self::TopRight,
        }
    }

    pub fn swap_horizontal(&self) -> Self {
        match self {
            Self::TopLeft => Self::TopRight,
            Self::TopCenter => Self::TopCenter,
            Self::TopRight => Self::TopLeft,
            Self::BottomLeft => Self::BottomRight,
            Self::BottomCenter => Self::BottomCenter,
            Self::BottomRight => Self::BottomLeft,
        }
    }
}

/// Layout axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::Horizontal)
    }

    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::Vertical)
    }
}

/// Left or right side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
}

/// Edge insets (like CSS padding/margin with per-side values).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub const ZERO: Edges = Edges {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

impl From<f32> for Edges {
    fn from(value: f32) -> Self {
        Self::all(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert!(r.contains(Point::new(50.0, 30.0)));
        assert!(!r.contains(Point::new(5.0, 30.0)));
    }

    #[test]
    fn rect_intersects() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(50.0, 50.0, 100.0, 100.0);
        let c = Rect::new(200.0, 200.0, 10.0, 10.0);
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }
}
