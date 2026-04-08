use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, InteractionMap};
use mozui_icons::IconName;
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

/// A single step in a Stepper.
pub struct StepperItem {
    label: String,
    description: Option<String>,
    icon: Option<IconName>,
}

impl StepperItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            icon: None,
        }
    }

    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

pub struct Stepper {
    items: Vec<StepperItem>,
    current: usize,
    disabled: bool,
    size: ComponentSize,
    active_color: Color,
    inactive_color: Color,
    completed_color: Color,
    label_color: Color,
    muted_color: Color,
    line_color: Color,
    on_click: Option<Box<dyn Fn(usize, &mut dyn std::any::Any)>>,
}

pub fn stepper(theme: &Theme) -> Stepper {
    Stepper {
        items: Vec::new(),
        current: 0,
        disabled: false,
        size: ComponentSize::Medium,
        active_color: theme.primary,
        inactive_color: theme.muted,
        completed_color: theme.success,
        label_color: theme.foreground,
        muted_color: theme.muted_foreground,
        line_color: theme.border,
        on_click: None,
    }
}

impl Stepper {
    pub fn item(mut self, item: StepperItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = StepperItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn current(mut self, index: usize) -> Self {
        self.current = index;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(usize, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn circle_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 20.0,
            ComponentSize::Small => 24.0,
            ComponentSize::Medium => 28.0,
            ComponentSize::Large => 36.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }

    fn label_size(&self) -> f32 {
        self.size.input_text_size()
    }

    fn num_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 10.0,
            ComponentSize::Small => 11.0,
            ComponentSize::Medium => 12.0,
            ComponentSize::Large => 14.0,
            ComponentSize::Custom(_) => 12.0,
        }
    }
}

impl Sizable for Stepper {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Stepper {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Element for Stepper {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        // Layout: single flex row with alternating [step-column, connector-line] pairs.
        // Each step-column is: circle (fixed) + label text below it.
        let circle_sz = self.circle_size();
        let mut row_children = Vec::new();

        for (i, item) in self.items.iter().enumerate() {
            // Step column: circle + label stacked vertically, centered
            let circle_node = engine.new_leaf(Style {
                size: Size {
                    width: length(circle_sz),
                    height: length(circle_sz),
                },
                flex_shrink: 0.0,
                ..Default::default()
            });

            let text_style = mozui_text::TextStyle {
                font_size: self.label_size(),
                color: self.label_color,
                ..Default::default()
            };
            let measured = mozui_text::measure_text(&item.label, &text_style, None, font_system);
            let label_node = engine.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
                },
                ..Default::default()
            });

