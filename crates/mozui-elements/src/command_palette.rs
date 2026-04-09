use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::animation::{Animated, Transition};
use mozui_style::{Color, Corners, Fill, Rect, Shadow, Theme};
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
    // Layout IDs
    layout_id: LayoutId,
    search_row_id: LayoutId,
    mag_id: LayoutId,
    search_text_id: LayoutId,
    divider_id: LayoutId,
    list_id: LayoutId,
    item_ids: Vec<LayoutId>,
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
        layout_id: LayoutId::NONE,
        search_row_id: LayoutId::NONE,
        mag_id: LayoutId::NONE,
        search_text_id: LayoutId::NONE,
        divider_id: LayoutId::NONE,
        list_id: LayoutId::NONE,
        item_ids: Vec::new(),
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
    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.item_ids.clear();
        let mut children = Vec::new();

        // Search input area
        let search_style = mozui_text::TextStyle {
            font_size: FONT_SIZE,
            ..Default::default()
        };
        let search_text = if self.query.is_empty() {
            "Type a command\u{2026}"
        } else {
            &self.query
        };
        let search_m = mozui_text::measure_text(search_text, &search_style, None, cx.font_system);

        self.mag_id = cx.new_leaf(Style {
            size: Size {
                width: length(ICON_SIZE),
                height: length(ICON_SIZE),
            },
            ..Default::default()
        });
        self.search_text_id = cx.new_leaf(Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: length(search_m.height),
            },
            ..Default::default()
        });
        self.search_row_id = cx.new_with_children(
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
            &[self.mag_id, self.search_text_id],
        );
        children.push(self.search_row_id);

        // Divider
        self.divider_id = cx.new_leaf(Style {
            size: Size {
                width: percent(1.0),
                height: length(1.0),
            },
            ..Default::default()
        });
        children.push(self.divider_id);

        // Item list
        let filtered_count = self.filtered_items().len();
        let visible = filtered_count.min(MAX_VISIBLE);
        for _ in 0..visible {
            let id = cx.new_leaf(Style {
                size: Size {
                    width: percent(1.0),
                    height: length(ITEM_HEIGHT),
                },
                ..Default::default()
            });
            self.item_ids.push(id);
        }

        // Empty state
        if filtered_count == 0 {
            let id = cx.new_leaf(Style {
                size: Size {
                    width: percent(1.0),
                    height: length(ITEM_HEIGHT),
                },
                ..Default::default()
            });
            self.item_ids.push(id);
        }

        self.list_id = cx.new_with_children(
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
            &self.item_ids,
        );
        children.push(self.list_id);

        self.layout_id = cx.new_with_children(
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
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        let filtered = self.filtered_items();
        let progress = self.anim.as_ref().map(|a| a.get()).unwrap_or(1.0);
        let fade = |c: Color| -> Color { c.with_alpha(c.a * progress) };

        // Container
        let container = cx.engine.bounds(self.layout_id);

        // Scale from 0.97 -> 1.0 during entrance
        let scale = 0.97 + 0.03 * progress;
        let ccx = container.x + container.width / 2.0;
        let ccy = container.y + container.height / 2.0;
        let sw = container.width * scale;
        let sh = container.height * scale;
        let bounds = Rect::new(ccx - sw / 2.0, ccy - sh / 2.0, sw, sh);

        let shadow = if progress < 0.5 {
            None
        } else {
            Some(self.shadow)
        };
        cx.draw_list.push(DrawCommand::Rect {
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
        let search_row = cx.engine.bounds(self.search_row_id);

        // Magnifying glass
        let mag = cx.bounds(self.mag_id);
        cx.draw_list.push(DrawCommand::Icon {
            name: IconName::MagnifyingGlass,
            weight: IconWeight::Regular,
            bounds: mag,
            color: fade(self.muted_fg),
            size_px: ICON_SIZE,
        });

        // Search text
        let search_l = cx.bounds(self.search_text_id);
        let search_text = if self.query.is_empty() {
            "Type a command\u{2026}"
        } else {
            &self.query
        };
        let search_color = if self.query.is_empty() {
            self.muted_fg
        } else {
            self.fg
        };
        cx.draw_list.push(DrawCommand::Text {
            text: search_text.to_string(),
            bounds: Rect::new(search_l.origin.x, search_row.y, search_l.size.width, INPUT_HEIGHT),
            font_size: FONT_SIZE,
            color: fade(search_color),
            weight: 400,
            italic: false,
        });

        // Divider
        let divider = cx.bounds(self.divider_id);
        cx.draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(divider.origin.x, divider.origin.y, divider.size.width, 1.0),
            background: Fill::Solid(fade(self.border_color)),
            corner_radii: Corners::uniform(0.0),
            border: None,
            shadow: None,
        });

        if filtered.is_empty() {
            // Empty state
            let empty_l = cx.bounds(self.item_ids[0]);
            cx.draw_list.push(DrawCommand::Text {
                text: "No results found".to_string(),
                bounds: Rect::new(
                    empty_l.origin.x + PX,
                    empty_l.origin.y,
                    empty_l.size.width - PX * 2.0,
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
            let item_bounds = cx.bounds(self.item_ids[vi]);

            let is_selected = vi == self.selected_index;
            let hovered = !item.disabled && cx.interactions.is_hovered(item_bounds);
            let alpha = if item.disabled { 0.4 } else { 1.0 };
            let a = alpha * progress;

            // Background
            if is_selected {
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: item_bounds,
                    background: Fill::Solid(fade(self.selected_bg)),
                    corner_radii: Corners::uniform(6.0),
                    border: None,
                    shadow: None,
                });
            } else if hovered {
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: item_bounds,
                    background: Fill::Solid(fade(self.hover_bg)),
                    corner_radii: Corners::uniform(6.0),
                    border: None,
                    shadow: None,
                });
            }

            let mut x = item_bounds.origin.x + PX;
            let iy = item_bounds.origin.y + (ITEM_HEIGHT - ICON_SIZE) / 2.0;

            // Icon
            if let Some(icon) = item.icon {
                cx.draw_list.push(DrawCommand::Icon {
                    name: icon,
                    weight: IconWeight::Regular,
                    bounds: Rect::new(x, iy, ICON_SIZE, ICON_SIZE),
                    color: self.muted_fg.with_alpha(a),
                    size_px: ICON_SIZE,
                });
                x += ICON_SIZE + GAP;
            }

            // Label
            cx.draw_list.push(DrawCommand::Text {
                text: item.label.clone(),
                bounds: Rect::new(x, item_bounds.origin.y, item_bounds.size.width * 0.6, ITEM_HEIGHT),
                font_size: FONT_SIZE,
                color: self.fg.with_alpha(a),
                weight: 400,
                italic: false,
            });

            // Shortcut (right-aligned)
            if let Some(ref shortcut) = item.shortcut {
                cx.draw_list.push(DrawCommand::Text {
                    text: shortcut.clone(),
                    bounds: Rect::new(
                        item_bounds.origin.x + item_bounds.size.width - PX - 80.0,
                        item_bounds.origin.y,
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
                    cx.interactions.register_click(
                        item_bounds,
                        Box::new(move |cx| unsafe { (*ptr)(&id, cx) }),
                    );
                }
                cx.interactions.register_hover_region(item_bounds);
            }
        }
    }
}
