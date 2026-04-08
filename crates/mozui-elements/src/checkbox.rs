use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct Checkbox {
    label: Option<String>,
    checked: bool,
    disabled: bool,
    size: ComponentSize,
    check_color: Color,
    border_color: Color,
    checked_bg: Color,
    label_color: Color,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn checkbox(theme: &Theme) -> Checkbox {
    Checkbox {
        label: None,
        checked: false,
        disabled: false,
        size: ComponentSize::Medium,
        check_color: theme.primary_foreground,
        border_color: theme.border,
        checked_bg: theme.primary,
        label_color: theme.foreground,
        on_click: None,
    }
}

impl Checkbox {
    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = Some(text.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn box_size(&self) -> f32 {
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

impl Sizable for Checkbox {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Element for Checkbox {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let box_sz = self.box_size();
        let mut children = Vec::new();

        // Checkbox box
        let box_node = engine.new_leaf(Style {
            size: Size {
                width: length(box_sz),
                height: length(box_sz),
            },
            ..Default::default()
        });
        children.push(box_node);

        // Label text
        if let Some(ref label_text) = self.label {
            let text_style = mozui_text::TextStyle {
                font_size: self.text_size(),
                color: self.label_color,
                ..Default::default()
            };
            let measured = mozui_text::measure_text(label_text, &text_style, None, font_system);
            let text_node = engine.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
                },
                ..Default::default()
            });
            children.push(text_node);
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
        interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let full_bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);

        // Checkbox box
        let box_layout = layouts[*index];
        *index += 1;
        let box_bounds = mozui_style::Rect::new(
            box_layout.x,
            box_layout.y,
            box_layout.width,
            box_layout.height,
        );

        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let radius = self.box_size() * 0.2;
        let hovered = !self.disabled && interactions.is_hovered(full_bounds);

        if self.checked {
            let bg = if hovered {
                self.checked_bg.with_alpha(alpha * 0.85)
            } else {
                self.checked_bg.with_alpha(alpha)
            };
            draw_list.push(DrawCommand::Rect {
                bounds: box_bounds,
                background: Fill::Solid(bg),
                corner_radii: Corners::uniform(radius),
                border: None,
            });
            // Check icon
            draw_list.push(DrawCommand::Icon {
                name: IconName::Check,
                weight: IconWeight::Regular,
                bounds: box_bounds,
                color: self.check_color.with_alpha(alpha),
                size_px: self.box_size() * 0.75,
            });
        } else {
            let border_c = if hovered {
                self.checked_bg.with_alpha(alpha)
            } else {
                self.border_color.with_alpha(alpha)
            };
            draw_list.push(DrawCommand::Rect {
                bounds: box_bounds,
                background: Fill::Solid(Color::TRANSPARENT),
                corner_radii: Corners::uniform(radius),
                border: Some(Border {
                    width: 1.5,
                    color: border_c,
                }),
            });
        }

        // Label
        if let Some(ref label_text) = self.label {
            let text_layout = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Text {
                text: label_text.clone(),
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
        }

        // Click handler on full bounds
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
