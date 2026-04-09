use crate::styled::{ComponentSize, Disableable, Selectable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use taffy::prelude::*;

/// Visual variant of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Secondary,
    Danger,
    Ghost,
    Outline,
    Link,
    Text,
}

/// Resolved colors for a button variant.
#[derive(Debug, Clone, Copy)]
struct ButtonColors {
    bg: Color,
    hover_bg: Color,
    active_bg: Color,
    fg: Color,
    border: Option<Color>,
}

impl ButtonColors {
    fn from_variant(variant: ButtonVariant, theme: &Theme) -> Self {
        match variant {
            ButtonVariant::Default => Self {
                bg: theme.secondary,
                hover_bg: theme.secondary_hover,
                active_bg: theme.secondary_active,
                fg: theme.secondary_foreground,
                border: None,
            },
            ButtonVariant::Primary => Self {
                bg: theme.button_primary,
                hover_bg: theme.button_primary_hover,
                active_bg: theme.button_primary_active,
                fg: theme.button_primary_foreground,
                border: None,
            },
            ButtonVariant::Secondary => Self {
                bg: theme.secondary,
                hover_bg: theme.secondary_hover,
                active_bg: theme.secondary_active,
                fg: theme.secondary_foreground,
                border: None,
            },
            ButtonVariant::Danger => Self {
                bg: theme.danger,
                hover_bg: theme.danger_hover,
                active_bg: theme.danger_active,
                fg: theme.danger_foreground,
                border: None,
            },
            ButtonVariant::Ghost => Self {
                bg: Color::TRANSPARENT,
                hover_bg: theme.secondary,
                active_bg: theme.secondary_hover,
                fg: theme.foreground,
                border: None,
            },
            ButtonVariant::Outline => Self {
                bg: Color::TRANSPARENT,
                hover_bg: theme.secondary,
                active_bg: theme.secondary_hover,
                fg: theme.foreground,
                border: Some(theme.border),
            },
            ButtonVariant::Link => Self {
                bg: Color::TRANSPARENT,
                hover_bg: Color::TRANSPARENT,
                active_bg: Color::TRANSPARENT,
                fg: theme.link,
                border: None,
            },
            ButtonVariant::Text => Self {
                bg: Color::TRANSPARENT,
                hover_bg: theme.secondary,
                active_bg: theme.secondary_hover,
                fg: theme.foreground,
                border: None,
            },
        }
    }
}

pub struct Button {
    layout_id: LayoutId,
    child_ids: Vec<LayoutId>,
    label: Option<String>,
    icon: Option<IconName>,
    icon_right: Option<IconName>,
    variant: ButtonVariant,
    colors: ButtonColors,
    size: ComponentSize,
    disabled: bool,
    selected: bool,
    compact: bool,
    corner_radius: f32,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn button(label: impl Into<String>, theme: &Theme) -> Button {
    let variant = ButtonVariant::Default;
    Button {
        layout_id: LayoutId::NONE,
        child_ids: Vec::new(),
        label: Some(label.into()),
        icon: None,
        icon_right: None,
        variant,
        colors: ButtonColors::from_variant(variant, theme),
        size: ComponentSize::Medium,
        disabled: false,
        selected: false,
        compact: false,
        corner_radius: 6.0,
        on_click: None,
    }
}

/// Create an icon-only button.
pub fn icon_button(icon: IconName, theme: &Theme) -> Button {
    let variant = ButtonVariant::Ghost;
    Button {
        layout_id: LayoutId::NONE,
        child_ids: Vec::new(),
        label: None,
        icon: Some(icon),
        icon_right: None,
        variant,
        colors: ButtonColors::from_variant(variant, theme),
        size: ComponentSize::Medium,
        disabled: false,
        selected: false,
        compact: true,
        corner_radius: 6.0,
        on_click: None,
    }
}

impl Button {
    fn set_variant(&mut self, variant: ButtonVariant, theme: &Theme) {
        self.variant = variant;
        self.colors = ButtonColors::from_variant(variant, theme);
    }

    pub fn primary(mut self, theme: &Theme) -> Self {
        self.set_variant(ButtonVariant::Primary, theme);
        self
    }

    pub fn secondary(mut self, theme: &Theme) -> Self {
        self.set_variant(ButtonVariant::Secondary, theme);
        self
    }

    pub fn danger(mut self, theme: &Theme) -> Self {
        self.set_variant(ButtonVariant::Danger, theme);
        self
    }

    pub fn ghost(mut self, theme: &Theme) -> Self {
        self.set_variant(ButtonVariant::Ghost, theme);
        self
    }

    pub fn outline(mut self, theme: &Theme) -> Self {
        self.set_variant(ButtonVariant::Outline, theme);
        self
    }

    pub fn link(mut self, theme: &Theme) -> Self {
        self.set_variant(ButtonVariant::Link, theme);
        self
    }

