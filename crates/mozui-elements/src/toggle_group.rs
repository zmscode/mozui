use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use taffy::prelude::*;

const ITEM_HEIGHT: f32 = 32.0;
const ITEM_PX: f32 = 14.0;
const ICON_SIZE: f32 = 16.0;
const GAP: f32 = 6.0;
const FONT_SIZE: f32 = 13.0;

/// A single item in a toggle group.
pub struct ToggleItem {
    pub value: String,
    pub label: String,
    pub icon: Option<IconName>,
    pub disabled: bool,
}

pub fn toggle_item(value: impl Into<String>, label: impl Into<String>) -> ToggleItem {
    ToggleItem {
        value: value.into(),
        label: label.into(),
        icon: None,
        disabled: false,
    }
}

impl ToggleItem {
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A group of mutually-exclusive toggle buttons, like a segmented control.
pub struct ToggleGroup {
    items: Vec<ToggleItem>,
    selected: Option<String>,
    on_change: Option<Box<dyn Fn(&str, &mut dyn std::any::Any)>>,
    // Theme
    bg: Color,
    muted_fg: Color,
    selected_bg: Color,
    selected_fg: Color,
    hover_bg: Color,
    border_color: Color,
    corner_radius: f32,
    // Layout IDs
    layout_id: LayoutId,
    item_ids: Vec<LayoutId>,
}

pub fn toggle_group(theme: &Theme) -> ToggleGroup {
    ToggleGroup {
        items: Vec::new(),
        selected: None,
        on_change: None,
        bg: theme.muted,
        muted_fg: theme.muted_foreground,
        selected_bg: theme.background,
        selected_fg: theme.foreground,
        hover_bg: theme.secondary_hover,
        border_color: theme.border,
        corner_radius: theme.radius_md,
        layout_id: LayoutId::NONE,
        item_ids: Vec::new(),
    }
}

impl ToggleGroup {
    pub fn items(mut self, items: Vec<ToggleItem>) -> Self {
        self.items = items;
        self
    }

    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected = Some(value.into());
        self
    }

    pub fn on_change(mut self, f: impl Fn(&str, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }
}

impl Element for ToggleGroup {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "ToggleGroup",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.item_ids.clear();

        for item in &self.items {
            let text_style = mozui_text::TextStyle {
                font_size: FONT_SIZE,
                ..Default::default()
            };
            let measured = mozui_text::measure_text(&item.label, &text_style, None, cx.font_system);
            let content_width = if item.icon.is_some() {
                ICON_SIZE + GAP + measured.width
            } else {
                measured.width
            };

            self.item_ids.push(cx.new_leaf(Style {
                size: Size {
                    width: length(content_width + ITEM_PX * 2.0),
                    height: length(ITEM_HEIGHT),
                },
                ..Default::default()
            }));
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                padding: taffy::Rect {
                    left: length(3.0),
                    right: length(3.0),
                    top: length(3.0),
                    bottom: length(3.0),
                },
                gap: Size {
                    width: length(2.0),
                    height: zero(),
                },
                ..Default::default()
            },
            &self.item_ids,
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Container background (pill shape)
        cx.draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius + 2.0),
            border: None,
            shadow: None,
        });

        for i in 0..self.items.len() {
            let cell_bounds = cx.bounds(self.item_ids[i]);

            let is_selected = self
                .selected
                .as_ref()
                .map_or(false, |s| *s == self.items[i].value);
            let hovered =
                !self.items[i].disabled && !is_selected && cx.interactions.is_hovered(cell_bounds);
            let alpha = if self.items[i].disabled { 0.4 } else { 1.0 };

            // Item background
            if is_selected {
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: cell_bounds,
                    background: Fill::Solid(self.selected_bg),
                    corner_radii: Corners::uniform(self.corner_radius),
                    border: Some(Border {
                        width: 1.0,
                        color: self.border_color,
                    }),
                    shadow: None,
                });
            } else if hovered {
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: cell_bounds,
                    background: Fill::Solid(self.hover_bg),
                    corner_radii: Corners::uniform(self.corner_radius),
                    border: None,
                    shadow: None,
                });
            }

            let fg = if is_selected {
                self.selected_fg
            } else {
                self.muted_fg
            };

            let mut x = cell_bounds.origin.x + ITEM_PX;
            let cy = cell_bounds.origin.y + (ITEM_HEIGHT - ICON_SIZE) / 2.0;

            // Optional icon
            if let Some(icon) = self.items[i].icon {
                cx.draw_list.push(DrawCommand::Icon {
                    name: icon,
                    weight: IconWeight::Regular,
                    bounds: Rect::new(x, cy, ICON_SIZE, ICON_SIZE),
                    color: fg.with_alpha(alpha),
                    size_px: ICON_SIZE,
                });
                x += ICON_SIZE + GAP;
            }

            // Label
            cx.draw_list.push(DrawCommand::Text {
                text: self.items[i].label.clone(),
                bounds: Rect::new(
                    x,
                    cell_bounds.origin.y,
                    cell_bounds.size.width - (x - cell_bounds.origin.x) - ITEM_PX,
                    ITEM_HEIGHT,
                ),
                font_size: FONT_SIZE,
                color: fg.with_alpha(alpha),
                weight: if is_selected { 600 } else { 400 },
                italic: false,
            });

            // Click handler
            if !self.items[i].disabled && !is_selected {
                if let Some(ref on_change) = self.on_change {
                    let value = self.items[i].value.clone();
                    let ptr = on_change.as_ref() as *const dyn Fn(&str, &mut dyn std::any::Any);
                    cx.interactions.register_click(
                        cell_bounds,
                        Box::new(move |cx| unsafe { (*ptr)(&value, cx) }),
                    );
                }
                cx.interactions.register_hover_region(cell_bounds);
            }
        }
    }
}
