use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use std::rc::Rc;
use taffy::prelude::*;

pub struct Radio {
    layout_id: LayoutId,
    circle_id: LayoutId,
    text_id: LayoutId,
    label: String,
    checked: bool,
    disabled: bool,
    size: ComponentSize,
    active_color: Color,
    border_color: Color,
    label_color: Color,
    on_click: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn radio(label: impl Into<String>, theme: &Theme) -> Radio {
    Radio {
        layout_id: LayoutId::NONE,
        circle_id: LayoutId::NONE,
        text_id: LayoutId::NONE,
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
        self.on_click = Some(Rc::new(handler));
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
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Radio",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let circle_sz = self.circle_size();

        // Outer circle
        self.circle_id = cx.new_leaf(Style {
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
        let measured = mozui_text::measure_text(&self.label, &text_style, None, cx.font_system);
        self.text_id = cx.new_leaf(Style {
            size: Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            ..Default::default()
        });

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
            &[self.circle_id, self.text_id],
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let hovered = !self.disabled && cx.interactions.is_hovered(bounds);

        // Outer circle
        let circle_bounds = cx.bounds(self.circle_id);
        let radius = self.circle_size() / 2.0;

        if self.checked {
            let bg = if hovered {
                self.active_color.with_alpha(alpha * 0.85)
            } else {
                self.active_color.with_alpha(alpha)
            };
            // Filled outer circle
            cx.draw_list.push(DrawCommand::Rect {
                bounds: circle_bounds,
                background: Fill::Solid(bg),
                corner_radii: Corners::uniform(radius),
                border: None,
                shadow: None,
            });
            // Inner white dot
            let inner_size = self.circle_size() * 0.4;
            let offset = (self.circle_size() - inner_size) / 2.0;
            cx.draw_list.push(DrawCommand::Rect {
                bounds: Rect::new(
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
            cx.draw_list.push(DrawCommand::Rect {
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
        let text_bounds = cx.bounds(self.text_id);
        cx.draw_list.push(DrawCommand::Text {
            text: self.label.clone(),
            bounds: text_bounds,
            font_size: self.text_size(),
            color: self.label_color.with_alpha(alpha),
            weight: 400,
            italic: false,
        });

        // Click handler
        if !self.disabled {
            if let Some(ref handler) = self.on_click {
                cx.interactions.register_click(
                    bounds,
                    handler.clone(),
                );
            }
        }
    }
}
