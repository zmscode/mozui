use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Point, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

use std::f32::consts::FRAC_PI_2;

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
        Hsv { h: self.h, s: 1.0, v: 1.0, a: 1.0 }.to_color()
    }
}

pub struct ColorPicker {
    color: Color,
    on_change: Option<Box<dyn Fn(Color, &mut dyn std::any::Any)>>,
    show_alpha: bool,
    show_preview: bool,
    show_hex: bool,
    bg: Color,
    fg: Color,
    border_color: Color,
    corner_radius: f32,
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
    }
}

impl ColorPicker {
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(Color, &mut dyn std::any::Any) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
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
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut children = Vec::new();

        // SV area
        children.push(engine.new_leaf(Style {
            size: Size { width: length(AREA_SIZE), height: length(AREA_SIZE) },
            ..Default::default()
        }));

        // Hue slider
        children.push(engine.new_leaf(Style {
            size: Size { width: length(AREA_SIZE), height: length(SLIDER_HEIGHT) },
            ..Default::default()
        }));

        // Alpha slider
        if self.show_alpha {
            children.push(engine.new_leaf(Style {
                size: Size { width: length(AREA_SIZE), height: length(SLIDER_HEIGHT) },
                ..Default::default()
            }));
        }

        // Preview row
        if self.show_preview {
            let mut preview_children = Vec::new();
            preview_children.push(engine.new_leaf(Style {
                size: Size { width: length(PREVIEW_SIZE), height: length(PREVIEW_SIZE) },
                ..Default::default()
            }));
            if self.show_hex {
                let hex_text = format_hex(&self.color);
                let text_style = mozui_text::TextStyle {
                    font_size: 13.0,
                    color: self.fg,
                    ..Default::default()
                };
                let m = mozui_text::measure_text(&hex_text, &text_style, None, font_system);
                preview_children.push(engine.new_leaf(Style {
                    size: Size { width: length(m.width), height: length(m.height) },
                    ..Default::default()
                }));
            }
            children.push(engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    gap: Size { width: length(GAP), height: zero() },
                    ..Default::default()
                },
                &preview_children,
            ));
        }

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                gap: Size { width: zero(), height: length(GAP) },
                padding: taffy::Rect {
                    left: length(PADDING),
                    right: length(PADDING),
                    top: length(PADDING),
                    bottom: length(PADDING),
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
        let hsv = Hsv::from_color(self.color);

        // Outer container
        let container = layouts[*index];
        *index += 1;
        let container_bounds = mozui_style::Rect::new(
            container.x, container.y, container.width, container.height,
        );
        draw_list.push(DrawCommand::Rect {
            bounds: container_bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(mozui_renderer::Border { width: 1.0, color: self.border_color }),
            shadow: None,
        });

        // ── SV Area ──────────────────────────────────────────────
        let sv_layout = layouts[*index];
        *index += 1;
        let sv_bounds = mozui_style::Rect::new(
            sv_layout.x, sv_layout.y, sv_layout.width, sv_layout.height,
        );
        let sv_radius = 6.0;

        // Base: solid hue color
        draw_list.push(DrawCommand::Rect {
            bounds: sv_bounds,
            background: Fill::Solid(hsv.hue_color()),
            corner_radii: Corners::uniform(sv_radius),
            border: None,
            shadow: None,
        });
        // Overlay: white → transparent (left to right = saturation)
        // angle=0 means left-to-right in our shader
        draw_list.push(DrawCommand::Rect {
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
        // angle=PI/2 means top-to-bottom
        draw_list.push(DrawCommand::Rect {
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
        draw_list.push(DrawCommand::Rect {
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
        draw_list.push(DrawCommand::Rect {
            bounds: mozui_style::Rect::new(
                thumb_x - THUMB_RADIUS - 1.0, thumb_y - THUMB_RADIUS - 1.0,
                (THUMB_RADIUS + 1.0) * 2.0, (THUMB_RADIUS + 1.0) * 2.0,
            ),
            background: Fill::Solid(Color::rgba(0, 0, 0, 0.3)),
            corner_radii: Corners::uniform(THUMB_RADIUS + 1.0),
            border: None,
            shadow: None,
        });
        draw_list.push(DrawCommand::Rect {
            bounds: mozui_style::Rect::new(
                thumb_x - THUMB_RADIUS, thumb_y - THUMB_RADIUS,
                THUMB_RADIUS * 2.0, THUMB_RADIUS * 2.0,
            ),
            background: Fill::Solid(self.color.with_alpha(1.0)),
            corner_radii: Corners::uniform(THUMB_RADIUS),
            border: Some(mozui_renderer::Border { width: 2.0, color: Color::WHITE }),
            shadow: None,
        });

        // SV drag handler — use screen-space bounds so scroll offset is accounted for
        if let Some(ref handler) = self.on_change {
            let handler_ptr = handler.as_ref() as *const dyn Fn(Color, &mut dyn std::any::Any);
            let screen_sv = interactions.adjust_bounds(sv_bounds);
            let sv_origin = screen_sv.origin;
            let sv_size = screen_sv.size;
            let hue = hsv.h;
            let alpha = hsv.a;
            interactions.register_drag_handler(
                sv_bounds,
                Box::new(move |pos: Point, cx| {
                    let s = ((pos.x - sv_origin.x) / sv_size.width).clamp(0.0, 1.0);
                    let v = 1.0 - ((pos.y - sv_origin.y) / sv_size.height).clamp(0.0, 1.0);
                    let new_color = Hsv { h: hue, s, v, a: alpha }.to_color();
                    unsafe { (*handler_ptr)(new_color, cx) };
                }),
            );
        }

        // ── Hue slider ──────────────────────────────────────────
        let hue_layout = layouts[*index];
        *index += 1;
        let hue_bounds = mozui_style::Rect::new(
            hue_layout.x, hue_layout.y, hue_layout.width, hue_layout.height,
        );
        let slider_radius = SLIDER_HEIGHT / 2.0;

        // Draw 6 segments for the full hue spectrum
        let seg_w = hue_bounds.size.width / HUE_SEGMENTS as f32;
        for i in 0..HUE_SEGMENTS {
            let (r0, g0, b0) = HUE_COLORS[i];
            let (r1, g1, b1) = HUE_COLORS[i + 1];
            let seg_x = hue_bounds.origin.x + i as f32 * seg_w;
            let seg_bounds = mozui_style::Rect::new(
                seg_x, hue_bounds.origin.y, seg_w + 1.0, hue_bounds.size.height,
            );
            // Corner radii: round only the leftmost and rightmost segments
            let corners = Corners {
                top_left: if i == 0 { slider_radius } else { 0.0 },
                bottom_left: if i == 0 { slider_radius } else { 0.0 },
                top_right: if i == HUE_SEGMENTS - 1 { slider_radius } else { 0.0 },
                bottom_right: if i == HUE_SEGMENTS - 1 { slider_radius } else { 0.0 },
            };
            draw_list.push(DrawCommand::Rect {
                bounds: seg_bounds,
                background: Fill::LinearGradient {
                    angle: 0.0, // left to right
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
        draw_list.push(DrawCommand::Rect {
            bounds: mozui_style::Rect::new(
                hue_thumb_x - thumb_r, hue_thumb_cy - thumb_r,
                thumb_r * 2.0, thumb_r * 2.0,
            ),
            background: Fill::Solid(hsv.hue_color()),
            corner_radii: Corners::uniform(thumb_r),
            border: Some(mozui_renderer::Border { width: 2.0, color: Color::WHITE }),
            shadow: None,
        });

        // Hue drag handler
        if let Some(ref handler) = self.on_change {
            let handler_ptr = handler.as_ref() as *const dyn Fn(Color, &mut dyn std::any::Any);
            let screen_hue = interactions.adjust_bounds(hue_bounds);
            let hue_origin_x = screen_hue.origin.x;
            let hue_width = screen_hue.size.width;
            let sat = hsv.s;
            let val = hsv.v;
            let alpha = hsv.a;
            interactions.register_drag_handler(
                hue_bounds,
                Box::new(move |pos: Point, cx| {
                    let h = ((pos.x - hue_origin_x) / hue_width).clamp(0.0, 1.0) * 360.0;
                    let new_color = Hsv { h, s: sat, v: val, a: alpha }.to_color();
                    unsafe { (*handler_ptr)(new_color, cx) };
                }),
            );
        }

        // ── Alpha slider ─────────────────────────────────────────
        if self.show_alpha {
            let alpha_layout = layouts[*index];
            *index += 1;
            let alpha_bounds = mozui_style::Rect::new(
                alpha_layout.x, alpha_layout.y, alpha_layout.width, alpha_layout.height,
            );

            // Checkerboard simulation: alternating light/dark gray segments
            let checker_count = 12;
            let checker_w = alpha_bounds.size.width / checker_count as f32;
            for i in 0..checker_count {
                let c = if i % 2 == 0 {
                    Color::hex("#cccccc")
                } else {
                    Color::hex("#999999")
                };
                let cx = alpha_bounds.origin.x + i as f32 * checker_w;
                let corners = Corners {
                    top_left: if i == 0 { slider_radius } else { 0.0 },
                    bottom_left: if i == 0 { slider_radius } else { 0.0 },
                    top_right: if i == checker_count - 1 { slider_radius } else { 0.0 },
                    bottom_right: if i == checker_count - 1 { slider_radius } else { 0.0 },
                };
                draw_list.push(DrawCommand::Rect {
                    bounds: mozui_style::Rect::new(cx, alpha_bounds.origin.y, checker_w, alpha_bounds.size.height),
                    background: Fill::Solid(c),
                    corner_radii: corners,
                    border: None,
                    shadow: None,
                });
            }

            // Alpha gradient overlay
            let opaque = self.color.with_alpha(1.0);
            draw_list.push(DrawCommand::Rect {
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
            draw_list.push(DrawCommand::Rect {
                bounds: mozui_style::Rect::new(
                    alpha_thumb_x - thumb_r, alpha_thumb_cy - thumb_r,
                    thumb_r * 2.0, thumb_r * 2.0,
                ),
                background: Fill::Solid(self.color),
                corner_radii: Corners::uniform(thumb_r),
                border: Some(mozui_renderer::Border { width: 2.0, color: Color::WHITE }),
                shadow: None,
            });

            // Alpha drag handler
            if let Some(ref handler) = self.on_change {
                let handler_ptr = handler.as_ref() as *const dyn Fn(Color, &mut dyn std::any::Any);
                let screen_alpha = interactions.adjust_bounds(alpha_bounds);
                let alpha_origin_x = screen_alpha.origin.x;
                let alpha_width = screen_alpha.size.width;
                let h = hsv.h;
                let s = hsv.s;
                let v = hsv.v;
                interactions.register_drag_handler(
                    alpha_bounds,
                    Box::new(move |pos: Point, cx| {
                        let a = ((pos.x - alpha_origin_x) / alpha_width).clamp(0.0, 1.0);
                        let new_color = Hsv { h, s, v, a }.to_color();
                        unsafe { (*handler_ptr)(new_color, cx) };
                    }),
                );
            }
        }

        // ── Preview row ──────────────────────────────────────────
        if self.show_preview {
            let _preview_row = layouts[*index];
            *index += 1;

            let swatch_layout = layouts[*index];
            *index += 1;
            let swatch_bounds = mozui_style::Rect::new(
                swatch_layout.x, swatch_layout.y, swatch_layout.width, swatch_layout.height,
            );
            draw_list.push(DrawCommand::Rect {
                bounds: swatch_bounds,
                background: Fill::Solid(self.color),
                corner_radii: Corners::uniform(6.0),
                border: Some(mozui_renderer::Border { width: 1.0, color: self.border_color }),
                shadow: None,
            });

            if self.show_hex {
                let hex_layout = layouts[*index];
                *index += 1;
                let hex_bounds = mozui_style::Rect::new(
                    hex_layout.x, hex_layout.y, hex_layout.width, hex_layout.height,
                );
                draw_list.push(DrawCommand::Text {
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
