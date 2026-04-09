use crate::styled::{ComponentSize, Disableable, Selectable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use taffy::prelude::*;

/// Visual style variant for the tab bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabBarVariant {
    /// Default: bottom border indicator on selected tab.
    #[default]
    Underline,
    /// Pill/capsule: selected tab gets a rounded filled background.
    Pill,
    /// Outlined: all tabs get a subtle border, selected tab gets filled bg.
    Outline,
    /// Segmented control: unified background with selected segment highlighted.
    Segmented,
}

/// Resolved tab colors from theme.
#[derive(Debug, Clone, Copy)]
struct TabColors {
    fg: Color,
    active_fg: Color,
    indicator: Color,
    hover_bg: Color,
    border: Color,
    surface: Color,
}

impl TabColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            fg: theme.tab_foreground,
            active_fg: theme.tab_active_foreground,
            indicator: theme.primary,
            hover_bg: theme.secondary,
            border: theme.border,
            surface: theme.surface,
        }
    }
}

pub struct Tab {
    layout_id: LayoutId,
    icon_id: LayoutId,
    text_id: LayoutId,

    label: String,
    icon: Option<IconName>,
    selected: bool,
    disabled: bool,
    size: ComponentSize,
    colors: TabColors,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    variant: TabBarVariant,
}

