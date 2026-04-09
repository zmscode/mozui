use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct Spinner {
    size: f32,
    color: Color,
    label: Option<String>,
    label_color: Color,
}

pub fn spinner(theme: &Theme) -> Spinner {
    Spinner {
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
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut children = Vec::new();

        // Spinner icon
        children.push(engine.new_leaf(Style {
            size: Size {
                width: length(self.size),
                height: length(self.size),
            },
            ..Default::default()
        }));

        // Optional label
        if let Some(ref label_text) = self.label {
            let style = mozui_text::TextStyle {
                font_size: 13.0,
                color: self.label_color,
                ..Default::default()
            };
            let m = mozui_text::measure_text(label_text, &style, None, font_system);
            children.push(engine.new_leaf(Style {
                size: Size {
                    width: length(m.width),
                    height: length(m.height),
                },
                ..Default::default()
            }));
        }

        engine.new_with_children(
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
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        _interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        // Container
        let _container = layouts[*index];
        *index += 1;

        // Spinner icon (SpinnerGap has a visual gap that suggests rotation)
        let icon_layout = layouts[*index];
        *index += 1;
        let icon_bounds = mozui_style::Rect::new(
            icon_layout.x,
            icon_layout.y,
            icon_layout.width,
            icon_layout.height,
        );
        draw_list.push(DrawCommand::Icon {
            name: IconName::SpinnerGap,
            weight: IconWeight::Bold,
            bounds: icon_bounds,
            color: self.color,
            size_px: self.size,
        });

        // Label
        if let Some(ref label_text) = self.label {
            let label_layout = layouts[*index];
            *index += 1;
            let label_bounds = mozui_style::Rect::new(
                label_layout.x,
                label_layout.y,
                label_layout.width,
                label_layout.height,
            );
            draw_list.push(DrawCommand::Text {
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
