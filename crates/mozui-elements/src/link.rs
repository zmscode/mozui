use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct Link {
    label: String,
    color: Color,
    hover_color: Color,
    disabled: bool,
    font_size: f32,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn link(label: impl Into<String>, theme: &Theme) -> Link {
    Link {
        label: label.into(),
        color: theme.link,
        hover_color: theme.link_hover,
        disabled: false,
        font_size: 13.0,
        on_click: None,
    }
}

impl Link {
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
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Element for Link {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let text_style = mozui_text::TextStyle {
            font_size: self.font_size,
            color: self.color,
            ..Default::default()
        };
        let measured = mozui_text::measure_text(&self.label, &text_style, None, font_system);

        // Container with space for underline
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
        interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);

        let text_layout = layouts[*index];
        *index += 1;
        let text_bounds = mozui_style::Rect::new(
            text_layout.x,
            text_layout.y,
            text_layout.width,
            text_layout.height,
        );

        let hovered = !self.disabled && interactions.is_hovered(bounds);
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let color = if hovered {
            self.hover_color.with_alpha(alpha)
        } else {
            self.color.with_alpha(alpha)
        };

        // Text
        draw_list.push(DrawCommand::Text {
            text: self.label.clone(),
            bounds: text_bounds,
            font_size: self.font_size,
            color,
            weight: 400,
            italic: false,
        });

        // Underline (1px line at bottom of text)
        let underline_y = text_bounds.origin.y + text_bounds.size.height - 1.0;
        draw_list.push(DrawCommand::Rect {
            bounds: mozui_style::Rect::new(
                text_bounds.origin.x,
                underline_y,
                text_bounds.size.width,
                1.0,
            ),
            background: mozui_style::Fill::Solid(color),
            corner_radii: mozui_style::Corners::uniform(0.0),
            border: None,
        });

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
