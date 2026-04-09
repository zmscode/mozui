use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Shadow, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

/// A single item in a menu — either a clickable entry or a separator.
pub enum MenuItem {
    Item {
        label: String,
        icon: Option<IconName>,
        shortcut: Option<String>,
        disabled: bool,
        on_select: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
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
        if let MenuItem::Item { icon: ref mut i, .. } = self {
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
            *h = Some(Box::new(handler));
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
    items: Vec<MenuItem>,
    on_dismiss: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
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
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    pub fn min_width(mut self, v: f32) -> Self {
        self.min_width = v;
        self
    }
}

impl Element for Menu {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut item_nodes = Vec::new();

        for item in &self.items {
            match item {
                MenuItem::Item {
                    label,
                    shortcut,
                    ..
                } => {
                    let mut row_children = Vec::new();

                    // Icon space (always reserve for alignment)
                    row_children.push(engine.new_leaf(Style {
                        size: taffy::Size {
                            width: length(self.icon_size),
                            height: length(self.icon_size),
                        },
                        ..Default::default()
                    }));

                    // Label
                    let style = mozui_text::TextStyle {
                        font_size: self.font_size,
                        color: self.fg,
                        ..Default::default()
                    };
                    let m = mozui_text::measure_text(label, &style, None, font_system);
                    row_children.push(engine.new_leaf(Style {
                        size: taffy::Size {
                            width: length(m.width),
                            height: length(m.height),
                        },
                        flex_grow: 1.0,
                        ..Default::default()
                    }));

                    // Shortcut
                    if let Some(sc) = shortcut {
                        let sc_style = mozui_text::TextStyle {
                            font_size: self.font_size - 1.0,
                            color: self.muted_fg,
                            ..Default::default()
                        };
                        let sc_m = mozui_text::measure_text(sc, &sc_style, None, font_system);
                        row_children.push(engine.new_leaf(Style {
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
                        }));
                    }

                    let row = engine.new_with_children(
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
                    item_nodes.push(row);
                }
                MenuItem::Separator => {
                    item_nodes.push(engine.new_leaf(Style {
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
                    }));
                }
            }
        }

        engine.new_with_children(
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
        // Outer container
        let outer = layouts[*index];
        *index += 1;
        let outer_bounds =
            mozui_style::Rect::new(outer.x, outer.y, outer.width, outer.height);

        // Draw menu background with shadow
        draw_list.push(DrawCommand::Rect {
            bounds: outer_bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
            shadow: Some(self.shadow),
        });

        // Register escape key handler
        if let Some(ref handler) = self.on_dismiss {
            let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            interactions.register_key_handler(Box::new(move |key, _mods, cx| {
                if key == mozui_events::Key::Escape {
                    unsafe { (*handler_ptr)(cx) };
                }
            }));
        }

        for item in &self.items {
            match item {
                MenuItem::Item {
                    label,
                    icon,
                    shortcut,
                    disabled,
                    on_select,
                } => {
                    let row_layout = layouts[*index];
                    *index += 1;
                    let row_bounds = mozui_style::Rect::new(
                        row_layout.x,
                        row_layout.y,
                        row_layout.width,
                        row_layout.height,
                    );

                    let alpha = if *disabled { 0.5 } else { 1.0 };
                    let hovered = !*disabled && interactions.is_hovered(row_bounds);

                    // Hover highlight
                    if hovered {
                        draw_list.push(DrawCommand::Rect {
                            bounds: row_bounds,
                            background: Fill::Solid(self.hover_bg),
                            corner_radii: Corners::uniform(4.0),
                            border: None,
                            shadow: None,
                        });
                    }

                    // Icon
                    let icon_layout = layouts[*index];
                    *index += 1;
                    if let Some(icon_name) = icon {
                        draw_list.push(DrawCommand::Icon {
                            name: *icon_name,
                            weight: IconWeight::Regular,
                            bounds: mozui_style::Rect::new(
                                icon_layout.x,
                                icon_layout.y,
                                icon_layout.width,
                                icon_layout.height,
                            ),
                            color: self.fg.with_alpha(alpha),
                            size_px: self.icon_size,
                        });
                    }

                    // Label
                    let label_layout = layouts[*index];
                    *index += 1;
                    draw_list.push(DrawCommand::Text {
                        text: label.clone(),
                        bounds: mozui_style::Rect::new(
                            label_layout.x,
                            label_layout.y,
                            label_layout.width,
                            label_layout.height,
                        ),
                        font_size: self.font_size,
                        color: self.fg.with_alpha(alpha),
                        weight: 400,
                        italic: false,
                    });

                    // Shortcut
                    if let Some(sc) = shortcut {
                        let sc_layout = layouts[*index];
                        *index += 1;
                        draw_list.push(DrawCommand::Text {
                            text: sc.clone(),
                            bounds: mozui_style::Rect::new(
                                sc_layout.x,
                                sc_layout.y,
                                sc_layout.width,
                                sc_layout.height,
                            ),
                            font_size: self.font_size - 1.0,
                            color: self.muted_fg.with_alpha(alpha),
                            weight: 400,
                            italic: false,
                        });
                    }

                    // Click handler
                    if !*disabled {
                        if let Some(handler) = on_select {
                            let handler_ptr =
                                handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                            // If on_dismiss is set, call both the item handler and dismiss
                            if let Some(ref dismiss) = self.on_dismiss {
                                let dismiss_ptr =
                                    dismiss.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                                interactions.register_click(
                                    row_bounds,
                                    Box::new(move |cx| unsafe {
                                        (*handler_ptr)(cx);
                                        (*dismiss_ptr)(cx);
                                    }),
                                );
                            } else {
                                interactions.register_click(
                                    row_bounds,
                                    Box::new(move |cx| unsafe { (*handler_ptr)(cx) }),
                                );
                            }
                        }
                    }
                }
                MenuItem::Separator => {
                    let sep_layout = layouts[*index];
                    *index += 1;
                    draw_list.push(DrawCommand::Rect {
                        bounds: mozui_style::Rect::new(
                            sep_layout.x + 4.0,
                            sep_layout.y,
                            sep_layout.width - 8.0,
                            sep_layout.height,
                        ),
                        background: Fill::Solid(self.border_color),
                        corner_radii: Corners::ZERO,
                        border: None,
                        shadow: None,
                    });
                }
            }
        }
    }
}
