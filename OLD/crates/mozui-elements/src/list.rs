use crate::styled::{ComponentSize, Disableable, Selectable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use std::rc::Rc;
use taffy::prelude::*;

pub struct ListItem {
    label: String,
    description: Option<String>,
    icon: Option<IconName>,
    selected: bool,
    disabled: bool,
    separator: bool,
    on_click: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn list_item(label: impl Into<String>) -> ListItem {
    ListItem {
        label: label.into(),
        description: None,
        icon: None,
        selected: false,
        disabled: false,
        separator: false,
        on_click: None,
    }
}

impl ListItem {
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn separator(mut self) -> Self {
        self.separator = true;
        self
    }
}

impl Selectable for ListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Disableable for ListItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Tracks layout IDs for a single list item row.
struct ListItemLayout {
    /// For separators this is the divider leaf; for normal items it's the item row node.
    item_id: LayoutId,
    icon_id: Option<LayoutId>,
    label_id: LayoutId,
    desc_id: Option<LayoutId>,
}

pub struct List {
    layout_id: LayoutId,
    item_layouts: Vec<ListItemLayout>,

    items: Vec<ListItem>,
    size: ComponentSize,
    fg: Color,
    muted_fg: Color,
    selected_bg: Color,
    selected_fg: Color,
    hover_bg: Color,
    divider_color: Color,
}

pub fn list(theme: &Theme) -> List {
    List {
        layout_id: LayoutId::NONE,
        item_layouts: Vec::new(),

        items: Vec::new(),
        size: ComponentSize::Medium,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        selected_bg: theme.primary,
        selected_fg: theme.primary_foreground,
        hover_bg: theme.secondary,
        divider_color: theme.border,
    }
}

impl List {
    pub fn child(mut self, item: ListItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn children(mut self, items: impl IntoIterator<Item = ListItem>) -> Self {
        self.items.extend(items);
        self
    }

    fn text_size(&self) -> f32 {
        self.size.input_text_size()
    }

    fn desc_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 10.0,
            ComponentSize::Small => 11.0,
            ComponentSize::Medium => 12.0,
            ComponentSize::Large => 13.0,
            ComponentSize::Custom(_) => 12.0,
        }
    }

    fn icon_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 14.0,
            ComponentSize::Small => 16.0,
            ComponentSize::Medium => 18.0,
            ComponentSize::Large => 20.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }

    fn item_py(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 4.0,
            ComponentSize::Small => 6.0,
            ComponentSize::Medium => 8.0,
            ComponentSize::Large => 10.0,
            ComponentSize::Custom(_) => 8.0,
        }
    }

    fn item_px(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 6.0,
            ComponentSize::Small => 8.0,
            ComponentSize::Medium => 12.0,
            ComponentSize::Large => 16.0,
            ComponentSize::Custom(_) => 12.0,
        }
    }
}

impl Sizable for List {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Element for List {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "List",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let px = self.item_px();
        let py = self.item_py();
        let font_size = self.text_size();
        let desc_size = self.desc_size();
        let icon_sz = self.icon_size();

        let mut row_nodes = Vec::new();
        self.item_layouts = Vec::new();

        for item in &self.items {
            if item.separator {
                // Divider: 1px line with vertical margin
                let div_id = cx.new_leaf(Style {
                    size: Size {
                        width: percent(1.0),
                        height: length(1.0),
                    },
                    margin: taffy::Rect {
                        left: zero(),
                        right: zero(),
                        top: length(4.0),
                        bottom: length(4.0),
                    },
                    ..Default::default()
                });
                row_nodes.push(div_id);
                self.item_layouts.push(ListItemLayout {
                    item_id: div_id,
                    icon_id: None,
                    label_id: LayoutId::NONE,
                    desc_id: None,
                });
                continue;
            }

            let mut item_children = Vec::new();

            // Icon
            let icon_id = if item.icon.is_some() {
                let id = cx.new_leaf(Style {
                    size: Size {
                        width: length(icon_sz),
                        height: length(icon_sz),
                    },
                    ..Default::default()
                });
                item_children.push(id);
                Some(id)
            } else {
                None
            };

            // Text column (label + optional description)
            let mut text_children = Vec::new();

            let label_style = mozui_text::TextStyle {
                font_size,
                color: self.fg,
                ..Default::default()
            };
            let label_m = mozui_text::measure_text(&item.label, &label_style, None, cx.font_system);
            let label_id = cx.new_leaf(Style {
                size: Size {
                    width: length(label_m.width),
                    height: length(label_m.height),
                },
                ..Default::default()
            });
            text_children.push(label_id);

            let desc_id = if let Some(ref desc) = item.description {
                let desc_style = mozui_text::TextStyle {
                    font_size: desc_size,
                    color: self.muted_fg,
                    ..Default::default()
                };
                let desc_m = mozui_text::measure_text(desc, &desc_style, None, cx.font_system);
                let id = cx.new_leaf(Style {
                    size: Size {
                        width: length(desc_m.width),
                        height: length(desc_m.height),
                    },
                    ..Default::default()
                });
                text_children.push(id);
                Some(id)
            } else {
                None
            };

            let text_col_id = cx.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    gap: Size {
                        width: zero(),
                        height: length(2.0),
                    },
                    ..Default::default()
                },
                &text_children,
            );
            item_children.push(text_col_id);

