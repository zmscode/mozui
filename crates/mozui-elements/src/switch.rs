use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use taffy::prelude::*;

pub struct Switch {
    layout_id: LayoutId,
    track_id: LayoutId,
    label_id: LayoutId,
    checked: bool,
    disabled: bool,
    label: Option<String>,
    size: ComponentSize,
    active_color: Color,
    inactive_color: Color,
    label_color: Color,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn switch(theme: &Theme) -> Switch {
    Switch {
        layout_id: LayoutId::NONE,
        track_id: LayoutId::NONE,
        label_id: LayoutId::NONE,
        checked: false,
        disabled: false,
        label: None,
        size: ComponentSize::Medium,
        active_color: theme.primary,
        inactive_color: theme.muted,
        label_color: theme.foreground,
        on_click: None,
    }
}

impl Switch {
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = Some(text.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn track_width(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 28.0,
            ComponentSize::Small => 32.0,
            ComponentSize::Medium => 36.0,
            ComponentSize::Large => 44.0,
            ComponentSize::Custom(px) => px as f32 * 2.0,
        }
    }

    fn track_height(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 14.0,
            ComponentSize::Small => 16.0,
            ComponentSize::Medium => 20.0,
            ComponentSize::Large => 24.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }

    fn thumb_size(&self) -> f32 {
        self.track_height() - 4.0
    }

    fn text_size(&self) -> f32 {
        self.size.input_text_size()
    }
}

impl Sizable for Switch {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Switch {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Element for Switch {
    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let mut children = Vec::new();

        // Track (contains thumb visually, but laid out as a single rect)
        self.track_id = cx.new_leaf(Style {
            size: Size {
                width: length(self.track_width()),
                height: length(self.track_height()),
            },
            ..Default::default()
        });
        children.push(self.track_id);

        // Label
        if let Some(ref label_text) = self.label {
            let text_style = mozui_text::TextStyle {
                font_size: self.text_size(),
                color: self.label_color,
                ..Default::default()
            };
            let measured = mozui_text::measure_text(label_text, &text_style, None, cx.font_system);
            self.label_id = cx.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
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

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let hovered = !self.disabled && cx.interactions.is_hovered(bounds);

        // Track
        let track_bounds = cx.bounds(self.track_id);
        let track_radius = self.track_height() / 2.0;

        let track_color = if self.checked {
            if hovered {
                self.active_color.with_alpha(0.85)
            } else {
                self.active_color
            }
        } else if hovered {
            self.inactive_color.with_alpha(0.85)
        } else {
            self.inactive_color
        };

        cx.draw_list.push(DrawCommand::Rect {
            bounds: track_bounds,
            background: Fill::Solid(track_color.with_alpha(alpha)),
            corner_radii: Corners::uniform(track_radius),
            border: None,
            shadow: None,
        });

        // Thumb
        let thumb_sz = self.thumb_size();
        let thumb_y = track_bounds.origin.y + 2.0;
        let thumb_x = if self.checked {
            track_bounds.origin.x + track_bounds.size.width - thumb_sz - 2.0
        } else {
            track_bounds.origin.x + 2.0
        };

        cx.draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(thumb_x, thumb_y, thumb_sz, thumb_sz),
            background: Fill::Solid(Color::WHITE.with_alpha(alpha)),
            corner_radii: Corners::uniform(thumb_sz / 2.0),
            border: None,
            shadow: None,
        });

        // Label
        if let Some(ref label_text) = self.label {
            let text_bounds = cx.bounds(self.label_id);
            cx.draw_list.push(DrawCommand::Text {
                text: label_text.clone(),
                bounds: text_bounds,
                font_size: self.text_size(),
                color: self.label_color.with_alpha(alpha),
                weight: 400,
                italic: false,
            });
        }

        // Click handler
        if !self.disabled {
            if let Some(ref handler) = self.on_click {
                let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                cx.interactions.register_click(
                    bounds,
                    Box::new(move |cx| unsafe { (*handler_ptr)(cx) }),
                );
            }
        }
    }
}
