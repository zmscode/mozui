use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct Radio {
    label: String,
    checked: bool,
    disabled: bool,
    size: ComponentSize,
    active_color: Color,
    border_color: Color,
    label_color: Color,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn radio(label: impl Into<String>, theme: &Theme) -> Radio {
    Radio {
        label: label.into(),
        checked: false,
        disabled: false,
        size: ComponentSize::Medium,
        active_color: theme.primary,
        border_color: theme.border,
        label_color: theme.foreground,
        on_click: None,
    }
}

impl Radio {
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn circle_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 12.0,
            ComponentSize::Small => 14.0,
            ComponentSize::Medium => 16.0,
            ComponentSize::Large => 20.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }

    fn text_size(&self) -> f32 {
        self.size.input_text_size()
    }
}

impl Sizable for Radio {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Radio {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Element for Radio {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let circle_sz = self.circle_size();

        // Outer circle
        let circle_node = engine.new_leaf(Style {
            size: Size {
                width: length(circle_sz),
                height: length(circle_sz),
            },
            ..Default::default()
        });

        // Label
        let text_style = mozui_text::TextStyle {
            font_size: self.text_size(),
            color: self.label_color,
            ..Default::default()
        };
        let measured = mozui_text::measure_text(&self.label, &text_style, None, font_system);
        let text_node = engine.new_leaf(Style {
            size: Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            ..Default::default()
        });

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
            &[circle_node, text_node],
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let full_bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let hovered = !self.disabled && interactions.is_hovered(full_bounds);

        // Outer circle
        let circle_layout = layouts[*index];
        *index += 1;
        let circle_bounds = mozui_style::Rect::new(
            circle_layout.x,
            circle_layout.y,
            circle_layout.width,
            circle_layout.height,
        );
        let radius = self.circle_size() / 2.0;

        if self.checked {
            let bg = if hovered {
                self.active_color.with_alpha(alpha * 0.85)
            } else {
                self.active_color.with_alpha(alpha)
            };
            // Filled outer circle
            draw_list.push(DrawCommand::Rect {
                bounds: circle_bounds,
                background: Fill::Solid(bg),
                corner_radii: Corners::uniform(radius),
                border: None,
                    shadow: None,
                });
            // Inner white dot
            let inner_size = self.circle_size() * 0.4;
            let offset = (self.circle_size() - inner_size) / 2.0;
            draw_list.push(DrawCommand::Rect {
                bounds: mozui_style::Rect::new(
                    circle_bounds.origin.x + offset,
                    circle_bounds.origin.y + offset,
                    inner_size,
                    inner_size,
                ),
                background: Fill::Solid(Color::WHITE.with_alpha(alpha)),
                corner_radii: Corners::uniform(inner_size / 2.0),
                border: None,
                    shadow: None,
                });
        } else {
            let border_c = if hovered {
                self.active_color.with_alpha(alpha)
            } else {
                self.border_color.with_alpha(alpha)
            };
            draw_list.push(DrawCommand::Rect {
                bounds: circle_bounds,
                background: Fill::Solid(Color::TRANSPARENT),
                corner_radii: Corners::uniform(radius),
                border: Some(Border {
                    width: 1.5,
                    color: border_c,
                }),
                    shadow: None,
                });
        }

        // Label
        let text_layout = layouts[*index];
        *index += 1;
        draw_list.push(DrawCommand::Text {
            text: self.label.clone(),
            bounds: mozui_style::Rect::new(
                text_layout.x,
                text_layout.y,
                text_layout.width,
                text_layout.height,
            ),
            font_size: self.text_size(),
            color: self.label_color.with_alpha(alpha),
            weight: 400,
            italic: false,
        });

        // Click handler
        if !self.disabled {
            if let Some(ref handler) = self.on_click {
                let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                interactions.register_click(
                    full_bounds,
                    Box::new(move |cx| unsafe { (*handler_ptr)(cx) }),
                );
            }
        }
    }
}
