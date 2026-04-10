use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Rect, Theme};
use std::rc::Rc;
use taffy::prelude::*;

pub struct Link {
    label: String,
    href: Option<String>,
    color: Color,
    hover_color: Color,
    disabled: bool,
    font_size: f32,
    on_click: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
    layout_id: LayoutId,
    text_id: LayoutId,
}

pub fn link(label: impl Into<String>, theme: &Theme) -> Link {
    Link {
        label: label.into(),
        href: None,
        color: theme.link,
        hover_color: theme.link_hover,
        disabled: false,
        font_size: 13.0,
        on_click: None,
        layout_id: LayoutId::NONE,
        text_id: LayoutId::NONE,
    }
}

impl Link {
    pub fn href(mut self, url: impl Into<String>) -> Self {
        self.href = Some(url.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Element for Link {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Link",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let text_style = mozui_text::TextStyle {
            font_size: self.font_size,
            color: self.color,
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
                ..Default::default()
            },
            &[self.text_id],
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let text_bounds = cx.bounds(self.text_id);

        let hovered = !self.disabled && cx.interactions.is_hovered(bounds);
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let color = if hovered {
            self.hover_color.with_alpha(alpha)
        } else {
            self.color.with_alpha(alpha)
        };

        // Text
        cx.draw_list.push(DrawCommand::Text {
            text: self.label.clone(),
            bounds: text_bounds,
            font_size: self.font_size,
            color,
            weight: 400,
            italic: false,
        });

        // Underline (1px line at bottom of text)
        let underline_y = text_bounds.origin.y + text_bounds.size.height - 1.0;
        cx.draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(
                text_bounds.origin.x,
                underline_y,
                text_bounds.size.width,
                1.0,
            ),
            background: mozui_style::Fill::Solid(color),
            corner_radii: mozui_style::Corners::uniform(0.0),
            border: None,
            shadow: None, shadows: vec![],
        });

        // Click handler
        if !self.disabled {
            if let Some(ref handler) = self.on_click {
                cx.interactions
                    .register_click(bounds, handler.clone());
            } else if let Some(ref url) = self.href {
                let url = url.clone();
                cx.interactions.register_click(
                    bounds,
                    Rc::new(move |_cx| {
                        mozui_platform::open_url(&url);
                    }),
                );
            }
        }
    }
}
