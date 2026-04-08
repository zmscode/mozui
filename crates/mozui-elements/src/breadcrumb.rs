use crate::{Element, InteractionMap};
use mozui_icons::IconName;
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct BreadcrumbItem {
    label: String,
    icon: Option<IconName>,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn breadcrumb_item(label: impl Into<String>) -> BreadcrumbItem {
    BreadcrumbItem {
        label: label.into(),
        icon: None,
        on_click: None,
    }
}

impl BreadcrumbItem {
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

pub struct Breadcrumb {
    items: Vec<BreadcrumbItem>,
    font_size: f32,
    color: Color,
    active_color: Color,
    separator_color: Color,
}

pub fn breadcrumb(theme: &Theme) -> Breadcrumb {
    Breadcrumb {
        items: Vec::new(),
        font_size: 13.0,
        color: theme.muted_foreground,
        active_color: theme.foreground,
        separator_color: theme.muted_foreground,
    }
}

impl Breadcrumb {
    pub fn child(mut self, item: BreadcrumbItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn children(mut self, items: impl IntoIterator<Item = BreadcrumbItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
}

impl Element for Breadcrumb {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut row_children = Vec::new();

        for (i, item) in self.items.iter().enumerate() {
            // Optional icon
            if let Some(_icon) = item.icon {
                row_children.push(engine.new_leaf(Style {
                    size: Size {
                        width: length(14.0),
                        height: length(14.0),
                    },
                    ..Default::default()
                }));
            }

            // Label text
            let is_last = i == self.items.len() - 1;
            let color = if is_last {
                self.active_color
            } else {
                self.color
            };
            let text_style = mozui_text::TextStyle {
                font_size: self.font_size,
                color,
                ..Default::default()
            };
            let measured = mozui_text::measure_text(&item.label, &text_style, None, font_system);
            row_children.push(engine.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
                },
                ..Default::default()
            }));

            // Separator (except after last)
            if !is_last {
                row_children.push(engine.new_leaf(Style {
                    size: Size {
                        width: length(14.0),
                        height: length(14.0),
                    },
                    ..Default::default()
                }));
            }
        }

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: Size {
                    width: length(4.0),
                    height: zero(),
                },
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
        _font_system: &FontSystem,
    ) {
        let _outer = layouts[*index];
        *index += 1;

        for (i, item) in self.items.iter().enumerate() {
            let is_last = i == self.items.len() - 1;

            // Icon
            if let Some(icon_name) = item.icon {
                let icon_layout = layouts[*index];
                *index += 1;
                let icon_bounds = mozui_style::Rect::new(
                    icon_layout.x,
                    icon_layout.y,
                    icon_layout.width,
                    icon_layout.height,
                );
                draw_list.push(DrawCommand::Icon {
                    name: icon_name,
                    bounds: icon_bounds,
                    color: if is_last {
                        self.active_color
                    } else {
                        self.color
                    },
                    size_px: 14.0,
                });
            }

            // Label
            let text_layout = layouts[*index];
            *index += 1;
            let text_bounds = mozui_style::Rect::new(
                text_layout.x,
                text_layout.y,
                text_layout.width,
                text_layout.height,
            );

            let hovered =
                !is_last && item.on_click.is_some() && interactions.is_hovered(text_bounds);
            let color = if is_last {
                self.active_color
            } else if hovered {
                self.active_color
            } else {
                self.color
            };

            draw_list.push(DrawCommand::Text {
                text: item.label.clone(),
                bounds: text_bounds,
                font_size: self.font_size,
                color,
                weight: if is_last { 600 } else { 400 },
                italic: false,
            });

            // Click handler (not on last item)
            if !is_last {
                if let Some(ref handler) = item.on_click {
                    let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                    interactions.register_click(
                        text_bounds,
                        Box::new(move |cx| unsafe { (*handler_ptr)(cx) }),
                    );
                }
            }

            // Separator chevron
            if !is_last {
                let sep_layout = layouts[*index];
                *index += 1;
                draw_list.push(DrawCommand::Icon {
                    name: IconName::CaretRight,
                    bounds: mozui_style::Rect::new(
                        sep_layout.x,
                        sep_layout.y,
                        sep_layout.width,
                        sep_layout.height,
                    ),
                    color: self.separator_color,
                    size_px: 14.0,
                });
            }
        }
    }
}
