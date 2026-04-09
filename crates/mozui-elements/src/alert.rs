use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
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
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        // Icon
        let icon_node = engine.new_leaf(Style {
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
        let title_m = mozui_text::measure_text(&self.title, &title_style, None, font_system);
        text_children.push(engine.new_leaf(Style {
            size: Size {
                width: length(title_m.width),
                height: length(title_m.height),
            },
            ..Default::default()
        }));

        if let Some(ref desc) = self.description {
            let desc_style = mozui_text::TextStyle {
                font_size: DESC_SIZE,
                ..Default::default()
            };
            let desc_m = mozui_text::measure_text(desc, &desc_style, None, font_system);
            text_children.push(engine.new_leaf(Style {
                size: Size {
                    width: length(desc_m.width),
                    height: length(desc_m.height),
                },
                ..Default::default()
            }));
        }

        let text_col = engine.new_with_children(
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

        let mut row_children = vec![icon_node, text_col];

        // Close button
        if self.dismissible {
            row_children.push(engine.new_leaf(Style {
                size: Size {
                    width: length(CLOSE_SIZE + 8.0),
                    height: length(CLOSE_SIZE + 8.0),
                },
                ..Default::default()
            }));
        }

        engine.new_with_children(
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
        // Container
        let container = layouts[*index];
        *index += 1;
        let bounds =
            mozui_style::Rect::new(container.x, container.y, container.width, container.height);

        draw_list.push(DrawCommand::Rect {
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
        let icon_l = layouts[*index];
        *index += 1;
        draw_list.push(DrawCommand::Icon {
            name: self.variant.icon(),
            weight: IconWeight::Fill,
            bounds: mozui_style::Rect::new(icon_l.x, icon_l.y, icon_l.width, icon_l.height),
            color: self.colors.icon_color,
            size_px: ICON_SIZE,
        });

        // Text column
        let _text_col = layouts[*index];
        *index += 1;

        // Title
        let title_l = layouts[*index];
        *index += 1;
        draw_list.push(DrawCommand::Text {
            text: self.title.clone(),
            bounds: mozui_style::Rect::new(title_l.x, title_l.y, title_l.width, title_l.height),
            font_size: TITLE_SIZE,
            color: self.colors.fg,
            weight: 600,
            italic: false,
        });

        // Description
        if let Some(ref desc) = self.description {
            let desc_l = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Text {
                text: desc.clone(),
                bounds: mozui_style::Rect::new(desc_l.x, desc_l.y, desc_l.width, desc_l.height),
                font_size: DESC_SIZE,
                color: self.muted_fg,
                weight: 400,
                italic: false,
            });
        }

        // Close button
        if self.dismissible {
            let close_l = layouts[*index];
            *index += 1;
            let close_bounds =
                mozui_style::Rect::new(close_l.x, close_l.y, close_l.width, close_l.height);
            let hovered = interactions.is_hovered(close_bounds);

            if hovered {
                draw_list.push(DrawCommand::Rect {
                    bounds: close_bounds,
                    background: Fill::Solid(self.colors.border),
                    corner_radii: Corners::uniform(self.corner_radius),
                    border: None,
                    shadow: None,
                });
            }

            draw_list.push(DrawCommand::Icon {
                name: IconName::X,
                weight: IconWeight::Bold,
                bounds: mozui_style::Rect::new(
                    close_l.x + 4.0,
                    close_l.y + 4.0,
                    CLOSE_SIZE,
                    CLOSE_SIZE,
                ),
                color: self.muted_fg,
                size_px: CLOSE_SIZE,
            });

            if let Some(ref on_dismiss) = self.on_dismiss {
                let ptr = on_dismiss.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                interactions
                    .register_click(close_bounds, Box::new(move |cx| unsafe { (*ptr)(cx) }));
                interactions.register_hover_region(close_bounds);
            }
        }
    }
}
