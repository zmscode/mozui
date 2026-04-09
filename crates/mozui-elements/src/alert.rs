use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use taffy::prelude::*;

const PAD_X: f32 = 16.0;
const PAD_Y: f32 = 12.0;
const ICON_SIZE: f32 = 18.0;
const CLOSE_SIZE: f32 = 14.0;
const GAP: f32 = 12.0;
const TITLE_SIZE: f32 = 14.0;
const DESC_SIZE: f32 = 13.0;

/// The semantic variant of an alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertVariant {
    Info,
    Success,
    Warning,
    Danger,
}

impl AlertVariant {
    fn icon(&self) -> IconName {
        match self {
            Self::Info => IconName::Info,
            Self::Success => IconName::CheckCircle,
            Self::Warning => IconName::Warning,
            Self::Danger => IconName::XCircle,
        }
    }
}

/// Colors resolved from variant + theme.
struct AlertColors {
    bg: Color,
    fg: Color,
    icon_color: Color,
    border: Color,
}

fn resolve_colors(variant: AlertVariant, theme: &Theme) -> AlertColors {
    match variant {
        AlertVariant::Info => AlertColors {
            bg: theme.info.with_alpha(0.1),
            fg: theme.foreground,
            icon_color: theme.info,
            border: theme.info.with_alpha(0.3),
        },
        AlertVariant::Success => AlertColors {
            bg: theme.success.with_alpha(0.1),
            fg: theme.foreground,
            icon_color: theme.success,
            border: theme.success.with_alpha(0.3),
        },
        AlertVariant::Warning => AlertColors {
            bg: theme.warning.with_alpha(0.1),
            fg: theme.foreground,
            icon_color: theme.warning,
            border: theme.warning.with_alpha(0.3),
        },
        AlertVariant::Danger => AlertColors {
            bg: theme.danger.with_alpha(0.1),
            fg: theme.foreground,
            icon_color: theme.danger,
            border: theme.danger.with_alpha(0.3),
        },
    }
}

/// A dismissible alert/banner component.
pub struct Alert {
    variant: AlertVariant,
    title: String,
    description: Option<String>,
    dismissible: bool,
    on_dismiss: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    colors: AlertColors,
    muted_fg: Color,
    corner_radius: f32,
    // Layout IDs
    layout_id: LayoutId,
    icon_id: LayoutId,
    text_col_id: LayoutId,
    title_id: LayoutId,
    desc_id: LayoutId,
    close_id: LayoutId,
}

pub fn alert(variant: AlertVariant, title: impl Into<String>, theme: &Theme) -> Alert {
    Alert {
        variant,
        title: title.into(),
        description: None,
        dismissible: false,
        on_dismiss: None,
        colors: resolve_colors(variant, theme),
        muted_fg: theme.muted_foreground,
        corner_radius: theme.radius_md,
        layout_id: LayoutId::NONE,
        icon_id: LayoutId::NONE,
        text_col_id: LayoutId::NONE,
        title_id: LayoutId::NONE,
        desc_id: LayoutId::NONE,
        close_id: LayoutId::NONE,
    }
}

impl Alert {
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn dismissible(mut self, on_dismiss: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.dismissible = true;
        self.on_dismiss = Some(Box::new(on_dismiss));
        self
    }
}

impl Element for Alert {
    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Icon
        self.icon_id = cx.new_leaf(Style {
            size: Size {
                width: length(ICON_SIZE),
                height: length(ICON_SIZE),
            },
            ..Default::default()
        });

        // Text column (title + optional description)
        let mut text_children = Vec::new();
        let title_style = mozui_text::TextStyle {
            font_size: TITLE_SIZE,
            ..Default::default()
        };
        let title_m = mozui_text::measure_text(&self.title, &title_style, None, cx.font_system);
        self.title_id = cx.new_leaf(Style {
            size: Size {
                width: length(title_m.width),
                height: length(title_m.height),
            },
            ..Default::default()
        });
        text_children.push(self.title_id);

        if let Some(ref desc) = self.description {
            let desc_style = mozui_text::TextStyle {
                font_size: DESC_SIZE,
                ..Default::default()
            };
            let desc_m = mozui_text::measure_text(desc, &desc_style, None, cx.font_system);
            self.desc_id = cx.new_leaf(Style {
                size: Size {
                    width: length(desc_m.width),
                    height: length(desc_m.height),
                },
                ..Default::default()
            });
            text_children.push(self.desc_id);
        }

        self.text_col_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                gap: Size {
                    width: zero(),
                    height: length(4.0),
                },
                ..Default::default()
            },
            &text_children,
        );

        let mut row_children = vec![self.icon_id, self.text_col_id];

        // Close button
        if self.dismissible {
            self.close_id = cx.new_leaf(Style {
                size: Size {
                    width: length(CLOSE_SIZE + 8.0),
                    height: length(CLOSE_SIZE + 8.0),
                },
                ..Default::default()
            });
            row_children.push(self.close_id);
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::FlexStart),
                padding: taffy::Rect {
                    left: length(PAD_X),
                    right: length(PAD_X),
                    top: length(PAD_Y),
                    bottom: length(PAD_Y),
                },
                gap: Size {
                    width: length(GAP),
                    height: zero(),
                },
                ..Default::default()
            },
            &row_children,
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        cx.draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.colors.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: self.colors.border,
            }),
            shadow: None,
        });

        // Icon
        let icon_bounds = cx.bounds(self.icon_id);
        cx.draw_list.push(DrawCommand::Icon {
            name: self.variant.icon(),
            weight: IconWeight::Fill,
            bounds: icon_bounds,
            color: self.colors.icon_color,
            size_px: ICON_SIZE,
        });

        // Title
        let title_bounds = cx.bounds(self.title_id);
        cx.draw_list.push(DrawCommand::Text {
            text: self.title.clone(),
            bounds: title_bounds,
            font_size: TITLE_SIZE,
            color: self.colors.fg,
            weight: 600,
            italic: false,
        });

        // Description
        if let Some(ref desc) = self.description {
            let desc_bounds = cx.bounds(self.desc_id);
            cx.draw_list.push(DrawCommand::Text {
                text: desc.clone(),
                bounds: desc_bounds,
                font_size: DESC_SIZE,
                color: self.muted_fg,
                weight: 400,
                italic: false,
            });
        }

        // Close button
        if self.dismissible {
            let close_bounds = cx.bounds(self.close_id);
            let hovered = cx.interactions.is_hovered(close_bounds);

            if hovered {
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: close_bounds,
                    background: Fill::Solid(self.colors.border),
                    corner_radii: Corners::uniform(self.corner_radius),
                    border: None,
                    shadow: None,
                });
            }

            cx.draw_list.push(DrawCommand::Icon {
                name: IconName::X,
                weight: IconWeight::Bold,
                bounds: Rect::new(
                    close_bounds.origin.x + 4.0,
                    close_bounds.origin.y + 4.0,
                    CLOSE_SIZE,
                    CLOSE_SIZE,
                ),
                color: self.muted_fg,
                size_px: CLOSE_SIZE,
            });

            if let Some(ref on_dismiss) = self.on_dismiss {
                let ptr = on_dismiss.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                cx.interactions
                    .register_click(close_bounds, Box::new(move |cx| unsafe { (*ptr)(cx) }));
                cx.interactions.register_hover_region(close_bounds);
            }
        }
    }
}
