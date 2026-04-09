use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Shadow, Theme};
use std::rc::Rc;
use taffy::prelude::*;

/// A single option in a select dropdown.
#[derive(Clone)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

/// Create a select option.
pub fn select_option(value: impl Into<String>, label: impl Into<String>) -> SelectOption {
    SelectOption {
        value: value.into(),
        label: label.into(),
        disabled: false,
    }
}

impl SelectOption {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A dropdown select component.
///
/// Renders as a button-like trigger. When `open` is true, renders a dropdown
/// list below the trigger with selectable options.
///
/// ```rust,ignore
/// let options = vec![
///     select_option("us", "United States"),
///     select_option("uk", "United Kingdom"),
///     select_option("de", "Germany"),
/// ];
/// select(&theme)
///     .options(options)
///     .selected("us")
///     .placeholder("Choose a country…")
///     .open(is_open)
///     .on_select(move |value, cx| { /* handle selection */ })
///     .on_toggle(move |cx| { /* toggle open state */ })
/// ```
pub struct Select {
    options: Vec<SelectOption>,
    selected_value: Option<String>,
    placeholder: String,
    open: bool,
    on_select: Option<Rc<dyn Fn(&str, &mut dyn std::any::Any)>>,
    on_toggle: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
    // Theme colors
    bg: Color,
    fg: Color,
    muted_fg: Color,
    hover_bg: Color,
    selected_bg: Color,
    border_color: Color,
    popover_bg: Color,
    shadow: Shadow,
    corner_radius: f32,
    font_size: f32,
    min_width: f32,
    disabled: bool,
    // Searchable (combobox)
    searchable: bool,
    search_text: String,
    // Layout IDs
    layout_id: LayoutId,
    trigger_id: LayoutId,
    text_id: LayoutId,
    chevron_id: LayoutId,
    dropdown_id: LayoutId,
    search_id: LayoutId,
    item_ids: Vec<LayoutId>,
}

pub fn select(theme: &Theme) -> Select {
    Select {
        options: Vec::new(),
        selected_value: None,
        placeholder: "Select…".into(),
        open: false,
        on_select: None,
        on_toggle: None,
        bg: theme.background,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        hover_bg: theme.secondary,
        selected_bg: theme.primary,
        border_color: theme.border,
        popover_bg: theme.popover,
        shadow: theme.shadow_md,
        corner_radius: theme.radius_md,
        font_size: theme.font_size_sm,
        min_width: 180.0,
        disabled: false,
        searchable: false,
        search_text: String::new(),
        layout_id: LayoutId::NONE,
        trigger_id: LayoutId::NONE,
        text_id: LayoutId::NONE,
        chevron_id: LayoutId::NONE,
        dropdown_id: LayoutId::NONE,
        search_id: LayoutId::NONE,
        item_ids: Vec::new(),
    }
}

impl Select {
    pub fn options(mut self, options: Vec<SelectOption>) -> Self {
        self.options = options;
        self
    }

    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected_value = Some(value.into());
        self
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&str, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn on_toggle(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    pub fn min_width(mut self, w: f32) -> Self {
        self.min_width = w;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Make this a searchable select (combobox).
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Set the current search text (for combobox mode).
    pub fn search_text(mut self, text: impl Into<String>) -> Self {
        self.search_text = text.into();
        self
    }

    fn selected_label(&self) -> Option<&str> {
        self.selected_value.as_ref().and_then(|val| {
            self.options
                .iter()
                .find(|o| o.value == *val)
                .map(|o| o.label.as_str())
        })
    }

    fn filtered_options(&self) -> Vec<&SelectOption> {
        if !self.searchable || self.search_text.is_empty() {
            self.options.iter().collect()
        } else {
            let query = self.search_text.to_lowercase();
            self.options
                .iter()
                .filter(|o| o.label.to_lowercase().contains(&query))
                .collect()
        }
    }
}

const TRIGGER_HEIGHT: f32 = 34.0;
const TRIGGER_PX: f32 = 12.0;
const ITEM_PY: f32 = 6.0;
const ITEM_PX: f32 = 8.0;
const DROPDOWN_PAD: f32 = 4.0;
const GAP: f32 = 4.0;
const CHEVRON_SIZE: f32 = 14.0;

impl Element for Select {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Select",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let display_text = self.selected_label().unwrap_or(&self.placeholder);
        let text_style = mozui_text::TextStyle {
            font_size: self.font_size,
            color: self.fg,
            ..Default::default()
        };
        let measured = mozui_text::measure_text(display_text, &text_style, None, cx.font_system);

        // Trigger button: [text] [chevron]
        self.text_id = cx.new_leaf(Style {
            size: taffy::Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            flex_grow: 1.0,
            ..Default::default()
        });
        self.chevron_id = cx.new_leaf(Style {
            size: taffy::Size {
                width: length(CHEVRON_SIZE),
                height: length(CHEVRON_SIZE),
            },
            ..Default::default()
        });
        self.trigger_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                min_size: taffy::Size {
                    width: length(self.min_width),
                    height: length(TRIGGER_HEIGHT),
                },
                padding: taffy::Rect {
                    left: length(TRIGGER_PX),
                    right: length(TRIGGER_PX),
                    top: zero(),
                    bottom: zero(),
                },
                gap: taffy::Size {
                    width: length(8.0),
                    height: zero(),
                },
                ..Default::default()
            },
            &[self.text_id, self.chevron_id],
        );

