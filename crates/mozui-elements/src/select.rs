use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Shadow, Theme};
use mozui_text::FontSystem;
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
    on_select: Option<Box<dyn Fn(&str, &mut dyn std::any::Any)>>,
    on_toggle: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
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
        self.on_select = Some(Box::new(handler));
        self
    }

    pub fn on_toggle(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_toggle = Some(Box::new(handler));
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
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let display_text = self.selected_label().unwrap_or(&self.placeholder);
        let text_style = mozui_text::TextStyle {
            font_size: self.font_size,
            color: self.fg,
            ..Default::default()
        };
        let measured = mozui_text::measure_text(display_text, &text_style, None, font_system);

        // Trigger button: [text] [chevron]
        let text_node = engine.new_leaf(Style {
            size: taffy::Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            flex_grow: 1.0,
            ..Default::default()
        });
        let chevron_node = engine.new_leaf(Style {
            size: taffy::Size {
                width: length(CHEVRON_SIZE),
                height: length(CHEVRON_SIZE),
            },
            ..Default::default()
        });
        let trigger = engine.new_with_children(
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
            &[text_node, chevron_node],
        );

        if !self.open {
            return trigger;
        }

        // Dropdown list
        let filtered = self.filtered_options();
        let mut item_nodes = Vec::new();

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
            let sm = mozui_text::measure_text(search_text, &search_style, None, font_system);
            item_nodes.push(engine.new_leaf(Style {
                size: taffy::Size {
                    width: percent(1.0),
                    height: length(sm.height + ITEM_PY * 2.0),
                },
                ..Default::default()
            }));
        }

        for option in &filtered {
            let style = mozui_text::TextStyle {
                font_size: self.font_size,
                color: self.fg,
                ..Default::default()
            };
            let m = mozui_text::measure_text(&option.label, &style, None, font_system);
            item_nodes.push(engine.new_leaf(Style {
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

        let dropdown = engine.new_with_children(
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
            &item_nodes,
        );

        // Wrapper: trigger + dropdown stacked vertically
        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                gap: taffy::Size {
                    width: zero(),
                    height: length(GAP),
                },
                ..Default::default()
            },
            &[trigger, dropdown],
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
        if !self.open {
            // Just the trigger
            self.paint_trigger(layouts, index, draw_list, interactions);
            return;
        }

        // Wrapper node
        let _wrapper = layouts[*index];
        *index += 1;

        // Trigger
        self.paint_trigger(layouts, index, draw_list, interactions);

        // Dropdown
        let dropdown_layout = layouts[*index];
        *index += 1;
        let dropdown_bounds = mozui_style::Rect::new(
            dropdown_layout.x,
            dropdown_layout.y,
            dropdown_layout.width,
            dropdown_layout.height,
        );

        // Dropdown background
        draw_list.push(DrawCommand::Rect {
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
            let search_layout = layouts[*index];
            *index += 1;
            let search_bounds = mozui_style::Rect::new(
                search_layout.x + ITEM_PX,
                search_layout.y + ITEM_PY,
                search_layout.width - ITEM_PX * 2.0,
                search_layout.height - ITEM_PY * 2.0,
            );

            // Search input background
            draw_list.push(DrawCommand::Rect {
                bounds: mozui_style::Rect::new(
                    search_layout.x,
                    search_layout.y,
                    search_layout.width,
                    search_layout.height,
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
            draw_list.push(DrawCommand::Text {
                text: text.to_string(),
                bounds: search_bounds,
                font_size: self.font_size,
                color,
                weight: 400,
                italic: false,
            });
        }

        // Options
        let filtered = self.filtered_options();
        for option in &filtered {
            let item_layout = layouts[*index];
            *index += 1;
            let item_bounds = mozui_style::Rect::new(
                item_layout.x,
                item_layout.y,
                item_layout.width,
                item_layout.height,
            );

            let is_selected = self
                .selected_value
                .as_ref()
                .map_or(false, |v| *v == option.value);
            let alpha = if option.disabled { 0.5 } else { 1.0 };
            let hovered = !option.disabled && interactions.is_hovered(item_bounds);

            // Hover / selected highlight
            if hovered || is_selected {
                let bg = if is_selected {
                    self.selected_bg
                } else {
                    self.hover_bg
                };
                draw_list.push(DrawCommand::Rect {
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
            draw_list.push(DrawCommand::Text {
                text: option.label.clone(),
                bounds: mozui_style::Rect::new(
                    item_layout.x + ITEM_PX,
                    item_layout.y + ITEM_PY,
                    item_layout.width - ITEM_PX * 2.0,
                    item_layout.height - ITEM_PY * 2.0,
                ),
                font_size: self.font_size,
                color: text_color,
                weight: if is_selected { 600 } else { 400 },
                italic: false,
            });

            // Check mark for selected item
            if is_selected {
                draw_list.push(DrawCommand::Icon {
                    name: IconName::Check,
                    weight: IconWeight::Bold,
                    bounds: mozui_style::Rect::new(
                        item_layout.x + item_layout.width - ITEM_PX - 14.0,
                        item_layout.y + (item_layout.height - 14.0) / 2.0,
                        14.0,
                        14.0,
                    ),
                    color: Color::WHITE,
                    size_px: 14.0,
                });
            }

            // Click handler
            if !option.disabled {
                if let Some(ref handler) = self.on_select {
                    let value = option.value.clone();
                    let handler_ptr =
                        handler.as_ref() as *const dyn Fn(&str, &mut dyn std::any::Any);
                    // Also dismiss on select
                    if let Some(ref toggle) = self.on_toggle {
                        let toggle_ptr =
                            toggle.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                        interactions.register_click(
                            item_bounds,
                            Box::new(move |cx| unsafe {
                                (*handler_ptr)(&value, cx);
                                (*toggle_ptr)(cx);
                            }),
                        );
                    } else {
                        interactions.register_click(
                            item_bounds,
                            Box::new(move |cx| unsafe {
                                (*handler_ptr)(&value, cx);
                            }),
                        );
                    }
                }
            }
        }

        // Escape to close
        if let Some(ref toggle) = self.on_toggle {
            let toggle_ptr = toggle.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            interactions.register_key_handler(Box::new(move |key, _mods, cx| {
                if key == mozui_events::Key::Escape {
                    unsafe { (*toggle_ptr)(cx) };
                }
            }));
        }
    }
}

impl Select {
    fn paint_trigger(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
    ) {
        let trigger_layout = layouts[*index];
        *index += 1;
        let trigger_bounds = mozui_style::Rect::new(
            trigger_layout.x,
            trigger_layout.y,
            trigger_layout.width,
            trigger_layout.height,
        );

        let hovered = !self.disabled && interactions.is_hovered(trigger_bounds);
        let active = !self.disabled && interactions.is_active(trigger_bounds);

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

        draw_list.push(DrawCommand::Rect {
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
        let text_layout = layouts[*index];
        *index += 1;
        let display_text = self.selected_label().unwrap_or(&self.placeholder);
        let text_color = if self.selected_value.is_some() {
            self.fg
        } else {
            self.muted_fg
        };
        draw_list.push(DrawCommand::Text {
            text: display_text.to_string(),
            bounds: mozui_style::Rect::new(
                text_layout.x,
                text_layout.y,
                text_layout.width,
                text_layout.height,
            ),
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
        let chevron_layout = layouts[*index];
        *index += 1;
        draw_list.push(DrawCommand::Icon {
            name: if self.open {
                IconName::CaretUp
            } else {
                IconName::CaretDown
            },
            weight: IconWeight::Bold,
            bounds: mozui_style::Rect::new(
                chevron_layout.x,
                chevron_layout.y,
                chevron_layout.width,
                chevron_layout.height,
            ),
            color: self.muted_fg,
            size_px: CHEVRON_SIZE,
        });

        // Click handler to toggle
        if !self.disabled {
            if let Some(ref toggle) = self.on_toggle {
                let toggle_ptr = toggle.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                interactions.register_click(
                    trigger_bounds,
                    Box::new(move |cx| unsafe { (*toggle_ptr)(cx) }),
                );
            }
        }
    }
}
