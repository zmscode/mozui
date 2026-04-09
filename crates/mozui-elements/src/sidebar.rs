use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

const EXPANDED_WIDTH: f32 = 240.0;
const COLLAPSED_WIDTH: f32 = 48.0;
const ITEM_HEIGHT: f32 = 32.0;
const ICON_SIZE: f32 = 18.0;
const FONT_SIZE: f32 = 13.0;
const PAD_X: f32 = 12.0;
const GAP: f32 = 6.0;
const TOGGLE_SIZE: f32 = 28.0;

/// Side of the window the sidebar appears on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarSide {
    Left,
    Right,
}

/// A menu item in the sidebar.
pub struct SidebarItem {
    _id: String,
    label: String,
    icon: Option<IconName>,
    active: bool,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn sidebar_item(id: impl Into<String>, label: impl Into<String>) -> SidebarItem {
    SidebarItem {
        _id: id.into(),
        label: label.into(),
        icon: None,
        active: false,
        disabled: false,
        on_click: None,
    }
}

impl SidebarItem {
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, f: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

/// A labeled group of sidebar items.
pub struct SidebarGroup {
    label: Option<String>,
    items: Vec<SidebarItem>,
}

pub fn sidebar_group() -> SidebarGroup {
    SidebarGroup {
        label: None,
        items: Vec::new(),
    }
}

impl SidebarGroup {
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn item(mut self, item: SidebarItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<SidebarItem>) -> Self {
        self.items = items;
        self
    }
}

/// A sidebar navigation panel with toggle button and animated width.
///
/// Pass `width_factor` as 0.0 (collapsed) to 1.0 (expanded).
/// Animate it with `cx.use_animated()` for smooth transitions.
pub struct Sidebar {
    side: SidebarSide,
    width_factor: f32,
    groups: Vec<SidebarGroup>,
    on_toggle: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    header: Option<Box<dyn Element>>,
    footer: Option<Box<dyn Element>>,
    // Theme
    bg: Color,
    fg: Color,
    muted_fg: Color,
    active_bg: Color,
    active_fg: Color,
    hover_bg: Color,
    border_color: Color,
    corner_radius: f32,
}

pub fn sidebar(theme: &Theme) -> Sidebar {
    Sidebar {
        side: SidebarSide::Left,
        width_factor: 1.0,
        groups: Vec::new(),
        on_toggle: None,
        header: None,
        footer: None,
        bg: theme.surface,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        active_bg: theme.secondary,
        active_fg: theme.foreground,
        hover_bg: theme.secondary_hover,
        border_color: theme.border,
        corner_radius: theme.radius_md,
    }
}

impl Sidebar {
    pub fn side(mut self, side: SidebarSide) -> Self {
        self.side = side;
        self
    }

    /// Width factor: 0.0 = collapsed, 1.0 = fully expanded.
    /// Animate with `cx.use_animated()`.
    pub fn width_factor(mut self, f: f32) -> Self {
        self.width_factor = f.clamp(0.0, 1.0);
        self
    }

    pub fn group(mut self, group: SidebarGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Toggle button callback. An icon button is drawn at the bottom of the sidebar.
    pub fn on_toggle(mut self, f: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }

    pub fn header(mut self, element: impl Element + 'static) -> Self {
        self.header = Some(Box::new(element));
        self
    }

    pub fn footer(mut self, element: impl Element + 'static) -> Self {
        self.footer = Some(Box::new(element));
        self
    }

    fn current_width(&self) -> f32 {
        COLLAPSED_WIDTH + (EXPANDED_WIDTH - COLLAPSED_WIDTH) * self.width_factor
    }

    fn is_expanded(&self) -> bool {
        self.width_factor > 0.5
    }
}

impl Element for Sidebar {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut children = Vec::new();

        // Header
        if let Some(ref header) = self.header {
            children.push(header.layout(engine, font_system));
        }

