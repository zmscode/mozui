use crate::styled::{ComponentSize, Sizable};
use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagVariant {
    #[default]
    Default,
    Primary,
    Secondary,
    Danger,
    Success,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy)]
struct TagColors {
    bg: Color,
    fg: Color,
    border: Color,
}

impl TagColors {
    fn from_variant(variant: TagVariant, theme: &Theme) -> Self {
        match variant {
            TagVariant::Default => Self {
                bg: theme.secondary,
                fg: theme.secondary_foreground,
                border: theme.border,
            },
            TagVariant::Primary => Self {
                bg: theme.primary.with_alpha(0.15),
                fg: theme.primary,
                border: theme.primary.with_alpha(0.3),
            },
            TagVariant::Secondary => Self {
                bg: theme.secondary,
                fg: theme.secondary_foreground,
                border: theme.border,
            },
            TagVariant::Danger => Self {
                bg: theme.danger.with_alpha(0.15),
                fg: theme.danger,
                border: theme.danger.with_alpha(0.3),
            },
            TagVariant::Success => Self {
                bg: theme.success.with_alpha(0.15),
                fg: theme.success,
                border: theme.success.with_alpha(0.3),
            },
            TagVariant::Warning => Self {
                bg: theme.warning.with_alpha(0.15),
                fg: theme.warning,
                border: theme.warning.with_alpha(0.3),
            },
            TagVariant::Info => Self {
                bg: theme.info.with_alpha(0.15),
                fg: theme.info,
                border: theme.info.with_alpha(0.3),
            },
        }
    }
}

pub struct Tag {
    label: String,
    colors: TagColors,
    outline: bool,
    pill: bool,
    size: ComponentSize,
    on_close: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn tag(label: impl Into<String>, theme: &Theme) -> Tag {
    Tag {
        label: label.into(),
        colors: TagColors::from_variant(TagVariant::Default, theme),
        outline: false,
        pill: false,
        size: ComponentSize::Small,
        on_close: None,
    }
}

impl Tag {
    fn set_variant(&mut self, variant: TagVariant, theme: &Theme) {
        self.colors = TagColors::from_variant(variant, theme);
    }

    pub fn primary(mut self, theme: &Theme) -> Self {
        self.set_variant(TagVariant::Primary, theme);
        self
    }

    pub fn danger(mut self, theme: &Theme) -> Self {
        self.set_variant(TagVariant::Danger, theme);
        self
    }

    pub fn success(mut self, theme: &Theme) -> Self {
        self.set_variant(TagVariant::Success, theme);
        self
    }

    pub fn warning(mut self, theme: &Theme) -> Self {
        self.set_variant(TagVariant::Warning, theme);
        self
    }

    pub fn info(mut self, theme: &Theme) -> Self {
        self.set_variant(TagVariant::Info, theme);
        self
    }

    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    pub fn pill(mut self) -> Self {
        self.pill = true;
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }

    fn text_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 10.0,
            ComponentSize::Small => 11.0,
            ComponentSize::Medium => 12.0,
            ComponentSize::Large => 13.0,
            ComponentSize::Custom(_) => 11.0,
        }
    }

    fn px(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 4.0,
            ComponentSize::Small => 6.0,
            ComponentSize::Medium => 8.0,
            ComponentSize::Large => 10.0,
            ComponentSize::Custom(_) => 6.0,
        }
    }

    fn py(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 1.0,
            ComponentSize::Small => 2.0,
            ComponentSize::Medium => 3.0,
            ComponentSize::Large => 4.0,
            ComponentSize::Custom(_) => 2.0,
        }
    }

    fn radius(&self) -> f32 {
        if self.pill {
            100.0
        } else {
            match self.size {
                ComponentSize::XSmall => 3.0,
                ComponentSize::Small => 4.0,
                ComponentSize::Medium => 5.0,
                ComponentSize::Large => 6.0,
                ComponentSize::Custom(_) => 4.0,
            }
        }
    }
}

impl Sizable for Tag {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Element for Tag {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let text_style = mozui_text::TextStyle {
            font_size: self.text_size(),
            color: self.colors.fg,
            ..Default::default()
        };
        let measured = mozui_text::measure_text(&self.label, &text_style, None, font_system);
        let text_node = engine.new_leaf(Style {
            size: Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            ..Default::default()
        });

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
                ..Default::default()
            },
            &[text_node],
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        _interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);
        let radius = self.radius();

        if self.outline {
            draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(Color::TRANSPARENT),
                corner_radii: Corners::uniform(radius),
                border: Some(Border {
                    width: 1.0,
                    color: self.colors.border,
                }),
                    shadow: None,
                });
        } else {
            draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(self.colors.bg),
                corner_radii: Corners::uniform(radius),
                border: Some(Border {
                    width: 1.0,
                    color: self.colors.border,
                }),
                    shadow: None,
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
            color: self.colors.fg,
            weight: 500,
            italic: false,
        });
    }
}
