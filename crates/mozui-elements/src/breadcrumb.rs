use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Rect, Theme};
use std::rc::Rc;
use taffy::prelude::*;

pub struct BreadcrumbItem {
    label: String,
    icon: Option<IconName>,
    on_click: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
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
        self.on_click = Some(Rc::new(handler));
        self
    }
}

/// Tracks layout IDs for a single breadcrumb entry.
struct BreadcrumbEntryLayout {
    icon_id: Option<LayoutId>,
    label_id: LayoutId,
    separator_id: Option<LayoutId>,
}

pub struct Breadcrumb {
    layout_id: LayoutId,
    entry_layouts: Vec<BreadcrumbEntryLayout>,

    items: Vec<BreadcrumbItem>,
    font_size: f32,
    color: Color,
    active_color: Color,
    separator_color: Color,
}

pub fn breadcrumb(theme: &Theme) -> Breadcrumb {
    Breadcrumb {
        layout_id: LayoutId::NONE,
        entry_layouts: Vec::new(),

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
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Breadcrumb",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let mut row_children = Vec::new();
        self.entry_layouts = Vec::new();

        for (i, item) in self.items.iter().enumerate() {
            // Optional icon
            let icon_id = if item.icon.is_some() {
                let id = cx.new_leaf(Style {
                    size: Size {
                        width: length(14.0),
                        height: length(14.0),
                    },
                    ..Default::default()
                });
                row_children.push(id);
                Some(id)
            } else {
                None
            };

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
            let measured = mozui_text::measure_text(&item.label, &text_style, None, cx.font_system);
            let label_id = cx.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
                },
                ..Default::default()
            });
            row_children.push(label_id);

            // Separator (except after last)
            let separator_id = if !is_last {
                let id = cx.new_leaf(Style {
                    size: Size {
                        width: length(14.0),
                        height: length(14.0),
                    },
                    ..Default::default()
                });
                row_children.push(id);
                Some(id)
            } else {
                None
            };

            self.entry_layouts.push(BreadcrumbEntryLayout {
                icon_id,
                label_id,
                separator_id,
            });
        }

        self.layout_id = cx.new_with_children(
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
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        for (i, item) in self.items.iter().enumerate() {
            let el = &self.entry_layouts[i];
            let is_last = i == self.items.len() - 1;

            // Icon
            if let Some(icon_name) = item.icon {
                let icon_bounds = cx.bounds(el.icon_id.unwrap());
                cx.draw_list.push(DrawCommand::Icon {
                    name: icon_name,
                    weight: IconWeight::Regular,
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
            let text_bounds = cx.bounds(el.label_id);

            let hovered =
                !is_last && item.on_click.is_some() && cx.interactions.is_hovered(text_bounds);
            let color = if is_last {
                self.active_color
            } else if hovered {
                self.active_color
            } else {
                self.color
            };

            cx.draw_list.push(DrawCommand::Text {
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
                    cx.interactions.register_click(
                        text_bounds,
                        handler.clone(),
                    );
                }
            }

            // Separator chevron
            if let Some(sep_id) = el.separator_id {
                let sep_bounds = cx.bounds(sep_id);
                cx.draw_list.push(DrawCommand::Icon {
                    name: IconName::CaretRight,
                    weight: IconWeight::Regular,
                    bounds: sep_bounds,
                    color: self.separator_color,
                    size_px: 14.0,
                });
            }
        }
    }
}