        // Groups
        for group in &self.groups {
            if group.label.is_some() && self.is_expanded() {
                children.push(engine.new_leaf(Style {
                    size: Size {
                        width: percent(1.0),
                        height: length(28.0),
                    },
                    ..Default::default()
                }));
            }

            for _item in &group.items {
                children.push(engine.new_leaf(Style {
                    size: Size {
                        width: percent(1.0),
                        height: length(ITEM_HEIGHT),
                    },
                    ..Default::default()
                }));
            }
        }

        // Spacer to push toggle to bottom
        children.push(engine.new_leaf(Style {
            flex_grow: 1.0,
            ..Default::default()
        }));

        // Footer
        if let Some(ref footer) = self.footer {
            children.push(footer.layout(engine, font_system));
        }

        // Toggle button row
        if self.on_toggle.is_some() {
            children.push(engine.new_leaf(Style {
                size: Size {
                    width: percent(1.0),
                    height: length(TOGGLE_SIZE),
                },
                ..Default::default()
            }));
        }

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                size: Size {
                    width: length(self.current_width()),
                    height: percent(1.0),
                },
                padding: taffy::Rect {
                    left: length(8.0),
                    right: length(8.0),
                    top: length(8.0),
                    bottom: length(8.0),
                },
                gap: Size {
                    width: zero(),
                    height: length(2.0),
                },
                overflow: taffy::Point {
                    x: taffy::Overflow::Hidden,
                    y: taffy::Overflow::Hidden,
                },
                ..Default::default()
            },
            &children,
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
        let container = layouts[*index];
        *index += 1;
        let bounds =
            mozui_style::Rect::new(container.x, container.y, container.width, container.height);

        // Background
        draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::ZERO,
            border: None,
            shadow: None,
        });

        // Border on inner edge
        let border_x = match self.side {
            SidebarSide::Left => container.x + container.width - 1.0,
            SidebarSide::Right => container.x,
        };
        draw_list.push(DrawCommand::Rect {
            bounds: mozui_style::Rect::new(border_x, container.y, 1.0, container.height),
            background: Fill::Solid(self.border_color),
            corner_radii: Corners::ZERO,
            border: None,
            shadow: None,
        });

        // Clip content
        draw_list.push_clip(bounds);

        // Header
        if let Some(ref header) = self.header {
            header.paint(layouts, index, draw_list, interactions, font_system);
        }

        let show_labels = self.is_expanded();
        let text_alpha = ((self.width_factor - 0.5) * 2.0).clamp(0.0, 1.0);

        // Groups
        for group in &self.groups {
            // Group label
            if let Some(ref lbl) = group.label {
                if show_labels {
                    let l = layouts[*index];
                    *index += 1;
                    draw_list.push(DrawCommand::Text {
                        text: lbl.to_uppercase(),
                        bounds: mozui_style::Rect::new(
                            l.x + PAD_X,
                            l.y + 8.0,
                            l.width - PAD_X * 2.0,
                            20.0,
                        ),
                        font_size: 10.0,
                        color: self.muted_fg.with_alpha(text_alpha * 0.7),
                        weight: 600,
                        italic: false,
                    });
                }
            }

            for item in &group.items {
                let l = layouts[*index];
                *index += 1;
                let item_bounds =
                    mozui_style::Rect::new(l.x, l.y, l.width, l.height);

                let alpha = if item.disabled { 0.4 } else { 1.0 };
                let hovered = !item.disabled && interactions.is_hovered(item_bounds);

                // Background
                if item.active {
                    draw_list.push(DrawCommand::Rect {
                        bounds: item_bounds,
                        background: Fill::Solid(self.active_bg),
                        corner_radii: Corners::uniform(self.corner_radius),
                        border: None,
                        shadow: None,
                    });
                } else if hovered {
                    draw_list.push(DrawCommand::Rect {
                        bounds: item_bounds,
                        background: Fill::Solid(self.hover_bg),
                        corner_radii: Corners::uniform(self.corner_radius),
                        border: None,
                        shadow: None,
                    });
                }

                let fg = if item.active {
                    self.active_fg.with_alpha(alpha)
                } else {
                    self.fg.with_alpha(alpha)
                };

                // Center icon in the collapsed state
                let icon_x = if show_labels {
                    l.x + PAD_X
                } else {
                    l.x + (l.width - ICON_SIZE) / 2.0
                };
                let icon_y = l.y + (ITEM_HEIGHT - ICON_SIZE) / 2.0;

                if let Some(icon) = item.icon {
                    let icon_color = if item.active { fg } else { self.muted_fg.with_alpha(alpha) };
                    draw_list.push(DrawCommand::Icon {
                        name: icon,
                        weight: if item.active { IconWeight::Fill } else { IconWeight::Regular },
                        bounds: mozui_style::Rect::new(icon_x, icon_y, ICON_SIZE, ICON_SIZE),
                        color: icon_color,
                        size_px: ICON_SIZE,
                    });
                }

                // Label (fades with animation)
                if show_labels {
                    let text_x = l.x + PAD_X + ICON_SIZE + GAP;
                    draw_list.push(DrawCommand::Text {
                        text: item.label.clone(),
                        bounds: mozui_style::Rect::new(
                            text_x,
                            l.y,
                            l.width - (text_x - l.x) - PAD_X,
                            ITEM_HEIGHT,
                        ),
                        font_size: FONT_SIZE,
                        color: fg.with_alpha(fg.a * text_alpha),
                        weight: if item.active { 500 } else { 400 },
                        italic: false,
                    });
                }

                if !item.disabled {
                    if let Some(ref on_click) = item.on_click {
                        let ptr = on_click.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                        interactions.register_click(
                            item_bounds,
                            Box::new(move |cx| unsafe { (*ptr)(cx) }),
                        );
                    }
                    interactions.register_hover_region(item_bounds);
                }
            }
        }

