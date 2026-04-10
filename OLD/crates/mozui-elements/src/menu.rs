use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Shadow, Theme};
use std::rc::Rc;
use taffy::prelude::*;

/// A single item in a menu — either a clickable entry or a separator.
pub enum MenuItem {
    Item {
        label: String,
        icon: Option<IconName>,
        shortcut: Option<String>,
        disabled: bool,
        on_select: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
    },
    Separator,
}

pub fn menu_item(label: impl Into<String>) -> MenuItem {
    MenuItem::Item {
        label: label.into(),
        icon: None,
        shortcut: None,
        disabled: false,
        on_select: None,
    }
}

pub fn menu_separator() -> MenuItem {
    MenuItem::Separator
}

impl MenuItem {
    pub fn icon(mut self, icon: IconName) -> Self {
        if let MenuItem::Item {
            icon: ref mut i, ..
        } = self
        {
            *i = Some(icon);
        }
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        if let MenuItem::Item {
            shortcut: ref mut s,
            ..
        } = self
        {
            *s = Some(shortcut.into());
        }
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        if let MenuItem::Item {
            disabled: ref mut d,
            ..
        } = self
        {
            *d = disabled;
        }
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        if let MenuItem::Item {
            on_select: ref mut h,
            ..
        } = self
        {
            *h = Some(Rc::new(handler));
        }
        self
    }
}

/// A dropdown / context menu with keyboard navigation.
///
/// ```rust,ignore
/// menu(&theme)
///     .item(menu_item("Cut").icon(IconName::Scissors).shortcut("⌘X"))
///     .item(menu_item("Copy").icon(IconName::Copy).shortcut("⌘C"))
///     .item(menu_separator())
///     .item(menu_item("Paste").icon(IconName::ClipboardText).shortcut("⌘V"))
///     .on_dismiss(|cx| { /* close menu */ })
/// ```
pub struct Menu {
    layout_id: LayoutId,
    /// Per-item layout IDs: for Item -> [row, icon, label, (shortcut)], for Separator -> [sep]
    item_ids: Vec<LayoutId>,

    items: Vec<MenuItem>,
    on_dismiss: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
    bg: Color,
    fg: Color,
    muted_fg: Color,
    hover_bg: Color,
    border_color: Color,
    shadow: Shadow,
    corner_radius: f32,
    min_width: f32,
    font_size: f32,
    icon_size: f32,
    item_py: f32,
    item_px: f32,
}

pub fn menu(theme: &Theme) -> Menu {
    Menu {
        layout_id: LayoutId::NONE,
        item_ids: Vec::new(),
        items: Vec::new(),
        on_dismiss: None,
        bg: theme.popover,
        fg: theme.popover_foreground,
        muted_fg: theme.muted_foreground,
        hover_bg: theme.secondary,
        border_color: theme.border,
        shadow: theme.shadow_md,
        corner_radius: theme.radius_md,
        min_width: 180.0,
        font_size: theme.font_size_sm,
        icon_size: 16.0,
        item_py: 6.0,
        item_px: 8.0,
    }
}

impl Menu {
    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Handler called when the menu should close (Escape or selection).
    pub fn on_dismiss(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.min_width = v;
        self
    }
}

impl Element for Menu {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Menu",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.item_ids.clear();
        let mut item_nodes = Vec::new();

