use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Rect, Theme};
use taffy::prelude::*;

pub struct Spinner {
    layout_id: LayoutId,
    icon_id: LayoutId,
    label_id: LayoutId,
    size: f32,
    color: Color,
    label: Option<String>,
    label_color: Color,
}

pub fn spinner(theme: &Theme) -> Spinner {
    Spinner {
        layout_id: LayoutId::NONE,
        icon_id: LayoutId::NONE,
        label_id: LayoutId::NONE,
        size: 20.0,
        color: theme.primary,
        label: None,
        label_color: theme.muted_foreground,
    }
}

impl Spinner {
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }
}

impl Element for Spinner {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Spinner",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let mut children = Vec::new();

        // Spinner icon
        self.icon_id = cx.new_leaf(Style {
            size: Size {
                width: length(self.size),
                height: length(self.size),
            },
            ..Default::default()
        });
        children.push(self.icon_id);

        // Optional label
        if let Some(ref label_text) = self.label {
            let style = mozui_text::TextStyle {
                font_size: 13.0,
                color: self.label_color,
                ..Default::default()
            };
            let m = mozui_text::measure_text(label_text, &style, None, cx.font_system);
            self.label_id = cx.new_leaf(Style {
                size: Size {
                    width: length(m.width),
                    height: length(m.height),
                },
                ..Default::default()
            });
            children.push(self.label_id);
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: Size {
                    width: length(8.0),
                    height: zero(),
                },
                ..Default::default()
            },
            &children,
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        // Spinner icon (SpinnerGap has a visual gap that suggests rotation)
        let icon_bounds = cx.bounds(self.icon_id);
        cx.draw_list.push(DrawCommand::Icon {
            name: IconName::SpinnerGap,
            weight: IconWeight::Bold,
            bounds: icon_bounds,
            color: self.color,
            size_px: self.size,
        });

        // Label
        if let Some(ref label_text) = self.label {
            let label_bounds = cx.bounds(self.label_id);
            cx.draw_list.push(DrawCommand::Text {
                text: label_text.clone(),
                bounds: label_bounds,
                font_size: 13.0,
                color: self.label_color,
                weight: 400,
                italic: false,
            });
        }
    }
}