        if !self.open {
            self.layout_id = self.trigger_id;
            return self.layout_id;
        }

        // Dropdown list — collect filtered indices to avoid borrowing self
        let filtered_indices: Vec<usize> = if !self.searchable || self.search_text.is_empty() {
            (0..self.options.len()).collect()
        } else {
            let query = self.search_text.to_lowercase();
            self.options
                .iter()
                .enumerate()
                .filter(|(_, o)| o.label.to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect()
        };
        self.item_ids.clear();

        // Optional search input
        if self.searchable {
            let search_style = mozui_text::TextStyle {
                font_size: self.font_size,
                color: self.fg,
                ..Default::default()
            };
            let search_text = if self.search_text.is_empty() {
                "Search…"
            } else {
                &self.search_text
            };
            let sm = mozui_text::measure_text(search_text, &search_style, None, cx.font_system);
            self.search_id = cx.new_leaf(Style {
                size: taffy::Size {
                    width: percent(1.0),
                    height: length(sm.height + ITEM_PY * 2.0),
                },
                ..Default::default()
            });
        }

        for &idx in &filtered_indices {
            let style = mozui_text::TextStyle {
                font_size: self.font_size,
                color: self.fg,
                ..Default::default()
            };
            let m = mozui_text::measure_text(&self.options[idx].label, &style, None, cx.font_system);
            self.item_ids.push(cx.new_leaf(Style {
                size: taffy::Size {
                    width: percent(1.0),
                    height: length(m.height + ITEM_PY * 2.0),
                },
                padding: taffy::Rect {
                    left: length(ITEM_PX),
                    right: length(ITEM_PX),
                    top: length(ITEM_PY),
                    bottom: length(ITEM_PY),
                },
                ..Default::default()
            }));
        }

        let mut dropdown_children = Vec::new();
        if self.searchable {
            dropdown_children.push(self.search_id);
        }
        dropdown_children.extend_from_slice(&self.item_ids);

        self.dropdown_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                min_size: taffy::Size {
                    width: length(self.min_width),
                    height: auto(),
                },
                padding: taffy::Rect {
                    left: length(DROPDOWN_PAD),
                    right: length(DROPDOWN_PAD),
                    top: length(DROPDOWN_PAD),
                    bottom: length(DROPDOWN_PAD),
                },
                ..Default::default()
            },
            &dropdown_children,
        );