            let step_col = engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: Some(AlignItems::Center),
                    gap: Size {
                        width: zero(),
                        height: length(6.0),
                    },
                    ..Default::default()
                },
                &[circle_node, label_node],
            );
            row_children.push(step_col);

            // Connector line between steps
            if i < self.items.len() - 1 {
                let line_node = engine.new_leaf(Style {
                    size: Size {
                        width: auto(),
                        height: length(2.0),
                    },
                    flex_grow: 1.0,
                    // Vertically center the line at the middle of the circle
                    margin: taffy::Rect {
                        left: length(4.0),
                        right: length(4.0),
                        top: length(circle_sz / 2.0 - 1.0),
                        bottom: zero(),
                    },
                    ..Default::default()
                });
                row_children.push(line_node);
            }
        }

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::FlexStart),
                ..Default::default()
            },
            &row_children,
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &FontSystem,
    ) {
        // Outer row
        let _outer = layouts[*index];
        *index += 1;

        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let circle_sz = self.circle_size();

        for (i, item) in self.items.iter().enumerate() {
            // Step column
            let _step_layout = layouts[*index];
            *index += 1;

            // Circle
            let circle_layout = layouts[*index];
            *index += 1;
            let circle_bounds = mozui_style::Rect::new(
                circle_layout.x,
                circle_layout.y,
                circle_layout.width,
                circle_layout.height,
            );

            let is_completed = i < self.current;
            let is_active = i == self.current;
            let hovered = !self.disabled && interactions.is_hovered(circle_bounds);

            let (circle_bg, circle_fg) = if is_completed {
                (self.completed_color, Color::WHITE)
            } else if is_active {
                (self.active_color, Color::WHITE)
            } else {
                (self.inactive_color, self.muted_color)
            };

            let circle_bg = if hovered {
                circle_bg.with_alpha(alpha * 0.85)
            } else {
                circle_bg.with_alpha(alpha)
            };

            draw_list.push(DrawCommand::Rect {
                bounds: circle_bounds,
                background: Fill::Solid(circle_bg),
                corner_radii: Corners::uniform(circle_sz / 2.0),
                border: None,
            });

            // Circle content: check icon for completed, number for others
            if is_completed {
                draw_list.push(DrawCommand::Icon {
                    name: IconName::Check,
                    bounds: circle_bounds,
                    color: circle_fg.with_alpha(alpha),
                    size_px: circle_sz * 0.5,
                });
            } else if let Some(icon_name) = item.icon {
                draw_list.push(DrawCommand::Icon {
                    name: icon_name,
                    bounds: circle_bounds,
                    color: circle_fg.with_alpha(alpha),
                    size_px: circle_sz * 0.5,
                });
            } else {
                // Number — measure text and center within circle
                let num_text = format!("{}", i + 1);
                let num_font_size = self.num_size();
                let text_style = mozui_text::TextStyle {
                    font_size: num_font_size,
                    color: circle_fg,
                    ..Default::default()
                };
                let measured = mozui_text::measure_text(&num_text, &text_style, None, font_system);
                // Center the text within the circle
                let text_x = circle_bounds.origin.x + (circle_sz - measured.width) / 2.0;
                let text_y = circle_bounds.origin.y + (circle_sz - measured.height) / 2.0;
                draw_list.push(DrawCommand::Text {
                    text: num_text,
                    bounds: mozui_style::Rect::new(text_x, text_y, measured.width, measured.height),
                    font_size: num_font_size,
                    color: circle_fg.with_alpha(alpha),
                    weight: 600,
                    italic: false,
                });
            }

            // Register click handler for step circle
            if !self.disabled {
                if let Some(ref handler) = self.on_click {
                    let handler_ptr =
                        handler.as_ref() as *const dyn Fn(usize, &mut dyn std::any::Any);
                    let step_idx = i;
                    interactions.register_click(
                        circle_bounds,
                        Box::new(move |cx| unsafe { (*handler_ptr)(step_idx, cx) }),
                    );
                }
            }

            // Label text below circle
            let label_layout = layouts[*index];
            *index += 1;
            let label_color = if is_active || is_completed {
                self.label_color
            } else {
                self.muted_color
            };
            draw_list.push(DrawCommand::Text {
                text: item.label.clone(),
                bounds: mozui_style::Rect::new(
                    label_layout.x,
                    label_layout.y,
                    label_layout.width,
                    label_layout.height,
                ),
                font_size: self.label_size(),
                color: label_color.with_alpha(alpha),
                weight: if is_active { 600 } else { 400 },
                italic: false,
            });

            // Connector line between steps
            if i < self.items.len() - 1 {
                let line_layout = layouts[*index];
                *index += 1;
                let line_color = if is_completed {
                    self.completed_color
                } else {
                    self.line_color
                };
                draw_list.push(DrawCommand::Rect {
                    bounds: mozui_style::Rect::new(
                        line_layout.x,
                        line_layout.y,
                        line_layout.width,
                        line_layout.height,
                    ),
                    background: Fill::Solid(line_color.with_alpha(alpha)),
                    corner_radii: Corners::uniform(1.0),
                    border: None,
                });
            }
        }
    }
}
