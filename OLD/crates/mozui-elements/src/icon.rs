use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Rect};
use taffy::prelude::*;

use crate::styled::{ComponentSize, Sizable};

pub struct Icon {
    layout_id: LayoutId,
    name: IconName,
    weight: IconWeight,
    color: Color,
    size_px: f32,
}

pub fn icon(name: IconName) -> Icon {
    Icon {
        layout_id: LayoutId::NONE,
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
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Icon",
            layout_id: self.layout_id,
            properties: vec![
                ("name", format!("{:?}", self.name)),
                ("size", format!("{}", self.size_px)),
                ("color", format!("{:?}", self.color)),
            ],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.layout_id = cx.new_leaf(Style {
            size: Size {
                width: length(self.size_px),
                height: length(self.size_px),
            },
            ..Default::default()
        });
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        cx.collect_debug_info(self, bounds);
        cx.draw_list.push(DrawCommand::Icon {
            name: self.name,
            weight: self.weight,
            bounds,
            color: self.color,
            size_px: self.size_px,
        });
    }
}
