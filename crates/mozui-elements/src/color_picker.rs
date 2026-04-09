use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Point, Rect, Theme};
use taffy::prelude::*;

use std::f32::consts::FRAC_PI_2;
use std::rc::Rc;

/// Width of the saturation/brightness gradient area.
const AREA_SIZE: f32 = 200.0;
/// Height of the hue/alpha sliders.
const SLIDER_HEIGHT: f32 = 16.0;
/// Gap between elements.
const GAP: f32 = 12.0;
/// Size of the color preview swatch.
const PREVIEW_SIZE: f32 = 32.0;
/// Padding around the picker.
const PADDING: f32 = 16.0;
/// Radius of the draggable thumb in the SV area.
const THUMB_RADIUS: f32 = 7.0;
/// Number of segments for the hue bar.
const HUE_SEGMENTS: usize = 6;

/// The 6 hue keypoints: Red, Yellow, Green, Cyan, Blue, Magenta, Red.
const HUE_COLORS: [(f32, f32, f32); 7] = [
    (1.0, 0.0, 0.0), // 0°   Red
    (1.0, 1.0, 0.0), // 60°  Yellow
    (0.0, 1.0, 0.0), // 120° Green
    (0.0, 1.0, 1.0), // 180° Cyan
    (0.0, 0.0, 1.0), // 240° Blue
    (1.0, 0.0, 1.0), // 300° Magenta
    (1.0, 0.0, 0.0), // 360° Red (wrap)
];

/// HSV representation for internal color manipulation.
#[derive(Debug, Clone, Copy)]
struct Hsv {
    h: f32, // 0..360
    s: f32, // 0..1
    v: f32, // 0..1
    a: f32, // 0..1
}

impl Hsv {
    fn from_color(c: Color) -> Self {
        let r = c.r;
        let g = c.g;
        let b = c.b;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let h = if delta < 0.0001 {
            0.0
        } else if (max - r).abs() < 0.0001 {
            60.0 * (((g - b) / delta) % 6.0)
        } else if (max - g).abs() < 0.0001 {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };
        let h = if h < 0.0 { h + 360.0 } else { h };

        let s = if max < 0.0001 { 0.0 } else { delta / max };
        let v = max;

        Hsv { h, s, v, a: c.a }
    }

    fn to_color(&self) -> Color {
        let c = self.v * self.s;
        let x = c * (1.0 - ((self.h / 60.0) % 2.0 - 1.0).abs());
        let m = self.v - c;

        let (r, g, b) = if self.h < 60.0 {
            (c, x, 0.0)
        } else if self.h < 120.0 {
            (x, c, 0.0)
        } else if self.h < 180.0 {
            (0.0, c, x)
        } else if self.h < 240.0 {
            (0.0, x, c)
        } else if self.h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Color::new(r + m, g + m, b + m, self.a)
    }

    fn hue_color(&self) -> Color {
        Hsv {
            h: self.h,
            s: 1.0,
            v: 1.0,
            a: 1.0,
        }
        .to_color()
    }
}

pub struct ColorPicker {
    color: Color,
    on_change: Option<Rc<dyn Fn(Color, &mut dyn std::any::Any)>>,
    show_alpha: bool,
    show_preview: bool,
    show_hex: bool,
    bg: Color,
    fg: Color,
    border_color: Color,
    corner_radius: f32,
    // Layout IDs
    layout_id: LayoutId,
    sv_id: LayoutId,
    hue_id: LayoutId,
    alpha_id: LayoutId,
    preview_row_id: LayoutId,
    swatch_id: LayoutId,
    hex_id: LayoutId,
}

pub fn color_picker(theme: &Theme) -> ColorPicker {
    ColorPicker {
        color: theme.primary,
        on_change: None,
        show_alpha: true,
        show_preview: true,
        show_hex: true,
        bg: theme.popover,
        fg: theme.popover_foreground,
        border_color: theme.border,
        corner_radius: theme.radius_lg,
        layout_id: LayoutId::NONE,
        sv_id: LayoutId::NONE,
        hue_id: LayoutId::NONE,
        alpha_id: LayoutId::NONE,
        preview_row_id: LayoutId::NONE,
        swatch_id: LayoutId::NONE,
        hex_id: LayoutId::NONE,
    }
}

impl ColorPicker {
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(Color, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    pub fn show_alpha(mut self, show: bool) -> Self {
        self.show_alpha = show;
        self
    }

    pub fn show_hex(mut self, show: bool) -> Self {
        self.show_hex = show;
        self
    }
}

impl Element for ColorPicker {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "ColorPicker",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let mut children = Vec::new();