    pub fn with_variant(mut self, variant: ButtonVariant, theme: &Theme) -> Self {
        self.set_variant(variant, theme);
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn icon_right(mut self, icon: IconName) -> Self {
        self.icon_right = Some(icon);
        self
    }

    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn px(&self) -> f32 {
        if self.compact {
            return self.size.input_py();
        }
        self.size.input_px()
    }

    fn py(&self) -> f32 {
        self.size.input_py()
    }

    fn text_size(&self) -> f32 {
        self.size.button_text_size()
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

    fn effective_fg(&self) -> Color {
        if self.disabled {
            self.colors.fg.with_alpha(0.5)
        } else {
            self.colors.fg
        }
    }
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Element for Button {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        let mut props = Vec::new();
        if let Some(ref label) = self.label {
            props.push(("label", label.clone()));
        }
        props.push(("variant", format!("{:?}", self.variant)));
        props.push(("disabled", format!("{}", self.disabled)));
        Some(mozui_devtools::ElementInfo {
            type_name: "Button",
            layout_id: self.layout_id,
            properties: props,
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let px = self.px();
        let py = self.py();
        let gap = 6.0_f32;

        self.child_ids.clear();

        // Left icon
        if self.icon.is_some() {
            let icon_sz = self.icon_size();
            let node = cx.new_leaf(Style {
                size: Size {
                    width: length(icon_sz),
                    height: length(icon_sz),
                },
                ..Default::default()
            });
            self.child_ids.push(node);
        }

        // Label
        if let Some(ref label_text) = self.label {
            let text_style = mozui_text::TextStyle {
                font_size: self.text_size(),
                color: self.effective_fg(),
                ..Default::default()
            };
            let measured = mozui_text::measure_text(label_text, &text_style, None, cx.font_system);
            let node = cx.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
                },
                ..Default::default()
            });
            self.child_ids.push(node);
        }

        // Right icon
        if self.icon_right.is_some() {
            let icon_sz = self.icon_size();
            let node = cx.new_leaf(Style {
                size: Size {
                    width: length(icon_sz),
                    height: length(icon_sz),
                },
                ..Default::default()
            });
            self.child_ids.push(node);
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                padding: taffy::Rect {
                    left: length(px),
                    right: length(px),
                    top: length(py),
                    bottom: length(py),
                },
                gap: Size {
                    width: length(gap),
                    height: zero(),
                },
                ..Default::default()
            },
            &self.child_ids,
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        cx.collect_debug_info(self, bounds);
        // Determine background color based on hover/active state
        let bg = if self.disabled {
            self.colors.bg.with_alpha(0.5)
        } else if cx.interactions.is_active(bounds) {
            self.colors.active_bg
        } else if cx.interactions.is_hovered(bounds) {
            self.colors.hover_bg
        } else {
            self.colors.bg
        };
        let fg = self.effective_fg();

        if bg.a > 0.0 {
            let border = self.colors.border.map(|c| Border {
                width: 1.0,
                color: if self.disabled { c.with_alpha(0.5) } else { c },
            });
            cx.draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(bg),
                corner_radii: Corners::uniform(self.corner_radius),
                border,
                shadow: None,
            });
        } else if let Some(border_color) = self.colors.border {
            // Outline-only: transparent bg but visible border
            cx.draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(Color::TRANSPARENT),
                corner_radii: Corners::uniform(self.corner_radius),
                border: Some(Border {
                    width: 1.0,
                    color: if self.disabled {
                        border_color.with_alpha(0.5)
                    } else {
                        border_color
                    },
                }),
                shadow: None,
            });
        }

        // Register click handler
        if !self.disabled {
            if let Some(ref handler) = self.on_click {
                let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                cx.interactions
                    .register_click(bounds, Box::new(move |cx| unsafe { (*handler_ptr)(cx) }));
            }
        }

        // Iterate child_ids in the same order they were pushed in layout:
        // icon, label, icon_right (only those that are Some)
        let mut idx = 0;

        // Left icon
        if let Some(icon_name) = self.icon {
            let child_bounds = cx.bounds(self.child_ids[idx]);
            idx += 1;
            cx.draw_list.push(DrawCommand::Icon {
                name: icon_name,
                weight: IconWeight::Regular,
                bounds: child_bounds,
                color: fg,
                size_px: self.icon_size(),
            });
        }

        // Label
        if let Some(ref label_text) = self.label {
            let child_bounds = cx.bounds(self.child_ids[idx]);
            idx += 1;
            cx.draw_list.push(DrawCommand::Text {
                text: label_text.clone(),
                bounds: child_bounds,
                font_size: self.text_size(),
                color: fg,
                weight: 500,
                italic: false,
            });
        }

        // Right icon
        if let Some(icon_name) = self.icon_right {
            let child_bounds = cx.bounds(self.child_ids[idx]);
            let _ = idx;
            cx.draw_list.push(DrawCommand::Icon {
                name: icon_name,
                weight: IconWeight::Regular,
                bounds: child_bounds,
                color: fg,
                size_px: self.icon_size(),
            });
        }
    }
}

// ── ButtonGroup ───────────────────────────────────────────────────

pub struct ButtonGroup {
    layout_id: LayoutId,
    child_ids: Vec<LayoutId>,
    buttons: Vec<Button>,
}

pub fn button_group() -> ButtonGroup {
    ButtonGroup {
        layout_id: LayoutId::NONE,
        child_ids: Vec::new(),
        buttons: Vec::new(),
    }
}

impl ButtonGroup {
    pub fn child(mut self, button: Button) -> Self {
        self.buttons.push(button);
        self
    }

    pub fn children(mut self, buttons: impl IntoIterator<Item = Button>) -> Self {
        self.buttons.extend(buttons);
        self
    }
}

impl Element for ButtonGroup {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "ButtonGroup",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.child_ids.clear();
        for button in &mut self.buttons {
            let id = button.layout(cx);
            self.child_ids.push(id);
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            &self.child_ids,
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        for (i, button) in self.buttons.iter_mut().enumerate() {
            let child_bounds = cx.bounds(self.child_ids[i]);
            button.paint(child_bounds, cx);
        }
    }
}
