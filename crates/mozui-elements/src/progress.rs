use crate::styled::{ComponentSize, Sizable};
use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct Progress {
    value: f32,
    size: ComponentSize,
    track_color: Color,
    fill_color: Color,
}

pub fn progress(theme: &Theme) -> Progress {
    Progress {
        value: 0.0,
        size: ComponentSize::Medium,
        track_color: theme.slider_bar,
        fill_color: theme.progress_bar,
    }
}

impl Progress {
    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(0.0, 100.0);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.fill_color = color;
        self
    }

    fn track_height(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 2.0,
            ComponentSize::Small => 4.0,
            ComponentSize::Medium => 6.0,
            ComponentSize::Large => 8.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }
}

impl Sizable for Progress {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Element for Progress {
    fn layout(&self, engine: &mut LayoutEngine, _font_system: &FontSystem) -> taffy::NodeId {
        engine.new_leaf(Style {
            size: Size {
                width: auto(),
                height: length(self.track_height()),
            },
            flex_grow: 1.0,
            min_size: Size {
                width: length(40.0),
                height: length(self.track_height()),
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

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);
        let h = self.track_height();
        let radius = h / 2.0;

        // Track
        draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.track_color),
            corner_radii: Corners::uniform(radius),
            border: None,
                    shadow: None,
                });

        // Fill
        let ratio = self.value / 100.0;
        let fill_w = bounds.size.width * ratio;
        if fill_w > 0.5 {
            draw_list.push(DrawCommand::Rect {
                bounds: mozui_style::Rect::new(bounds.origin.x, bounds.origin.y, fill_w, h),
                background: Fill::Solid(self.fill_color),
                corner_radii: Corners::uniform(radius),
                border: None,
                    shadow: None,
                });
        }
    }
}
