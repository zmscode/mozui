use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use std::rc::Rc;
use taffy::prelude::*;

const INPUT_HEIGHT: f32 = 34.0;
const BTN_WIDTH: f32 = 28.0;
const ICON_SIZE: f32 = 14.0;
const FONT_SIZE: f32 = 14.0;

/// A numeric input with increment/decrement buttons.
pub struct NumberInput {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    precision: usize,
    disabled: bool,
    width: f32,
    on_change: Option<Rc<dyn Fn(f64, &mut dyn std::any::Any)>>,
    // Theme
    bg: Color,
    fg: Color,
    muted_fg: Color,
    border_color: Color,
    hover_bg: Color,
    corner_radius: f32,
    // Layout IDs
    layout_id: LayoutId,
    dec_id: LayoutId,
    value_id: LayoutId,
    inc_id: LayoutId,
}

pub fn number_input(theme: &Theme) -> NumberInput {
    NumberInput {
        value: 0.0,
        min: f64::MIN,
        max: f64::MAX,
        step: 1.0,
        precision: 0,
        disabled: false,
        width: 140.0,
        on_change: None,
        bg: theme.input_background,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        border_color: theme.border,
        hover_bg: theme.secondary,
        corner_radius: theme.radius_md,
        layout_id: LayoutId::NONE,
        dec_id: LayoutId::NONE,
        value_id: LayoutId::NONE,
        inc_id: LayoutId::NONE,
    }
}

impl NumberInput {
    pub fn value(mut self, v: f64) -> Self {
        self.value = v;
        self
    }

    pub fn min(mut self, v: f64) -> Self {
        self.min = v;
        self
    }

    pub fn max(mut self, v: f64) -> Self {
        self.max = v;
        self
    }

    pub fn step(mut self, v: f64) -> Self {
        self.step = v;
        self
    }

    pub fn precision(mut self, p: usize) -> Self {
        self.precision = p;
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }

