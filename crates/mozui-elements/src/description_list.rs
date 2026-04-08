use crate::styled::{ComponentSize, Sizable};
use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct DescriptionItem {
    label: String,
    value: String,
}

pub fn description_item(label: impl Into<String>) -> DescriptionItem {
    DescriptionItem {
        label: label.into(),
        value: String::new(),
    }
}

impl DescriptionItem {
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptionLayout {
    Horizontal,
    Vertical,
}

pub struct DescriptionList {
    items: Vec<DescriptionItem>,
    layout_mode: DescriptionLayout,
    bordered: bool,
    label_width: f32,
    size: ComponentSize,
    label_color: Color,
    value_color: Color,
    border_color: Color,
}

pub fn description_list(theme: &Theme) -> DescriptionList {
    DescriptionList {
        items: Vec::new(),
        layout_mode: DescriptionLayout::Horizontal,
        bordered: false,
        label_width: 120.0,
        size: ComponentSize::Medium,
        label_color: theme.muted_foreground,
        value_color: theme.foreground,
        border_color: theme.border,
    }
}

impl DescriptionList {
    pub fn child(mut self, item: DescriptionItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn children(mut self, items: impl IntoIterator<Item = DescriptionItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn vertical(mut self) -> Self {
        self.layout_mode = DescriptionLayout::Vertical;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.layout_mode = DescriptionLayout::Horizontal;
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }

    fn text_size(&self) -> f32 {
        self.size.input_text_size()
    }

    fn row_padding(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 4.0,
            ComponentSize::Small => 6.0,
            ComponentSize::Medium => 8.0,
            ComponentSize::Large => 12.0,
            ComponentSize::Custom(_) => 8.0,
        }
    }
}

impl Sizable for DescriptionList {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Element for DescriptionList {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let font_size = self.text_size();
        let row_py = self.row_padding();
        let mut row_nodes = Vec::new();

        for item in &self.items {
            // Label
            let label_style = mozui_text::TextStyle {
                font_size,
                color: self.label_color,
                ..Default::default()
            };
            let label_measured =
                mozui_text::measure_text(&item.label, &label_style, None, font_system);

            // Value
            let value_style = mozui_text::TextStyle {
                font_size,
                color: self.value_color,
                ..Default::default()
            };
            let value_measured =
                mozui_text::measure_text(&item.value, &value_style, None, font_system);

            match self.layout_mode {
                DescriptionLayout::Horizontal => {
                    let label_node = engine.new_leaf(Style {
                        size: Size {
                            width: length(self.label_width),
                            height: length(label_measured.height),
                        },
                        ..Default::default()
                    });
                    let value_node = engine.new_leaf(Style {
                        size: Size {
                            width: length(value_measured.width),
                            height: length(value_measured.height),
                        },
                        flex_grow: 1.0,
                        ..Default::default()
                    });
                    let row = engine.new_with_children(
                        Style {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            align_items: Some(AlignItems::FlexStart),
                            padding: taffy::Rect {
                                left: zero(),
                                right: zero(),
                                top: length(row_py),
                                bottom: length(row_py),
                            },
                            gap: Size {
                                width: length(16.0),
                                height: zero(),
                            },
                            ..Default::default()
                        },
                        &[label_node, value_node],
                    );
                    row_nodes.push(row);
                }
                DescriptionLayout::Vertical => {
                    let label_node = engine.new_leaf(Style {
                        size: Size {
                            width: length(label_measured.width),
                            height: length(label_measured.height),
                        },
                        ..Default::default()
                    });
                    let value_node = engine.new_leaf(Style {
                        size: Size {
                            width: length(value_measured.width),
                            height: length(value_measured.height),
                        },
                        ..Default::default()
                    });
                    let row = engine.new_with_children(
                        Style {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            padding: taffy::Rect {
                                left: zero(),
                                right: zero(),
                                top: length(row_py),
                                bottom: length(row_py),
                            },
                            gap: Size {
                                width: zero(),
                                height: length(4.0),
                            },
                            ..Default::default()
                        },
                        &[label_node, value_node],
                    );
                    row_nodes.push(row);
                }
            }

            // Divider leaf (1px height) — used if bordered
            if self.bordered {
                row_nodes.push(engine.new_leaf(Style {
                    size: Size {
                        width: percent(1.0),
                        height: length(1.0),
                    },
                    ..Default::default()
                }));
            }
        }

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &row_nodes,
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
        let _outer = layouts[*index];
        *index += 1;

        let font_size = self.text_size();

        for (i, item) in self.items.iter().enumerate() {
            // Row container
            let _row_layout = layouts[*index];
            *index += 1;

            // Label
            let label_layout = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Text {
                text: item.label.clone(),
                bounds: mozui_style::Rect::new(
                    label_layout.x,
                    label_layout.y,
                    label_layout.width,
                    label_layout.height,
                ),
                font_size,
                color: self.label_color,
                weight: 500,
                italic: false,
            });

            // Value
            let value_layout = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Text {
                text: item.value.clone(),
                bounds: mozui_style::Rect::new(
                    value_layout.x,
                    value_layout.y,
                    value_layout.width,
                    value_layout.height,
                ),
                font_size,
                color: self.value_color,
                weight: 400,
                italic: false,
            });

            // Bordered divider
            if self.bordered && i < self.items.len() - 1 {
                let div_layout = layouts[*index];
                *index += 1;
                draw_list.push(DrawCommand::Rect {
                    bounds: mozui_style::Rect::new(
                        div_layout.x,
                        div_layout.y,
                        div_layout.width,
                        div_layout.height,
                    ),
                    background: Fill::Solid(self.border_color),
                    corner_radii: Corners::uniform(0.0),
                    border: None,
                });
            } else if self.bordered {
                // Last item still has a divider node — skip it
                *index += 1;
            }
        }
    }
}