            let item_id = cx.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    padding: taffy::Rect {
                        left: length(px),
                        right: length(px),
                        top: length(py),
                        bottom: length(py),
                    },
                    gap: Size {
                        width: length(10.0),
                        height: zero(),
                    },
                    ..Default::default()
                },
                &item_children,
            );
            row_nodes.push(item_id);

            self.item_layouts.push(ListItemLayout {
                item_id,
                icon_id,
                label_id,
                desc_id,
            });
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &row_nodes,
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        let font_size = self.text_size();
        let desc_size = self.desc_size();
        let icon_sz = self.icon_size();

        for (idx, item) in self.items.iter().enumerate() {
            let il = &self.item_layouts[idx];

            if item.separator {
                let div_bounds = cx.bounds(il.item_id);
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: div_bounds,
                    background: Fill::Solid(self.divider_color),
                    corner_radii: Corners::uniform(0.0),
                    border: None,
                    shadow: None, shadows: vec![],
                });
                continue;
            }

            let item_bounds = cx.bounds(il.item_id);

            let alpha = if item.disabled { 0.5 } else { 1.0 };
            let hovered = !item.disabled && !item.selected && cx.interactions.is_hovered(item_bounds);

            // Background
            let bg = if item.selected {
                self.selected_bg.with_alpha(alpha)
            } else if hovered {
                self.hover_bg.with_alpha(alpha)
            } else {
                Color::TRANSPARENT
            };
            if bg.a > 0.0 {
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: item_bounds,
                    background: Fill::Solid(bg),
                    corner_radii: Corners::uniform(6.0),
                    border: None,
                    shadow: None, shadows: vec![],
                });
            }

            let fg = if item.selected {
                self.selected_fg.with_alpha(alpha)
            } else {
                self.fg.with_alpha(alpha)
            };
            let muted = if item.selected {
                self.selected_fg.with_alpha(alpha * 0.7)
            } else {
                self.muted_fg.with_alpha(alpha)
            };

            // Icon
            if let Some(icon_name) = item.icon {
                let icon_bounds = cx.bounds(il.icon_id.unwrap());
                cx.draw_list.push(DrawCommand::Icon {
                    name: icon_name,
                    weight: IconWeight::Regular,
                    bounds: icon_bounds,
                    color: fg,
                    size_px: icon_sz,
                });
            }

            // Label
            let label_bounds = cx.bounds(il.label_id);
            cx.draw_list.push(DrawCommand::Text {
                text: item.label.clone(),
                bounds: label_bounds,
                font_size,
                color: fg,
                weight: 400,
                italic: false,
            });

            // Description
            if let Some(ref desc) = item.description {
                let desc_bounds = cx.bounds(il.desc_id.unwrap());
                cx.draw_list.push(DrawCommand::Text {
                    text: desc.clone(),
                    bounds: desc_bounds,
                    font_size: desc_size,
                    color: muted,
                    weight: 400,
                    italic: false,
                });
            }

            // Click handler
            if !item.disabled {
                if let Some(ref handler) = item.on_click {
                    cx.interactions.register_click(
                        item_bounds,
                        handler.clone(),
                    );
                }
            }
        }
    }
}