        // Spacer
        let _spacer = layouts[*index];
        *index += 1;

        // Footer
        if let Some(ref footer) = self.footer {
            footer.paint(layouts, index, draw_list, interactions, font_system);
        }

        // Toggle button
        if let Some(ref on_toggle) = self.on_toggle {
            let l = layouts[*index];
            *index += 1;
            let btn_bounds = mozui_style::Rect::new(l.x, l.y, l.width, l.height);
            let hovered = interactions.is_hovered(btn_bounds);

            if hovered {
                draw_list.push(DrawCommand::Rect {
                    bounds: btn_bounds,
                    background: Fill::Solid(self.hover_bg),
                    corner_radii: Corners::uniform(self.corner_radius),
                    border: None,
                    shadow: None,
                });
            }

            let toggle_icon = match (self.side, self.is_expanded()) {
                (SidebarSide::Left, true) => IconName::CaretLeft,
                (SidebarSide::Left, false) => IconName::CaretRight,
                (SidebarSide::Right, true) => IconName::CaretRight,
                (SidebarSide::Right, false) => IconName::CaretLeft,
            };

            let icon_x = if show_labels {
                l.x + PAD_X
            } else {
                l.x + (l.width - 16.0) / 2.0
            };
            let icon_y = l.y + (TOGGLE_SIZE - 16.0) / 2.0;

            draw_list.push(DrawCommand::Icon {
                name: toggle_icon,
                weight: IconWeight::Bold,
                bounds: mozui_style::Rect::new(icon_x, icon_y, 16.0, 16.0),
                color: self.muted_fg,
                size_px: 16.0,
            });

            if show_labels {
                draw_list.push(DrawCommand::Text {
                    text: "Collapse".to_string(),
                    bounds: mozui_style::Rect::new(
                        icon_x + 16.0 + GAP,
                        l.y,
                        l.width - PAD_X * 2.0 - 16.0 - GAP,
                        TOGGLE_SIZE,
                    ),
                    font_size: 12.0,
                    color: self.muted_fg.with_alpha(text_alpha),
                    weight: 400,
                    italic: false,
                });
            }

            let ptr = on_toggle.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            interactions.register_click(
                btn_bounds,
                Box::new(move |cx| unsafe { (*ptr)(cx) }),
            );
            interactions.register_hover_region(btn_bounds);
        }

        draw_list.pop_clip();
    }
}
