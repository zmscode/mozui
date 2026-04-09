use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Rect, Theme};
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

/// Tracks layout IDs for a single step.
struct StepLayout {
    circle_id: LayoutId,
    label_id: LayoutId,
    line_id: Option<LayoutId>,
}

pub struct Stepper {
    layout_id: LayoutId,
    step_layouts: Vec<StepLayout>,

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
        layout_id: LayoutId::NONE,
        step_layouts: Vec::new(),

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
    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Layout: single flex row with alternating [step-column, connector-line] pairs.
        // Each step-column is: circle (fixed) + label text below it.
        let circle_sz = self.circle_size();
        let mut row_children = Vec::new();
        self.step_layouts = Vec::new();

        for (i, item) in self.items.iter().enumerate() {
            // Step column: circle + label stacked vertically, centered
            let circle_id = cx.new_leaf(Style {
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
            let measured = mozui_text::measure_text(&item.label, &text_style, None, cx.font_system);
            let label_id = cx.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
                },
                ..Default::default()
            });

            let step_col_id = cx.new_with_children(
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
                &[circle_id, label_id],
            );
            row_children.push(step_col_id);

            // Connector line between steps
            let line_id = if i < self.items.len() - 1 {
                let id = cx.new_leaf(Style {
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
                row_children.push(id);
                Some(id)
            } else {
                None
            };

            self.step_layouts.push(StepLayout {
                circle_id,
                label_id,
                line_id,
            });
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::FlexStart),
                ..Default::default()
            },
            &row_children,
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let circle_sz = self.circle_size();

        for (i, item) in self.items.iter().enumerate() {
            let sl = &self.step_layouts[i];

            // Circle
            let circle_bounds = cx.bounds(sl.circle_id);

            let is_completed = i < self.current;
            let is_active = i == self.current;
            let hovered = !self.disabled && cx.interactions.is_hovered(circle_bounds);

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

            cx.draw_list.push(DrawCommand::Rect {
                bounds: circle_bounds,
                background: Fill::Solid(circle_bg),
                corner_radii: Corners::uniform(circle_sz / 2.0),
                border: None,
                shadow: None,
            });

            // Circle content: check icon for completed, number for others
            if is_completed {
                cx.draw_list.push(DrawCommand::Icon {
                    name: IconName::Check,
                    weight: IconWeight::Regular,
                    bounds: circle_bounds,
                    color: circle_fg.with_alpha(alpha),
                    size_px: circle_sz * 0.5,
                });
            } else if let Some(icon_name) = item.icon {
                cx.draw_list.push(DrawCommand::Icon {
                    name: icon_name,
                    weight: IconWeight::Regular,
                    bounds: circle_bounds,
                    color: circle_fg.with_alpha(alpha),
                    size_px: circle_sz * 0.5,
                });
            } else {
                // Number -- measure text and center within circle
                let num_text = format!("{}", i + 1);
                let num_font_size = self.num_size();
                let text_style = mozui_text::TextStyle {
                    font_size: num_font_size,
                    color: circle_fg,
                    ..Default::default()
                };
                let measured =
                    mozui_text::measure_text(&num_text, &text_style, None, cx.font_system);
                // Center the text within the circle
                let text_x = circle_bounds.origin.x + (circle_sz - measured.width) / 2.0;
                let text_y = circle_bounds.origin.y + (circle_sz - measured.height) / 2.0;
                cx.draw_list.push(DrawCommand::Text {
                    text: num_text,
                    bounds: Rect::new(text_x, text_y, measured.width, measured.height),
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
                    cx.interactions.register_click(
                        circle_bounds,
                        Box::new(move |cx| unsafe { (*handler_ptr)(step_idx, cx) }),
                    );
                }
            }

            // Label text below circle
            let label_bounds = cx.bounds(sl.label_id);
            let label_color = if is_active || is_completed {
                self.label_color
            } else {
                self.muted_color
            };
            cx.draw_list.push(DrawCommand::Text {
                text: item.label.clone(),
                bounds: label_bounds,
                font_size: self.label_size(),
                color: label_color.with_alpha(alpha),
                weight: if is_active { 600 } else { 400 },
                italic: false,
            });

            // Connector line between steps
            if let Some(line_id) = sl.line_id {
                let line_bounds = cx.bounds(line_id);
                let line_color = if is_completed {
                    self.completed_color
                } else {
                    self.line_color
                };
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: line_bounds,
                    background: Fill::Solid(line_color.with_alpha(alpha)),
                    corner_radii: Corners::uniform(1.0),
                    border: None,
                    shadow: None,
                });
            }
        }
    }
}