        // SV area
        self.sv_id = cx.new_leaf(Style {
            size: Size {
                width: length(AREA_SIZE),
                height: length(AREA_SIZE),
            },
            ..Default::default()
        });
        children.push(self.sv_id);

        // Hue slider
        self.hue_id = cx.new_leaf(Style {
            size: Size {
                width: length(AREA_SIZE),
                height: length(SLIDER_HEIGHT),
            },
            ..Default::default()
        });
        children.push(self.hue_id);

        // Alpha slider
        if self.show_alpha {
            self.alpha_id = cx.new_leaf(Style {
                size: Size {
                    width: length(AREA_SIZE),
                    height: length(SLIDER_HEIGHT),
                },
                ..Default::default()
            });
            children.push(self.alpha_id);
        }

        // Preview row
        if self.show_preview {
            let mut preview_children = Vec::new();
            self.swatch_id = cx.new_leaf(Style {
                size: Size {
                    width: length(PREVIEW_SIZE),
                    height: length(PREVIEW_SIZE),
                },
                ..Default::default()
            });
            preview_children.push(self.swatch_id);

            if self.show_hex {
                let hex_text = format_hex(&self.color);
                let text_style = mozui_text::TextStyle {
                    font_size: 13.0,
                    color: self.fg,
                    ..Default::default()
                };
                let m = mozui_text::measure_text(&hex_text, &text_style, None, cx.font_system);
                self.hex_id = cx.new_leaf(Style {
                    size: Size {
                        width: length(m.width),
                        height: length(m.height),
                    },
                    ..Default::default()
                });
                preview_children.push(self.hex_id);
            }

            self.preview_row_id = cx.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    gap: Size {
                        width: length(GAP),
                        height: zero(),
                    },
                    ..Default::default()
                },
                &preview_children,
            );
            children.push(self.preview_row_id);
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                gap: Size {
                    width: zero(),
                    height: length(GAP),
                },
                padding: taffy::Rect {
                    left: length(PADDING),
                    right: length(PADDING),
                    top: length(PADDING),
                    bottom: length(PADDING),
                },
                ..Default::default()
            },
            &children,
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let hsv = Hsv::from_color(self.color);

        // Outer container
        cx.draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(mozui_renderer::Border {
                width: 1.0,
                color: self.border_color,
            }),
            shadow: None,
        });

        // ── SV Area ──────────────────────────────────────────────
        let sv_bounds = cx.bounds(self.sv_id);
        let sv_radius = 6.0;

        // Base: solid hue color
        cx.draw_list.push(DrawCommand::Rect {
            bounds: sv_bounds,
            background: Fill::Solid(hsv.hue_color()),
            corner_radii: Corners::uniform(sv_radius),
            border: None,
            shadow: None,
        });
        // Overlay: white → transparent (left to right = saturation)
        cx.draw_list.push(DrawCommand::Rect {
            bounds: sv_bounds,
            background: Fill::LinearGradient {
                angle: 0.0,
                stops: vec![(0.0, Color::WHITE), (1.0, Color::TRANSPARENT)],
            },
            corner_radii: Corners::uniform(sv_radius),
            border: None,
            shadow: None,
        });
        // Overlay: transparent → black (top to bottom = brightness)
        cx.draw_list.push(DrawCommand::Rect {
            bounds: sv_bounds,
            background: Fill::LinearGradient {
                angle: FRAC_PI_2,
                stops: vec![(0.0, Color::TRANSPARENT), (1.0, Color::BLACK)],
            },
            corner_radii: Corners::uniform(sv_radius),
            border: None,
            shadow: None,
        });
        // Border on the SV area
        cx.draw_list.push(DrawCommand::Rect {
            bounds: sv_bounds,
            background: Fill::Solid(Color::TRANSPARENT),
            corner_radii: Corners::uniform(sv_radius),
            border: Some(mozui_renderer::Border {
                width: 1.0,
                color: self.border_color.with_alpha(0.3),
            }),
            shadow: None,
        });

        // SV thumb
        let thumb_x = sv_bounds.origin.x + hsv.s * sv_bounds.size.width;
        let thumb_y = sv_bounds.origin.y + (1.0 - hsv.v) * sv_bounds.size.height;
        // Outer ring (shadow)
        cx.draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(
                thumb_x - THUMB_RADIUS - 1.0,
                thumb_y - THUMB_RADIUS - 1.0,
                (THUMB_RADIUS + 1.0) * 2.0,
                (THUMB_RADIUS + 1.0) * 2.0,
            ),
            background: Fill::Solid(Color::rgba(0, 0, 0, 0.3)),
            corner_radii: Corners::uniform(THUMB_RADIUS + 1.0),
            border: None,
            shadow: None,
        });
        cx.draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(
                thumb_x - THUMB_RADIUS,
                thumb_y - THUMB_RADIUS,
                THUMB_RADIUS * 2.0,
                THUMB_RADIUS * 2.0,
            ),
            background: Fill::Solid(self.color.with_alpha(1.0)),
            corner_radii: Corners::uniform(THUMB_RADIUS),
            border: Some(mozui_renderer::Border {
                width: 2.0,
                color: Color::WHITE,
            }),
            shadow: None,
        });

        // SV drag handler
        if let Some(ref handler) = self.on_change {
            let h = handler.clone();
            let screen_sv = cx.interactions.adjust_bounds(sv_bounds);
            let sv_origin = screen_sv.origin;
            let sv_size = screen_sv.size;
            let hue = hsv.h;
            let alpha = hsv.a;
            cx.interactions.register_drag_handler(
                sv_bounds,
                Rc::new(move |pos: Point, cx: &mut dyn std::any::Any| {
                    let s = ((pos.x - sv_origin.x) / sv_size.width).clamp(0.0, 1.0);
                    let v = 1.0 - ((pos.y - sv_origin.y) / sv_size.height).clamp(0.0, 1.0);
                    let new_color = Hsv {
                        h: hue,
                        s,
                        v,
                        a: alpha,
                    }
                    .to_color();
                    h(new_color, cx);
                }),
            );
        }

        // ── Hue slider ──────────────────────────────────────────
        let hue_bounds = cx.bounds(self.hue_id);
        let slider_radius = SLIDER_HEIGHT / 2.0;

        // Draw 6 segments for the full hue spectrum
        let seg_w = hue_bounds.size.width / HUE_SEGMENTS as f32;
        for i in 0..HUE_SEGMENTS {
            let (r0, g0, b0) = HUE_COLORS[i];
            let (r1, g1, b1) = HUE_COLORS[i + 1];
            let seg_x = hue_bounds.origin.x + i as f32 * seg_w;
            let seg_bounds = Rect::new(
                seg_x,
                hue_bounds.origin.y,
                seg_w + 1.0,
                hue_bounds.size.height,
            );
            let corners = Corners {
                top_left: if i == 0 { slider_radius } else { 0.0 },
                bottom_left: if i == 0 { slider_radius } else { 0.0 },
                top_right: if i == HUE_SEGMENTS - 1 {
                    slider_radius
                } else {
                    0.0
                },
                bottom_right: if i == HUE_SEGMENTS - 1 {
                    slider_radius
                } else {
                    0.0
                },
            };
            cx.draw_list.push(DrawCommand::Rect {
                bounds: seg_bounds,
                background: Fill::LinearGradient {
                    angle: 0.0,
                    stops: vec![
                        (0.0, Color::new(r0, g0, b0, 1.0)),
                        (1.0, Color::new(r1, g1, b1, 1.0)),
                    ],
                },
                corner_radii: corners,
                border: None,
                shadow: None,
            });
        }

        // Hue thumb
        let hue_frac = hsv.h / 360.0;
        let hue_thumb_x = hue_bounds.origin.x + hue_frac * hue_bounds.size.width;
        let hue_thumb_cy = hue_bounds.origin.y + hue_bounds.size.height / 2.0;
        let thumb_r = slider_radius;
        cx.draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(
                hue_thumb_x - thumb_r,
                hue_thumb_cy - thumb_r,
                thumb_r * 2.0,
                thumb_r * 2.0,
            ),
            background: Fill::Solid(hsv.hue_color()),
            corner_radii: Corners::uniform(thumb_r),
            border: Some(mozui_renderer::Border {
                width: 2.0,
                color: Color::WHITE,
            }),
            shadow: None,
        });

        // Hue drag handler
        if let Some(ref handler) = self.on_change {
            let h = handler.clone();
            let screen_hue = cx.interactions.adjust_bounds(hue_bounds);
            let hue_origin_x = screen_hue.origin.x;
            let hue_width = screen_hue.size.width;
            let sat = hsv.s;
            let val = hsv.v;
            let alpha = hsv.a;
            cx.interactions.register_drag_handler(
                hue_bounds,
                Rc::new(move |pos: Point, cx: &mut dyn std::any::Any| {
                    let hue = ((pos.x - hue_origin_x) / hue_width).clamp(0.0, 1.0) * 360.0;
                    let new_color = Hsv {
                        h: hue,
                        s: sat,
                        v: val,
                        a: alpha,
                    }
                    .to_color();
                    h(new_color, cx);
                }),
            );
        }

        // ── Alpha slider ─────────────────────────────────────────
        if self.show_alpha {
            let alpha_bounds = cx.bounds(self.alpha_id);

            // Checkerboard simulation: alternating light/dark gray segments
            let checker_count = 12;
            let checker_w = alpha_bounds.size.width / checker_count as f32;
            for i in 0..checker_count {
                let c = if i % 2 == 0 {
                    Color::hex("#cccccc")
                } else {
                    Color::hex("#999999")
                };
                let checker_x = alpha_bounds.origin.x + i as f32 * checker_w;
                let corners = Corners {
                    top_left: if i == 0 { slider_radius } else { 0.0 },
                    bottom_left: if i == 0 { slider_radius } else { 0.0 },
                    top_right: if i == checker_count - 1 {
                        slider_radius
                    } else {
                        0.0
                    },
                    bottom_right: if i == checker_count - 1 {
                        slider_radius
                    } else {
                        0.0
                    },
                };
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: Rect::new(
                        checker_x,
                        alpha_bounds.origin.y,
                        checker_w,
                        alpha_bounds.size.height,
                    ),
                    background: Fill::Solid(c),
                    corner_radii: corners,
                    border: None,
                    shadow: None,
                });
            }

            // Alpha gradient overlay
            let opaque = self.color.with_alpha(1.0);
            cx.draw_list.push(DrawCommand::Rect {
                bounds: alpha_bounds,
                background: Fill::LinearGradient {
                    angle: 0.0,
                    stops: vec![(0.0, Color::TRANSPARENT), (1.0, opaque)],
                },
                corner_radii: Corners::uniform(slider_radius),
                border: None,
                shadow: None,
            });

            // Alpha thumb
            let alpha_thumb_x = alpha_bounds.origin.x + hsv.a * alpha_bounds.size.width;
            let alpha_thumb_cy = alpha_bounds.origin.y + alpha_bounds.size.height / 2.0;
            cx.draw_list.push(DrawCommand::Rect {
                bounds: Rect::new(
                    alpha_thumb_x - thumb_r,
                    alpha_thumb_cy - thumb_r,
                    thumb_r * 2.0,
                    thumb_r * 2.0,
                ),
                background: Fill::Solid(self.color),
                corner_radii: Corners::uniform(thumb_r),
                border: Some(mozui_renderer::Border {
                    width: 2.0,
                    color: Color::WHITE,
                }),
                shadow: None,
            });

            // Alpha drag handler
            if let Some(ref handler) = self.on_change {
                let hh = handler.clone();
                let screen_alpha = cx.interactions.adjust_bounds(alpha_bounds);
                let alpha_origin_x = screen_alpha.origin.x;
                let alpha_width = screen_alpha.size.width;
                let h = hsv.h;
                let s = hsv.s;
                let v = hsv.v;
                cx.interactions.register_drag_handler(
                    alpha_bounds,
                    Rc::new(move |pos: Point, cx: &mut dyn std::any::Any| {
                        let a = ((pos.x - alpha_origin_x) / alpha_width).clamp(0.0, 1.0);
                        let new_color = Hsv { h, s, v, a }.to_color();
                        hh(new_color, cx);
                    }),
                );
            }
        }

        // ── Preview row ──────────────────────────────────────────
        if self.show_preview {
            let swatch_bounds = cx.bounds(self.swatch_id);
            cx.draw_list.push(DrawCommand::Rect {
                bounds: swatch_bounds,
                background: Fill::Solid(self.color),
                corner_radii: Corners::uniform(6.0),
                border: Some(mozui_renderer::Border {
                    width: 1.0,
                    color: self.border_color,
                }),
                shadow: None,
            });

            if self.show_hex {
                let hex_bounds = cx.bounds(self.hex_id);
                cx.draw_list.push(DrawCommand::Text {
                    text: format_hex(&self.color),
                    bounds: hex_bounds,
                    font_size: 13.0,
                    color: self.fg,
                    weight: 400,
                    italic: false,
                });
            }
        }
    }
}

fn format_hex(c: &Color) -> String {
    let r = (c.r * 255.0) as u8;
    let g = (c.g * 255.0) as u8;
    let b = (c.b * 255.0) as u8;
    if c.a < 0.999 {
        let a = (c.a * 255.0) as u8;
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    } else {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }
}
