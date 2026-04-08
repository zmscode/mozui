use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::Color;
use mozui_text::FontSystem;
use taffy::prelude::*;

use crate::styled::{ComponentSize, Sizable};

pub struct Icon {
    name: IconName,
    weight: IconWeight,
    color: Color,
    size_px: f32,
}

pub fn icon(name: IconName) -> Icon {
    Icon {
        name,
        weight: IconWeight::Regular,
        color: Color::WHITE,
        size_px: 16.0,
    }
}

impl Icon {
    pub fn weight(mut self, weight: IconWeight) -> Self {
        self.weight = weight;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn size_px(mut self, size: f32) -> Self {
        self.size_px = size;
        self
    }

    fn size_for_component(size: ComponentSize) -> f32 {
        match size {
            ComponentSize::XSmall => 12.0,
            ComponentSize::Small => 14.0,
            ComponentSize::Medium => 16.0,
            ComponentSize::Large => 20.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }
}

impl Sizable for Icon {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size_px = Self::size_for_component(size.into());
        self
    }
}

impl Element for Icon {
    fn layout(&self, engine: &mut LayoutEngine, _font_system: &FontSystem) -> taffy::NodeId {
        engine.new_leaf(Style {
            size: Size {
                width: length(self.size_px),
                height: length(self.size_px),
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

        draw_list.push(DrawCommand::Icon {
            name: self.name,
            bounds,
            color: self.color,
            size_px: self.size_px,
        });
    }
}
