use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Rect};
use taffy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerDirection {
    Horizontal,
    Vertical,
}

/// Visual style of the divider line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerVariant {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

pub struct Divider {
    layout_id: LayoutId,
    left_line_id: LayoutId,
    text_id: LayoutId,
    right_line_id: LayoutId,
    direction: DividerDirection,
    color: Color,
    thickness: f32,
    label: Option<String>,
    variant: DividerVariant,
}

pub fn divider() -> Divider {
    Divider {
        layout_id: LayoutId::NONE,
        left_line_id: LayoutId::NONE,
        text_id: LayoutId::NONE,
        right_line_id: LayoutId::NONE,
        direction: DividerDirection::Horizontal,
        color: Color::new(1.0, 1.0, 1.0, 0.12),
        thickness: 1.0,
        label: None,
        variant: DividerVariant::Solid,
    }
}

impl Divider {
    pub fn vertical(mut self) -> Self {
        self.direction = DividerDirection::Vertical;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.direction = DividerDirection::Horizontal;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = Some(text.into());
        self
    }

    pub fn dashed(mut self) -> Self {
        self.variant = DividerVariant::Dashed;
        self
    }

    pub fn dotted(mut self) -> Self {
        self.variant = DividerVariant::Dotted;
        self
    }

    pub fn variant(mut self, variant: DividerVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Element for Divider {
    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        if self.label.is_some() {
            // Labeled divider: row with [line, text, line]
            let text_style = mozui_text::TextStyle {
                font_size: 12.0,
                color: self.color,
                ..Default::default()
            };
            let measured = mozui_text::measure_text(
                self.label.as_deref().unwrap(),
                &text_style,
                None,
                cx.font_system,
            );

            self.left_line_id = cx.new_leaf(Style {
                flex_grow: 1.0,
                size: Size {
                    width: auto(),
                    height: length(self.thickness),
                },
                align_self: Some(AlignSelf::Center),
                ..Default::default()
            });

            self.text_id = cx.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
                },
                margin: taffy::Rect {
                    left: length(8.0),
                    right: length(8.0),
                    top: zero(),
                    bottom: zero(),
                },
                ..Default::default()
            });

            self.right_line_id = cx.new_leaf(Style {
                flex_grow: 1.0,
                size: Size {
                    width: auto(),
                    height: length(self.thickness),
                },
                align_self: Some(AlignSelf::Center),
                ..Default::default()
            });

            self.layout_id = cx.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    size: Size {
                        width: percent(1.0),
                        height: auto(),
                    },
                    ..Default::default()
                },
                &[self.left_line_id, self.text_id, self.right_line_id],
            );
            self.layout_id
        } else {
            match self.direction {
                DividerDirection::Horizontal => {
                    self.layout_id = cx.new_leaf(Style {
                        size: Size {
                            width: percent(1.0),
                            height: length(self.thickness),
                        },
                        ..Default::default()
                    });
                }
                DividerDirection::Vertical => {
                    self.layout_id = cx.new_leaf(Style {
                        size: Size {
                            width: length(self.thickness),
                            height: percent(1.0),
                        },
                        ..Default::default()
                    });
                }
            }
            self.layout_id
        }
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        if self.label.is_some() {
            // Labeled divider has 3 children: left line, text, right line
            // Left line
            let left_bounds = cx.bounds(self.left_line_id);
            self.paint_line(&mut cx.draw_list, left_bounds);

            // Text
            let text_bounds = cx.bounds(self.text_id);
            cx.draw_list.push(DrawCommand::Text {
                text: self.label.clone().unwrap(),
                bounds: text_bounds,
                font_size: 12.0,
                color: self.color,
                weight: 400,
                italic: false,
            });

            // Right line
            let right_bounds = cx.bounds(self.right_line_id);
            self.paint_line(&mut cx.draw_list, right_bounds);
        } else {
            self.paint_line(&mut cx.draw_list, bounds);
        }
    }
}

impl Divider {
    /// Paint a line (solid, dashed, or dotted) within the given bounds.
    fn paint_line(&self, draw_list: &mut DrawList, bounds: Rect) {
        match self.variant {
            DividerVariant::Solid => {
                draw_list.push(DrawCommand::Rect {
                    bounds,
                    background: Fill::Solid(self.color),
                    corner_radii: Corners::ZERO,
                    border: None,
                    shadow: None,
                });
            }
            DividerVariant::Dashed => {
                // Dash: 6px on, 4px off
                let (dash_len, gap_len) = (6.0, 4.0);
                self.paint_segments(draw_list, bounds, dash_len, gap_len, Corners::ZERO);
            }
            DividerVariant::Dotted => {
                // Dot: thickness-sized circles with gaps
                let dot_size = self.thickness.max(2.0);
                let gap_len = dot_size * 1.5;
                let radius = dot_size / 2.0;
                self.paint_segments(
                    draw_list,
                    bounds,
                    dot_size,
                    gap_len,
                    Corners::uniform(radius),
                );
            }
        }
    }

    /// Paint repeated segments along the divider axis.
    fn paint_segments(
        &self,
        draw_list: &mut DrawList,
        bounds: Rect,
        seg_len: f32,
        gap_len: f32,
        corners: Corners,
    ) {
        let is_horizontal =
            self.direction == DividerDirection::Horizontal || (self.label.is_some()); // labeled dividers are always horizontal segments
        let total = if is_horizontal {
            bounds.size.width
        } else {
            bounds.size.height
        };
        let stride = seg_len + gap_len;
        let mut pos = 0.0;
        while pos < total {
            let len = (seg_len).min(total - pos);
            let seg_bounds = if is_horizontal {
                Rect::new(
                    bounds.origin.x + pos,
                    bounds.origin.y,
                    len,
                    bounds.size.height,
                )
            } else {
                Rect::new(
                    bounds.origin.x,
                    bounds.origin.y + pos,
                    bounds.size.width,
                    len,
                )
            };
            draw_list.push(DrawCommand::Rect {
                bounds: seg_bounds,
                background: Fill::Solid(self.color),
                corner_radii: corners,
                border: None,
                shadow: None,
            });
            pos += stride;
        }
    }
}