        for item in &self.items {
            match item {
                MenuItem::Item {
                    label, shortcut, ..
                } => {
                    let mut row_children = Vec::new();

                    // Icon space (always reserve for alignment)
                    let icon_id = cx.new_leaf(Style {
                        size: taffy::Size {
                            width: length(self.icon_size),
                            height: length(self.icon_size),
                        },
                        ..Default::default()
                    });
                    row_children.push(icon_id);

                    // Label
                    let style = mozui_text::TextStyle {
                        font_size: self.font_size,
                        color: self.fg,
                        ..Default::default()
                    };
                    let m = mozui_text::measure_text(label, &style, None, cx.font_system);
                    let label_id = cx.new_leaf(Style {
                        size: taffy::Size {
                            width: length(m.width),
                            height: length(m.height),
                        },
                        flex_grow: 1.0,
                        ..Default::default()
                    });
                    row_children.push(label_id);

                    // Shortcut
                    let shortcut_id = if let Some(sc) = shortcut {
                        let sc_style = mozui_text::TextStyle {
                            font_size: self.font_size - 1.0,
                            color: self.muted_fg,
                            ..Default::default()
                        };
                        let sc_m = mozui_text::measure_text(sc, &sc_style, None, cx.font_system);
                        let sc_id = cx.new_leaf(Style {
                            size: taffy::Size {
                                width: length(sc_m.width),
                                height: length(sc_m.height),
                            },
                            margin: taffy::Rect {
                                left: length(24.0),
                                right: zero(),
                                top: zero(),
                                bottom: zero(),
                            },
                            ..Default::default()
                        });
                        row_children.push(sc_id);
                        Some(sc_id)
                    } else {
                        None
                    };

                    let row_id = cx.new_with_children(
                        Style {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            align_items: Some(AlignItems::Center),
                            padding: taffy::Rect {
                                left: length(self.item_px),
                                right: length(self.item_px),
                                top: length(self.item_py),
                                bottom: length(self.item_py),
                            },
                            gap: taffy::Size {
                                width: length(8.0),
                                height: zero(),
                            },
                            ..Default::default()
                        },
                        &row_children,
                    );
                    // Store: row, icon, label, [shortcut]
                    self.item_ids.push(row_id);
                    self.item_ids.push(icon_id);
                    self.item_ids.push(label_id);
                    if let Some(sc_id) = shortcut_id {
                        self.item_ids.push(sc_id);
                    }
                    item_nodes.push(row_id);
                }
                MenuItem::Separator => {
                    let sep_id = cx.new_leaf(Style {
                        size: taffy::Size {
                            width: percent(1.0),
                            height: length(1.0),
                        },
                        margin: taffy::Rect {
                            top: length(4.0),
                            bottom: length(4.0),
                            left: zero(),
                            right: zero(),
                        },
                        ..Default::default()
                    });
                    self.item_ids.push(sep_id);
                    item_nodes.push(sep_id);
                }
            }
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                min_size: taffy::Size {
                    width: length(self.min_width),
                    height: auto(),
                },
                padding: taffy::Rect {
                    left: length(4.0),
                    right: length(4.0),
                    top: length(4.0),
                    bottom: length(4.0),
                },
                ..Default::default()
            },
            &item_nodes,
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Draw menu background with shadow
        cx.draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
            shadow: Some(self.shadow), shadows: vec![],
        });

        // Register escape key handler
        if let Some(ref handler) = self.on_dismiss {
            let h = handler.clone();
            cx.interactions.register_key_handler(Rc::new(move |key, _mods, cx| {
                if key == mozui_events::Key::Escape {
                    h(cx);
                }
            }));
        }

        let mut id_idx = 0;
        for item in &self.items {
            match item {
                MenuItem::Item {
                    label,
                    icon,
                    shortcut,
                    disabled,
                    on_select,
                } => {
                    let row_id = self.item_ids[id_idx];
                    let icon_id = self.item_ids[id_idx + 1];
                    let label_id = self.item_ids[id_idx + 2];
                    id_idx += 3;

                    let shortcut_id = if shortcut.is_some() {
                        let sc_id = self.item_ids[id_idx];
                        id_idx += 1;
                        Some(sc_id)
                    } else {
                        None
                    };

                    let row_bounds = cx.bounds(row_id);
                    let alpha = if *disabled { 0.5 } else { 1.0 };
                    let hovered = !*disabled && cx.interactions.is_hovered(row_bounds);

                    // Hover highlight
                    if hovered {
                        cx.draw_list.push(DrawCommand::Rect {
                            bounds: row_bounds,
                            background: Fill::Solid(self.hover_bg),
                            corner_radii: Corners::uniform(4.0),
                            border: None,
                            shadow: None, shadows: vec![],
                        });
                    }

                    // Icon
                    let icon_bounds = cx.bounds(icon_id);
                    if let Some(icon_name) = icon {
                        cx.draw_list.push(DrawCommand::Icon {
                            name: *icon_name,
                            weight: IconWeight::Regular,
                            bounds: icon_bounds,
                            color: self.fg.with_alpha(alpha),
                            size_px: self.icon_size,
                        });
                    }

                    // Label
                    let label_bounds = cx.bounds(label_id);
                    cx.draw_list.push(DrawCommand::Text {
                        text: label.clone(),
                        bounds: label_bounds,
                        font_size: self.font_size,
                        color: self.fg.with_alpha(alpha),
                        weight: 400,
                        italic: false,
                    });

                    // Shortcut
                    if let Some(sc) = shortcut {
                        let sc_bounds = cx.bounds(shortcut_id.unwrap());
                        cx.draw_list.push(DrawCommand::Text {
                            text: sc.clone(),
                            bounds: sc_bounds,
                            font_size: self.font_size - 1.0,
                            color: self.muted_fg.with_alpha(alpha),
                            weight: 400,
                            italic: false,
                        });
                    }

                    // Click handler
                    if !*disabled {
                        if let Some(handler) = on_select {
                            // If on_dismiss is set, call both the item handler and dismiss
                            if let Some(ref dismiss) = self.on_dismiss {
                                let h = handler.clone();
                                let d = dismiss.clone();
                                cx.interactions.register_click(
                                    row_bounds,
                                    Rc::new(move |cx: &mut dyn std::any::Any| {
                                        h(cx);
                                        d(cx);
                                    }),
                                );
                            } else {
                                cx.interactions.register_click(
                                    row_bounds,
                                    handler.clone(),
                                );
                            }
                        }
                    }
                }
                MenuItem::Separator => {
                    let sep_id = self.item_ids[id_idx];
                    id_idx += 1;
                    let sep_bounds = cx.bounds(sep_id);
                    cx.draw_list.push(DrawCommand::Rect {
                        bounds: Rect::new(
                            sep_bounds.origin.x + 4.0,
                            sep_bounds.origin.y,
                            sep_bounds.size.width - 8.0,
                            sep_bounds.size.height,
                        ),
                        background: Fill::Solid(self.border_color),
                        corner_radii: Corners::ZERO,
                        border: None,
                        shadow: None, shadows: vec![],
                    });
                }
            }
        }
    }
}
