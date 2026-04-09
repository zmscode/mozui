use crate::styled::{ComponentSize, Sizable};
use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

/// Shape variant for skeleton placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonShape {
    /// Rounded rectangle (default).
    Rect,
    /// Circle — width and height are both set to the height value.
    Circle,
    /// Pill — fully rounded corners.
    Pill,
}

pub struct Skeleton {
    width: f32,
    height: f32,
    shape: SkeletonShape,
    color: Color,
    radius: f32,
}

pub fn skeleton(theme: &Theme) -> Skeleton {
    Skeleton {
        width: 100.0,
        height: 16.0,
        shape: SkeletonShape::Rect,
        color: theme.skeleton,
        radius: theme.radius_sm,
    }
}

impl Skeleton {
    pub fn w(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn h(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn shape(mut self, shape: SkeletonShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn circle(mut self) -> Self {
        self.shape = SkeletonShape::Circle;
        self
    }

    pub fn pill(mut self) -> Self {
        self.shape = SkeletonShape::Pill;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Preset: text line placeholder (full width, 14px tall).
    pub fn text_line(theme: &Theme) -> Self {
        skeleton(theme).w(200.0).h(14.0)
    }

    /// Preset: avatar placeholder (circle).
    pub fn avatar(theme: &Theme, size: f32) -> Self {
        skeleton(theme).w(size).h(size).circle()
    }

    /// Preset: button placeholder.
    pub fn button(theme: &Theme) -> Self {
        skeleton(theme).w(80.0).h(32.0).pill()
    }

    fn resolved_size(&self) -> (f32, f32) {
        match self.shape {
            SkeletonShape::Circle => (self.height, self.height),
            _ => (self.width, self.height),
        }
    }

    fn corner_radii(&self) -> Corners {
        match self.shape {
            SkeletonShape::Circle => Corners::uniform(self.height / 2.0),
            SkeletonShape::Pill => Corners::uniform(self.height / 2.0),
            SkeletonShape::Rect => Corners::uniform(self.radius),
        }
    }
}

impl Sizable for Skeleton {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        let cs: ComponentSize = size.into();
        self.height = cs.input_height();
        self
    }
}

impl Element for Skeleton {
    fn layout(&self, engine: &mut LayoutEngine, _font_system: &FontSystem) -> taffy::NodeId {
        let (w, h) = self.resolved_size();
        engine.new_leaf(Style {
            size: Size {
                width: length(w),
                height: length(h),
            },
            ..Default::default()
        })
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        _interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds =
            mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);

        draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.color),
            corner_radii: self.corner_radii(),
            border: None,
            shadow: None,
        });
    }
}
