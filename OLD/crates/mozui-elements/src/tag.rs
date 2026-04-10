use crate::styled::{ComponentSize, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Theme};
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
    layout_id: LayoutId,
    text_id: LayoutId,
    label: String,
    colors: TagColors,
    outline: bool,
    pill: bool,
    size: ComponentSize,
    on_close: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn tag(label: impl Into<String>, theme: &Theme) -> Tag {
    Tag {
        layout_id: LayoutId::NONE,
        text_id: LayoutId::NONE,
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
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Tag",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let text_style = mozui_text::TextStyle {
            font_size: self.text_size(),
            color: self.colors.fg,
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
                ..Default::default()
            },
            &[self.text_id],
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let radius = self.radius();

        if self.outline {
            cx.draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(Color::TRANSPARENT),
                corner_radii: Corners::uniform(radius),
                border: Some(Border {
                    width: 1.0,
                    color: self.colors.border,
                }),
                shadow: None, shadows: vec![],
            });
        } else {
            cx.draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(self.colors.bg),
                corner_radii: Corners::uniform(radius),
                border: Some(Border {
                    width: 1.0,
                    color: self.colors.border,
                }),
                shadow: None, shadows: vec![],
            });
        }

        // Label
        let text_bounds = cx.bounds(self.text_id);
        cx.draw_list.push(DrawCommand::Text {
            text: self.label.clone(),
            bounds: text_bounds,
            font_size: self.text_size(),
            color: self.colors.fg,
            weight: 500,
            italic: false,
        });
    }
}