        // Wrapper: trigger + dropdown stacked vertically
        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                gap: taffy::Size {
                    width: zero(),
                    height: length(GAP),
                },
                ..Default::default()
            },
            &[self.trigger_id, self.dropdown_id],
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        if !self.open {
            // Just the trigger
            self.paint_trigger(cx);
            return;
        }

        // Trigger
        self.paint_trigger(cx);

        // Dropdown
        let dropdown_bounds = cx.bounds(self.dropdown_id);

        // Dropdown background
        cx.draw_list.push(DrawCommand::Rect {
            bounds: dropdown_bounds,
            background: Fill::Solid(self.popover_bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
            shadow: Some(self.shadow),
        });

        // Search input (if searchable)
        if self.searchable {
            let search_bounds = cx.bounds(self.search_id);

            // Search input background
            cx.draw_list.push(DrawCommand::Rect {
                bounds: Rect::new(
                    search_bounds.origin.x,
                    search_bounds.origin.y,
                    search_bounds.size.width,
                    search_bounds.size.height,
                ),
                background: Fill::Solid(self.bg),
                corner_radii: Corners::uniform(self.corner_radius),
                border: Some(Border {
                    width: 1.0,
                    color: self.border_color,
                }),
                shadow: None,
            });

            let text = if self.search_text.is_empty() {
                "Search…"
            } else {
                &self.search_text
            };
            let color = if self.search_text.is_empty() {
                self.muted_fg
            } else {
                self.fg
            };
            cx.draw_list.push(DrawCommand::Text {
                text: text.to_string(),
                bounds: Rect::new(
                    search_bounds.origin.x + ITEM_PX,
                    search_bounds.origin.y + ITEM_PY,
                    search_bounds.size.width - ITEM_PX * 2.0,
                    search_bounds.size.height - ITEM_PY * 2.0,
                ),
                font_size: self.font_size,
                color,
                weight: 400,
                italic: false,
            });
        }

        // Options
        let filtered = self.filtered_options();
        for i in 0..filtered.len() {
            let item_bounds = cx.bounds(self.item_ids[i]);

            let is_selected = self
                .selected_value
                .as_ref()
                .map_or(false, |v| *v == filtered[i].value);
            let alpha = if filtered[i].disabled { 0.5 } else { 1.0 };
            let hovered = !filtered[i].disabled && cx.interactions.is_hovered(item_bounds);

            // Hover / selected highlight
            if hovered || is_selected {
                let bg = if is_selected {
                    self.selected_bg
                } else {
                    self.hover_bg
                };
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: item_bounds,
                    background: Fill::Solid(bg),
                    corner_radii: Corners::uniform(4.0),
                    border: None,
                    shadow: None,
                });
            }

            // Label text
            let text_color = if is_selected {
                Color::WHITE
            } else {
                self.fg.with_alpha(alpha)
            };
            cx.draw_list.push(DrawCommand::Text {
                text: filtered[i].label.clone(),
                bounds: Rect::new(
                    item_bounds.origin.x + ITEM_PX,
                    item_bounds.origin.y + ITEM_PY,
                    item_bounds.size.width - ITEM_PX * 2.0,
                    item_bounds.size.height - ITEM_PY * 2.0,
                ),
                font_size: self.font_size,
                color: text_color,
                weight: if is_selected { 600 } else { 400 },
                italic: false,
            });

            // Check mark for selected item
            if is_selected {
                cx.draw_list.push(DrawCommand::Icon {
                    name: IconName::Check,
                    weight: IconWeight::Bold,
                    bounds: Rect::new(
                        item_bounds.origin.x + item_bounds.size.width - ITEM_PX - 14.0,
                        item_bounds.origin.y + (item_bounds.size.height - 14.0) / 2.0,
                        14.0,
                        14.0,
                    ),
                    color: Color::WHITE,
                    size_px: 14.0,
                });
            }

            // Click handler
            if !filtered[i].disabled {
                if let Some(ref handler) = self.on_select {
                    let value = filtered[i].value.clone();
                    // Also dismiss on select
                    if let Some(ref toggle) = self.on_toggle {
                        let h = handler.clone();
                        let t = toggle.clone();
                        cx.interactions.register_click(
                            item_bounds,
                            Rc::new(move |cx: &mut dyn std::any::Any| {
                                h(&value, cx);
                                t(cx);
                            }),
                        );
                    } else {
                        let h = handler.clone();
                        cx.interactions.register_click(
                            item_bounds,
                            Rc::new(move |cx: &mut dyn std::any::Any| {
                                h(&value, cx);
                            }),
                        );
                    }
                }
            }
        }

        // Escape to close
        if let Some(ref toggle) = self.on_toggle {
            let t = toggle.clone();
            cx.interactions
                .register_key_handler(Rc::new(move |key, _mods, cx| {
                    if key == mozui_events::Key::Escape {
                        t(cx);
                    }
                }));
        }
    }
}

impl Select {
    fn paint_trigger(&self, cx: &mut PaintContext) {
        let trigger_bounds = cx.bounds(self.trigger_id);

        let hovered = !self.disabled && cx.interactions.is_hovered(trigger_bounds);
        let active = !self.disabled && cx.interactions.is_active(trigger_bounds);

        // Trigger background
        let bg = if active {
            self.hover_bg
        } else if hovered {
            Color::new(
                self.bg.r * 0.95 + self.hover_bg.r * 0.05,
                self.bg.g * 0.95 + self.hover_bg.g * 0.05,
                self.bg.b * 0.95 + self.hover_bg.b * 0.05,
                1.0,
            )
        } else {
            self.bg
        };

        cx.draw_list.push(DrawCommand::Rect {
            bounds: trigger_bounds,
            background: Fill::Solid(bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: if self.open {
                    self.selected_bg
                } else {
                    self.border_color
                },
            }),
            shadow: None,
        });

        // Text
        let text_bounds = cx.bounds(self.text_id);
        let display_text = self.selected_label().unwrap_or(&self.placeholder);
        let text_color = if self.selected_value.is_some() {
            self.fg
        } else {
            self.muted_fg
        };
        cx.draw_list.push(DrawCommand::Text {
            text: display_text.to_string(),
            bounds: text_bounds,
            font_size: self.font_size,
            color: if self.disabled {
                text_color.with_alpha(0.5)
            } else {
                text_color
            },
            weight: 400,
            italic: false,
        });

        // Chevron icon
        let chevron_bounds = cx.bounds(self.chevron_id);
        cx.draw_list.push(DrawCommand::Icon {
            name: if self.open {
                IconName::CaretUp
            } else {
                IconName::CaretDown
            },
            weight: IconWeight::Bold,
            bounds: chevron_bounds,
            color: self.muted_fg,
            size_px: CHEVRON_SIZE,
        });

        // Click handler to toggle
        if !self.disabled {
            if let Some(ref toggle) = self.on_toggle {
                cx.interactions.register_click(
                    trigger_bounds,
                    toggle.clone(),
                );
            }
        }
    }
}
