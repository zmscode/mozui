use crate::styled::{ComponentSize, Disableable, Selectable, Sizable};
use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

/// Resolved tab colors from theme.
#[derive(Debug, Clone, Copy)]
struct TabColors {
    bg: Color,
    active_bg: Color,
    fg: Color,
    active_fg: Color,
    indicator: Color,
    hover_bg: Color,
}

impl TabColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            bg: Color::TRANSPARENT,
            active_bg: Color::TRANSPARENT,
            fg: theme.tab_foreground,
            active_fg: theme.tab_active_foreground,
            indicator: theme.primary,
            hover_bg: theme.secondary,
        }
    }
}

pub struct Tab {
    label: String,
    icon: Option<IconName>,
    selected: bool,
    disabled: bool,
    size: ComponentSize,
    colors: TabColors,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn tab(label: impl Into<String>, theme: &Theme) -> Tab {
    Tab {
        label: label.into(),
        icon: None,
        selected: false,
        disabled: false,
        size: ComponentSize::Medium,
        colors: TabColors::from_theme(theme),
        on_click: None,
    }
}

impl Tab {
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn text_size(&self) -> f32 {
        self.size.button_text_size()
    }

    fn px(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 6.0,
            ComponentSize::Small => 8.0,
            ComponentSize::Medium => 12.0,
            ComponentSize::Large => 16.0,
            ComponentSize::Custom(_) => 12.0,
        }
    }

    fn py(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 4.0,
            ComponentSize::Small => 6.0,
            ComponentSize::Medium => 8.0,
            ComponentSize::Large => 10.0,
            ComponentSize::Custom(_) => 8.0,
        }
    }

    fn icon_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 12.0,
            ComponentSize::Small => 14.0,
            ComponentSize::Medium => 16.0,
            ComponentSize::Large => 18.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }
}

impl Sizable for Tab {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Tab {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Tab {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Element for Tab {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut children = Vec::new();

        if let Some(_icon) = self.icon {
            let icon_sz = self.icon_size();
            children.push(engine.new_leaf(Style {
                size: Size {
                    width: length(icon_sz),
                    height: length(icon_sz),
                },
                ..Default::default()
            }));
        }

        let text_style = mozui_text::TextStyle {
            font_size: self.text_size(),
            color: if self.selected {
                self.colors.active_fg
            } else {
                self.colors.fg
            },
            ..Default::default()
        };
        let measured = mozui_text::measure_text(&self.label, &text_style, None, font_system);
        children.push(engine.new_leaf(Style {
            size: Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            ..Default::default()
        }));

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                padding: taffy::Rect {
                    left: length(self.px()),
                    right: length(self.px()),
                    top: length(self.py()),
                    bottom: length(self.py()),
                },
                gap: Size {
                    width: length(6.0),
                    height: zero(),
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
        let layout = layouts[*index];
        *index += 1;

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let hovered = !self.disabled && !self.selected && interactions.is_hovered(bounds);

        // Background (hover only when not selected)
        let bg = if hovered {
            self.colors.hover_bg
        } else if self.selected {
            self.colors.active_bg
        } else {
            self.colors.bg
        };

        if bg.a > 0.0 {
            draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(bg.with_alpha(bg.a * alpha)),
                corner_radii: Corners::uniform(4.0),
                border: None,
            });
        }

        let fg = if self.selected {
            self.colors.active_fg
        } else {
            self.colors.fg
        };

        // Icon
        if let Some(icon_name) = self.icon {
            let icon_layout = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Icon {
                name: icon_name,
                weight: IconWeight::Regular,
                bounds: mozui_style::Rect::new(
                    icon_layout.x,
                    icon_layout.y,
                    icon_layout.width,
                    icon_layout.height,
                ),
                color: fg.with_alpha(alpha),
                size_px: self.icon_size(),
            });
        }

        // Label
        let text_layout = layouts[*index];
        *index += 1;
        draw_list.push(DrawCommand::Text {
            text: self.label.clone(),
            bounds: mozui_style::Rect::new(
                text_layout.x,
                text_layout.y,
                text_layout.width,
                text_layout.height,
            ),
            font_size: self.text_size(),
            color: fg.with_alpha(alpha),
            weight: if self.selected { 600 } else { 400 },
            italic: false,
        });

        // Bottom indicator line when selected
        if self.selected {
            let indicator_h = 2.0;
            draw_list.push(DrawCommand::Rect {
                bounds: mozui_style::Rect::new(
                    bounds.origin.x,
                    bounds.origin.y + bounds.size.height - indicator_h,
                    bounds.size.width,
                    indicator_h,
                ),
                background: Fill::Solid(self.colors.indicator.with_alpha(alpha)),
                corner_radii: Corners::uniform(1.0),
                border: None,
            });
        }

        // Click handler
        if !self.disabled {
            if let Some(ref handler) = self.on_click {
                let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                interactions
                    .register_click(bounds, Box::new(move |cx| unsafe { (*handler_ptr)(cx) }));
            }
        }
    }
}

// ── TabBar ────────────────────────────────────────────────────────

pub struct TabBar {
    tabs: Vec<Tab>,
    bar_color: Color,
    border_color: Color,
}

pub fn tab_bar(theme: &Theme) -> TabBar {
    TabBar {
        tabs: Vec::new(),
        bar_color: theme.tab_bar,
        border_color: theme.border,
    }
}

impl TabBar {
    pub fn child(mut self, tab: Tab) -> Self {
        self.tabs.push(tab);
        self
    }

    pub fn children(mut self, tabs: impl IntoIterator<Item = Tab>) -> Self {
        self.tabs.extend(tabs);
        self
    }
}

impl Element for TabBar {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let child_nodes: Vec<taffy::NodeId> = self
            .tabs
            .iter()
            .map(|t| t.layout(engine, font_system))
            .collect();

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Stretch),
                ..Default::default()
            },
            &child_nodes,
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
        let layout = layouts[*index];
        *index += 1;

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);

        // Bar background
        if self.bar_color.a > 0.0 {
            draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(self.bar_color),
                corner_radii: Corners::uniform(0.0),
                border: None,
            });
        }

        // Bottom border
        draw_list.push(DrawCommand::Rect {
            bounds: mozui_style::Rect::new(
                bounds.origin.x,
                bounds.origin.y + bounds.size.height - 1.0,
                bounds.size.width,
                1.0,
            ),
            background: Fill::Solid(self.border_color),
            corner_radii: Corners::uniform(0.0),
            border: None,
        });

        for tab in &self.tabs {
            tab.paint(layouts, index, draw_list, interactions, font_system);
        }
    }
}
