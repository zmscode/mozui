use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill};
use mozui_text::FontSystem;
use taffy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerDirection {
    Horizontal,
    Vertical,
}

pub struct Divider {
    direction: DividerDirection,
    color: Color,
    thickness: f32,
    label: Option<String>,
}

pub fn divider() -> Divider {
    Divider {
        direction: DividerDirection::Horizontal,
        color: Color::new(1.0, 1.0, 1.0, 0.12),
        thickness: 1.0,
        label: None,
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
}

impl Element for Divider {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        if self.label.is_some() {
            // Labeled divider: row with [line, text, line]
            let text_style = mozui_text::TextStyle {
                font_size: 12.0,
                color: self.color,
                ..Default::default()
            };
            let measured =
                mozui_text::measure_text(self.label.as_deref().unwrap(), &text_style, None, font_system);

            let left_line = engine.new_leaf(Style {
                flex_grow: 1.0,
                size: Size {
                    width: auto(),
                    height: length(self.thickness),
                },
                align_self: Some(AlignSelf::Center),
                ..Default::default()
            });

            let text_node = engine.new_leaf(Style {
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

            let right_line = engine.new_leaf(Style {
                flex_grow: 1.0,
                size: Size {
                    width: auto(),
                    height: length(self.thickness),
                },
                align_self: Some(AlignSelf::Center),
                ..Default::default()
            });

            engine.new_with_children(
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
                &[left_line, text_node, right_line],
            )
        } else {
            match self.direction {
                DividerDirection::Horizontal => engine.new_leaf(Style {
                    size: Size {
                        width: percent(1.0),
                        height: length(self.thickness),
                    },
                    ..Default::default()
                }),
                DividerDirection::Vertical => engine.new_leaf(Style {
                    size: Size {
                        width: length(self.thickness),
                        height: percent(1.0),
                    },
                    ..Default::default()
                }),
            }
        }
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        _interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        if self.label.is_some() {
            // Labeled divider has 3 children: left line, text, right line
            // Left line
            let left = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Rect {
                bounds: mozui_style::Rect::new(left.x, left.y, left.width, left.height),
                background: Fill::Solid(self.color),
                corner_radii: Corners::ZERO,
                border: None,
            });

            // Text
            let text_layout = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Text {
                text: self.label.clone().unwrap(),
                bounds: mozui_style::Rect::new(
                    text_layout.x,
                    text_layout.y,
                    text_layout.width,
                    text_layout.height,
                ),
                font_size: 12.0,
                color: self.color,
                weight: 400,
                italic: false,
            });

            // Right line
            let right = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Rect {
                bounds: mozui_style::Rect::new(right.x, right.y, right.width, right.height),
                background: Fill::Solid(self.color),
                corner_radii: Corners::ZERO,
                border: None,
            });
        } else {
            let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);
            draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(self.color),
                corner_radii: Corners::ZERO,
                border: None,
            });
        }
    }
}