pub fn tab(label: impl Into<String>, theme: &Theme) -> Tab {
    Tab {
        layout_id: LayoutId::NONE,
        icon_id: LayoutId::NONE,
        text_id: LayoutId::NONE,
        label: label.into(),
        icon: None,
        selected: false,
        disabled: false,
        size: ComponentSize::Medium,
        colors: TabColors::from_theme(theme),
        on_click: None,
        variant: TabBarVariant::Underline,
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
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Tab",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let mut children = Vec::new();

        if let Some(_icon) = self.icon {
            let icon_sz = self.icon_size();
            self.icon_id = cx.new_leaf(Style {
                size: Size {
                    width: length(icon_sz),
                    height: length(icon_sz),
                },
                ..Default::default()
            });
            children.push(self.icon_id);
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
        let measured = mozui_text::measure_text(&self.label, &text_style, None, cx.font_system);
        self.text_id = cx.new_leaf(Style {
            size: Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            ..Default::default()
        });
        children.push(self.text_id);

        self.layout_id = cx.new_with_children(
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
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let hovered = !self.disabled && !self.selected && cx.interactions.is_hovered(bounds);

        // Variant-specific background and decoration
        match self.variant {
            TabBarVariant::Underline => {
                // Hover bg only
                if hovered {
                    cx.draw_list.push(DrawCommand::Rect {
                        bounds,
                        background: Fill::Solid(
                            self.colors
                                .hover_bg
                                .with_alpha(self.colors.hover_bg.a * alpha),
                        ),
                        corner_radii: Corners::uniform(4.0),
                        border: None,
                        shadow: None,
                    });
                }
            }
            TabBarVariant::Pill => {
                let bg = if self.selected {
                    self.colors.surface
                } else if hovered {
                    self.colors.hover_bg
                } else {
                    Color::TRANSPARENT
                };
                if bg.a > 0.0 {
                    cx.draw_list.push(DrawCommand::Rect {
                        bounds,
                        background: Fill::Solid(bg.with_alpha(bg.a * alpha)),
                        corner_radii: Corners::uniform(bounds.size.height / 2.0),
                        border: None,
                        shadow: None,
                    });
                }
            }
            TabBarVariant::Outline => {
                let bg = if self.selected {
                    self.colors.surface
                } else if hovered {
                    self.colors.hover_bg
                } else {
                    Color::TRANSPARENT
                };
                let border = if self.selected || hovered {
                    Some(mozui_renderer::Border {
                        width: 1.0,
                        color: self.colors.border.with_alpha(self.colors.border.a * alpha),
                    })
                } else {
                    None
                };
                cx.draw_list.push(DrawCommand::Rect {
                    bounds,
                    background: Fill::Solid(bg.with_alpha(bg.a * alpha)),
                    corner_radii: Corners::uniform(6.0),
                    border,
                    shadow: None,
                });
            }
            TabBarVariant::Segmented => {
                // Individual segment highlight (bar draws the container bg)
                if self.selected {
                    cx.draw_list.push(DrawCommand::Rect {
                        bounds,
                        background: Fill::Solid(self.colors.surface.with_alpha(alpha)),
                        corner_radii: Corners::uniform(5.0),
                        border: Some(mozui_renderer::Border {
                            width: 1.0,
                            color: self
                                .colors
                                .border
                                .with_alpha(self.colors.border.a * 0.5 * alpha),
                        }),
                        shadow: None,
                    });
                } else if hovered {
                    cx.draw_list.push(DrawCommand::Rect {
                        bounds,
                        background: Fill::Solid(
                            self.colors
                                .hover_bg
                                .with_alpha(self.colors.hover_bg.a * 0.5 * alpha),
                        ),
                        corner_radii: Corners::uniform(5.0),
                        border: None,
                        shadow: None,
                    });
                }
            }
        }

        let fg = if self.selected {
            self.colors.active_fg
        } else {
            self.colors.fg
        };

        // Icon
        if let Some(icon_name) = self.icon {
            let icon_bounds = cx.bounds(self.icon_id);
            cx.draw_list.push(DrawCommand::Icon {
                name: icon_name,
                weight: IconWeight::Regular,
                bounds: icon_bounds,
                color: fg.with_alpha(alpha),
                size_px: self.icon_size(),
            });
        }

        // Label
        let text_bounds = cx.bounds(self.text_id);
        cx.draw_list.push(DrawCommand::Text {
            text: self.label.clone(),
            bounds: text_bounds,
            font_size: self.text_size(),
            color: fg.with_alpha(alpha),
            weight: if self.selected { 600 } else { 400 },
            italic: false,
        });

        // Bottom indicator line — Underline variant only
        if self.variant == TabBarVariant::Underline && self.selected {
            let indicator_h = 2.0;
            cx.draw_list.push(DrawCommand::Rect {
                bounds: Rect::new(
                    bounds.origin.x,
                    bounds.origin.y + bounds.size.height - indicator_h,
                    bounds.size.width,
                    indicator_h,
                ),
                background: Fill::Solid(self.colors.indicator.with_alpha(alpha)),
                corner_radii: Corners::uniform(1.0),
                border: None,
                shadow: None,
            });
        }

        // Click handler
        if !self.disabled {
            if let Some(ref handler) = self.on_click {
                let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                cx.interactions
                    .register_click(bounds, Box::new(move |cx| unsafe { (*handler_ptr)(cx) }));
            }
        }
    }
}

// ── TabBar ────────────────────────────────────────────────────────

pub struct TabBar {
    layout_id: LayoutId,
    tab_ids: Vec<LayoutId>,

    tabs: Vec<Tab>,
    bar_color: Color,
    border_color: Color,
    variant: TabBarVariant,
    _surface_color: Color,
}

pub fn tab_bar(theme: &Theme) -> TabBar {
    TabBar {
        layout_id: LayoutId::NONE,
        tab_ids: Vec::new(),
        tabs: Vec::new(),
        bar_color: theme.tab_bar,
        border_color: theme.border,
        variant: TabBarVariant::Underline,
        _surface_color: theme.surface,
    }
}

impl TabBar {
    pub fn child(mut self, mut tab: Tab) -> Self {
        tab.variant = self.variant;
        self.tabs.push(tab);
        self
    }

    pub fn children(mut self, tabs: impl IntoIterator<Item = Tab>) -> Self {
        let variant = self.variant;
        self.tabs.extend(tabs.into_iter().map(|mut t| {
            t.variant = variant;
            t
        }));
        self
    }

    pub fn variant(mut self, variant: TabBarVariant) -> Self {
        self.variant = variant;
        // Update any already-added tabs
        for tab in &mut self.tabs {
            tab.variant = variant;
        }
        self
    }
}

impl Element for TabBar {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "TabBar",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.tab_ids = self
            .tabs
            .iter_mut()
            .map(|t| t.layout(cx))
            .collect();

        let (padding, gap, _radius) = match self.variant {
            TabBarVariant::Segmented => (
                taffy::Rect {
                    left: length(3.0),
                    right: length(3.0),
                    top: length(3.0),
                    bottom: length(3.0),
                },
                Size {
                    width: length(2.0),
                    height: zero(),
                },
                6.0,
            ),
            TabBarVariant::Pill => (
                taffy::Rect {
                    left: zero(),
                    right: zero(),
                    top: zero(),
                    bottom: zero(),
                },
                Size {
                    width: length(4.0),
                    height: zero(),
                },
                0.0,
            ),
            TabBarVariant::Outline => (
                taffy::Rect {
                    left: zero(),
                    right: zero(),
                    top: zero(),
                    bottom: zero(),
                },
                Size {
                    width: length(4.0),
                    height: zero(),
                },
                0.0,
            ),
            TabBarVariant::Underline => (
                taffy::Rect {
                    left: zero(),
                    right: zero(),
                    top: zero(),
                    bottom: zero(),
                },
                Size {
                    width: zero(),
                    height: zero(),
                },
                0.0,
            ),
        };

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Stretch),
                padding,
                gap,
                ..Default::default()
            },
            &self.tab_ids,
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        match self.variant {
            TabBarVariant::Segmented => {
                // Segmented: rounded container background with muted fill
                cx.draw_list.push(DrawCommand::Rect {
                    bounds,
                    background: Fill::Solid(self.bar_color),
                    corner_radii: Corners::uniform(8.0),
                    border: Some(mozui_renderer::Border {
                        width: 1.0,
                        color: self.border_color,
                    }),
                    shadow: None,
                });
            }
            TabBarVariant::Underline => {
                // Bar background
                if self.bar_color.a > 0.0 {
                    cx.draw_list.push(DrawCommand::Rect {
                        bounds,
                        background: Fill::Solid(self.bar_color),
                        corner_radii: Corners::ZERO,
                        border: None,
                        shadow: None,
                    });
                }
                // Bottom border
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: Rect::new(
                        bounds.origin.x,
                        bounds.origin.y + bounds.size.height - 1.0,
                        bounds.size.width,
                        1.0,
                    ),
                    background: Fill::Solid(self.border_color),
                    corner_radii: Corners::ZERO,
                    border: None,
                    shadow: None,
                });
            }
            TabBarVariant::Pill | TabBarVariant::Outline => {
                // No bar chrome — tabs style themselves
            }
        }

        for i in 0..self.tabs.len() {
            let tab_bounds = cx.bounds(self.tab_ids[i]);
            self.tabs[i].paint(tab_bounds, cx);
        }
    }
}