    pub fn on_change(mut self, f: impl Fn(f64, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    fn format_value(&self) -> String {
        format!("{:.prec$}", self.value, prec = self.precision)
    }

    fn can_decrement(&self) -> bool {
        self.value - self.step >= self.min
    }

    fn can_increment(&self) -> bool {
        self.value + self.step <= self.max
    }
}

impl Element for NumberInput {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "NumberInput",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // [- button] [value text] [+ button]
        self.dec_id = cx.new_leaf(Style {
            size: Size {
                width: length(BTN_WIDTH),
                height: length(INPUT_HEIGHT),
            },
            ..Default::default()
        });
        self.value_id = cx.new_leaf(Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: length(INPUT_HEIGHT),
            },
            ..Default::default()
        });
        self.inc_id = cx.new_leaf(Style {
            size: Size {
                width: length(BTN_WIDTH),
                height: length(INPUT_HEIGHT),
            },
            ..Default::default()
        });

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: Size {
                    width: length(self.width),
                    height: length(INPUT_HEIGHT),
                },
                ..Default::default()
            },
            &[self.dec_id, self.value_id, self.inc_id],
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Overall background + border
        cx.draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
            shadow: None, shadows: vec![],
        });

        let alpha = if self.disabled { 0.5 } else { 1.0 };

        // Decrement button
        let dec_bounds = cx.bounds(self.dec_id);
        let dec_hovered =
            !self.disabled && self.can_decrement() && cx.interactions.is_hovered(dec_bounds);
        if dec_hovered {
            cx.draw_list.push(DrawCommand::Rect {
                bounds: dec_bounds,
                background: Fill::Solid(self.hover_bg),
                corner_radii: Corners {
                    top_left: self.corner_radius,
                    bottom_left: self.corner_radius,
                    top_right: 0.0,
                    bottom_right: 0.0,
                },
                border: None,
                shadow: None, shadows: vec![],
            });
        }
        let dec_color = if self.can_decrement() {
            self.fg.with_alpha(alpha)
        } else {
            self.muted_fg.with_alpha(0.3)
        };
        cx.draw_list.push(DrawCommand::Icon {
            name: IconName::Minus,
            weight: IconWeight::Bold,
            bounds: Rect::new(
                dec_bounds.origin.x + (BTN_WIDTH - ICON_SIZE) / 2.0,
                dec_bounds.origin.y + (INPUT_HEIGHT - ICON_SIZE) / 2.0,
                ICON_SIZE,
                ICON_SIZE,
            ),
            color: dec_color,
            size_px: ICON_SIZE,
        });

        // Value area
        let val_bounds = cx.bounds(self.value_id);
        let val_text = self.format_value();
        // Center the text
        let style = mozui_text::TextStyle {
            font_size: FONT_SIZE,
            ..Default::default()
        };
        let measured = mozui_text::measure_text(&val_text, &style, None, cx.font_system);
        let text_x = val_bounds.origin.x + (val_bounds.size.width - measured.width) / 2.0;
        cx.draw_list.push(DrawCommand::Text {
            text: val_text,
            bounds: Rect::new(text_x, val_bounds.origin.y, measured.width, val_bounds.size.height),
            font_size: FONT_SIZE,
            color: self.fg.with_alpha(alpha),
            weight: 500,
            italic: false,
        });

        // Divider lines
        cx.draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(
                dec_bounds.origin.x + dec_bounds.size.width,
                dec_bounds.origin.y + 6.0,
                1.0,
                INPUT_HEIGHT - 12.0,
            ),
            background: Fill::Solid(self.border_color),
            corner_radii: Corners::uniform(0.0),
            border: None,
            shadow: None, shadows: vec![],
        });
        cx.draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(
                val_bounds.origin.x + val_bounds.size.width,
                val_bounds.origin.y + 6.0,
                1.0,
                INPUT_HEIGHT - 12.0,
            ),
            background: Fill::Solid(self.border_color),
            corner_radii: Corners::uniform(0.0),
            border: None,
            shadow: None, shadows: vec![],
        });

        // Increment button
        let inc_bounds = cx.bounds(self.inc_id);
        let inc_hovered =
            !self.disabled && self.can_increment() && cx.interactions.is_hovered(inc_bounds);
        if inc_hovered {
            cx.draw_list.push(DrawCommand::Rect {
                bounds: inc_bounds,
                background: Fill::Solid(self.hover_bg),
                corner_radii: Corners {
                    top_left: 0.0,
                    bottom_left: 0.0,
                    top_right: self.corner_radius,
                    bottom_right: self.corner_radius,
                },
                border: None,
                shadow: None, shadows: vec![],
            });
        }
        let inc_color = if self.can_increment() {
            self.fg.with_alpha(alpha)
        } else {
            self.muted_fg.with_alpha(0.3)
        };
        cx.draw_list.push(DrawCommand::Icon {
            name: IconName::Plus,
            weight: IconWeight::Bold,
            bounds: Rect::new(
                inc_bounds.origin.x + (BTN_WIDTH - ICON_SIZE) / 2.0,
                inc_bounds.origin.y + (INPUT_HEIGHT - ICON_SIZE) / 2.0,
                ICON_SIZE,
                ICON_SIZE,
            ),
            color: inc_color,
            size_px: ICON_SIZE,
        });

        // Click handlers
        if !self.disabled {
            if let Some(ref on_change) = self.on_change {
                if self.can_decrement() {
                    let h = on_change.clone();
                    let new_val = (self.value - self.step).max(self.min);
                    cx.interactions.register_click(
                        dec_bounds,
                        Rc::new(move |cx: &mut dyn std::any::Any| { h(new_val, cx) }),
                    );
                }

                if self.can_increment() {
                    let h = on_change.clone();
                    let new_val = (self.value + self.step).min(self.max);
                    cx.interactions.register_click(
                        inc_bounds,
                        Rc::new(move |cx: &mut dyn std::any::Any| { h(new_val, cx) }),
                    );
                }
            }
        }

        cx.interactions.register_hover_region(dec_bounds);
        cx.interactions.register_hover_region(inc_bounds);
    }
}
