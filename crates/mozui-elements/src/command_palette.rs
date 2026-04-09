use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::animation::{Animated, Transition};
use mozui_style::{Color, Corners, Fill, Shadow, Theme};
use mozui_text::FontSystem;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;
use taffy::prelude::*;

const INPUT_HEIGHT: f32 = 40.0;
const ITEM_HEIGHT: f32 = 36.0;
const ICON_SIZE: f32 = 16.0;
const SHORTCUT_SIZE: f32 = 11.0;
const PX: f32 = 14.0;
const PAD: f32 = 6.0;
const GAP: f32 = 6.0;
const FONT_SIZE: f32 = 14.0;
const MAX_VISIBLE: usize = 8;
const ANIM_MS: u64 = 150;

/// A single command in the palette.
pub struct CommandItem {
    pub id: String,
    pub label: String,
    pub icon: Option<IconName>,
    pub shortcut: Option<String>,
    pub disabled: bool,
}

pub fn command_item(id: impl Into<String>, label: impl Into<String>) -> CommandItem {
    CommandItem {
        id: id.into(),
        label: label.into(),
        icon: None,
        shortcut: None,
        disabled: false,
    }
}

impl CommandItem {
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A searchable command palette (like VS Code's Cmd+K).
///
/// Renders as a search input + filtered list of commands.
/// Designed to be shown inside a dialog or overlay.
pub struct CommandPalette {
    items: Vec<CommandItem>,
    query: String,
    selected_index: usize,
    on_select: Option<Box<dyn Fn(&str, &mut dyn std::any::Any)>>,
    on_query_change: Option<Box<dyn Fn(&str, &mut dyn std::any::Any)>>,
    // Theme
    bg: Color,
    fg: Color,
    muted_fg: Color,
    selected_bg: Color,
    hover_bg: Color,
    border_color: Color,
    corner_radius: f32,
    shadow: Shadow,
    width: f32,
    anim: Option<Animated<f32>>,
}

/// Create an entrance animation handle for a command palette.
/// Store this in state and pass via `.anim()`.
pub fn command_palette_anim(animation_flag: Rc<Cell<bool>>) -> Animated<f32> {
    let transition =
        Transition::new(Duration::from_millis(ANIM_MS)).custom_bezier(0.4, 0.0, 0.2, 1.0);
    let anim = Animated::new(0.0, transition, animation_flag);
    anim.set(1.0);
    anim
}

pub fn command_palette(theme: &Theme) -> CommandPalette {
    CommandPalette {
        items: Vec::new(),
        query: String::new(),
        selected_index: 0,
        on_select: None,
        on_query_change: None,
        bg: theme.popover,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        selected_bg: theme.secondary,
        hover_bg: theme.secondary_hover,
        border_color: theme.border,
        corner_radius: theme.radius_lg,
        shadow: theme.shadow_lg,
        width: 480.0,
        anim: None,
    }
}

impl CommandPalette {
    pub fn items(mut self, items: Vec<CommandItem>) -> Self {
        self.items = items;
        self
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    pub fn selected_index(mut self, idx: usize) -> Self {
        self.selected_index = idx;
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    pub fn on_select(mut self, f: impl Fn(&str, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }

    pub fn on_query_change(mut self, f: impl Fn(&str, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_query_change = Some(Box::new(f));
        self
    }

    /// Attach a persisted animation handle (from `command_palette_anim()`).
    pub fn anim(mut self, anim: Animated<f32>) -> Self {
        self.anim = Some(anim);
        self
    }

    fn filtered_items(&self) -> Vec<(usize, &CommandItem)> {
        if self.query.is_empty() {
            self.items.iter().enumerate().collect()
        } else {
            let q = self.query.to_lowercase();
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.label.to_lowercase().contains(&q))
                .collect()
        }
    }
}

impl Element for CommandPalette {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut children = Vec::new();

        // Search input area
        let search_style = mozui_text::TextStyle {
            font_size: FONT_SIZE,
            ..Default::default()
        };
        let search_text = if self.query.is_empty() {
            "Type a command…"
        } else {
            &self.query
        };
        let search_m = mozui_text::measure_text(search_text, &search_style, None, font_system);

        let mag_icon = engine.new_leaf(Style {
            size: Size {
                width: length(ICON_SIZE),
                height: length(ICON_SIZE),
            },
            ..Default::default()
        });
        let search_text_node = engine.new_leaf(Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: length(search_m.height),
            },
            ..Default::default()
        });
        let search_row = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: Size {
                    width: percent(1.0),
                    height: length(INPUT_HEIGHT),
                },
                padding: taffy::Rect {
                    left: length(PX),
                    right: length(PX),
                    top: zero(),
                    bottom: zero(),
                },
                gap: Size {
                    width: length(GAP + 4.0),
                    height: zero(),
                },
                ..Default::default()
            },
            &[mag_icon, search_text_node],
        );
        children.push(search_row);

        // Divider
        children.push(engine.new_leaf(Style {
            size: Size {
                width: percent(1.0),
                height: length(1.0),
            },
            ..Default::default()
        }));

        // Item list
        let filtered = self.filtered_items();
        let visible = filtered.len().min(MAX_VISIBLE);
        let mut item_nodes = Vec::new();
        for _ in 0..visible {
            item_nodes.push(engine.new_leaf(Style {
                size: Size {
                    width: percent(1.0),
                    height: length(ITEM_HEIGHT),
                },
                ..Default::default()
            }));
        }

        // Empty state
        if filtered.is_empty() {
            item_nodes.push(engine.new_leaf(Style {
                size: Size {
                    width: percent(1.0),
                    height: length(ITEM_HEIGHT),
                },
                ..Default::default()
            }));
        }

        let list = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: taffy::Rect {
                    left: length(PAD),
                    right: length(PAD),
                    top: length(PAD),
                    bottom: length(PAD),
                },
                ..Default::default()
            },
            &item_nodes,
        );
        children.push(list);

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: length(self.width),
                    height: auto(),
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
        _font_system: &FontSystem,
    ) {
        let filtered = self.filtered_items();
        let progress = self.anim.as_ref().map(|a| a.get()).unwrap_or(1.0);
        let fade = |c: Color| -> Color { c.with_alpha(c.a * progress) };

        // Container
        let container = layouts[*index];
        *index += 1;

        // Scale from 0.97 → 1.0 during entrance
        let scale = 0.97 + 0.03 * progress;
        let cx = container.x + container.width / 2.0;
        let cy = container.y + container.height / 2.0;
        let sw = container.width * scale;
        let sh = container.height * scale;
        let bounds = mozui_style::Rect::new(cx - sw / 2.0, cy - sh / 2.0, sw, sh);

        let shadow = if progress < 0.5 {
            None
        } else {
            Some(self.shadow)
        };
        draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(fade(self.bg)),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: fade(self.border_color),
            }),
            shadow,
        });

        // Search row
        let search_row = layouts[*index];
        *index += 1;

        // Magnifying glass
        let mag = layouts[*index];
        *index += 1;
        draw_list.push(DrawCommand::Icon {
            name: IconName::MagnifyingGlass,
            weight: IconWeight::Regular,
            bounds: mozui_style::Rect::new(mag.x, mag.y, mag.width, mag.height),
            color: fade(self.muted_fg),
            size_px: ICON_SIZE,
        });

        // Search text
        let search_l = layouts[*index];
        *index += 1;
        let search_text = if self.query.is_empty() {
            "Type a command…"
        } else {
            &self.query
        };
        let search_color = if self.query.is_empty() {
            self.muted_fg
        } else {
            self.fg
        };
        draw_list.push(DrawCommand::Text {
            text: search_text.to_string(),
            bounds: mozui_style::Rect::new(search_l.x, search_row.y, search_l.width, INPUT_HEIGHT),
            font_size: FONT_SIZE,
            color: fade(search_color),
            weight: 400,
            italic: false,
        });

        // Divider
        let divider = layouts[*index];
        *index += 1;
        draw_list.push(DrawCommand::Rect {
            bounds: mozui_style::Rect::new(divider.x, divider.y, divider.width, 1.0),
            background: Fill::Solid(fade(self.border_color)),
            corner_radii: Corners::uniform(0.0),
            border: None,
            shadow: None,
        });

        // Item list container
        let _list = layouts[*index];
        *index += 1;

        if filtered.is_empty() {
            // Empty state
            let empty_l = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Text {
                text: "No results found".to_string(),
                bounds: mozui_style::Rect::new(
                    empty_l.x + PX,
                    empty_l.y,
                    empty_l.width - PX * 2.0,
                    ITEM_HEIGHT,
                ),
                font_size: FONT_SIZE,
                color: fade(self.muted_fg),
                weight: 400,
                italic: true,
            });
            return;
        }

        let visible = filtered.len().min(MAX_VISIBLE);
        for (vi, (_orig_idx, item)) in filtered.iter().take(visible).enumerate() {
            let item_l = layouts[*index];
            *index += 1;
            let item_bounds =
                mozui_style::Rect::new(item_l.x, item_l.y, item_l.width, item_l.height);

            let is_selected = vi == self.selected_index;
            let hovered = !item.disabled && interactions.is_hovered(item_bounds);
            let alpha = if item.disabled { 0.4 } else { 1.0 };
            let a = alpha * progress;

            // Background
            if is_selected {
                draw_list.push(DrawCommand::Rect {
                    bounds: item_bounds,
                    background: Fill::Solid(fade(self.selected_bg)),
                    corner_radii: Corners::uniform(6.0),
                    border: None,
                    shadow: None,
                });
            } else if hovered {
                draw_list.push(DrawCommand::Rect {
                    bounds: item_bounds,
                    background: Fill::Solid(fade(self.hover_bg)),
                    corner_radii: Corners::uniform(6.0),
                    border: None,
                    shadow: None,
                });
            }

            let mut x = item_l.x + PX;
            let iy = item_l.y + (ITEM_HEIGHT - ICON_SIZE) / 2.0;

            // Icon
            if let Some(icon) = item.icon {
                draw_list.push(DrawCommand::Icon {
                    name: icon,
                    weight: IconWeight::Regular,
                    bounds: mozui_style::Rect::new(x, iy, ICON_SIZE, ICON_SIZE),
                    color: self.muted_fg.with_alpha(a),
                    size_px: ICON_SIZE,
                });
                x += ICON_SIZE + GAP;
            }

            // Label
            draw_list.push(DrawCommand::Text {
                text: item.label.clone(),
                bounds: mozui_style::Rect::new(x, item_l.y, item_l.width * 0.6, ITEM_HEIGHT),
                font_size: FONT_SIZE,
                color: self.fg.with_alpha(a),
                weight: 400,
                italic: false,
            });

            // Shortcut (right-aligned)
            if let Some(ref shortcut) = item.shortcut {
                draw_list.push(DrawCommand::Text {
                    text: shortcut.clone(),
                    bounds: mozui_style::Rect::new(
                        item_l.x + item_l.width - PX - 80.0,
                        item_l.y,
                        80.0,
                        ITEM_HEIGHT,
                    ),
                    font_size: SHORTCUT_SIZE,
                    color: self.muted_fg.with_alpha(a),
                    weight: 400,
                    italic: false,
                });
            }

            // Click handler
            if !item.disabled {
                if let Some(ref on_select) = self.on_select {
                    let id = item.id.clone();
                    let ptr = on_select.as_ref() as *const dyn Fn(&str, &mut dyn std::any::Any);
                    interactions.register_click(
                        item_bounds,
                        Box::new(move |cx| unsafe { (*ptr)(&id, cx) }),
                    );
                }
                interactions.register_hover_region(item_bounds);
            }
        }
    }
}
