use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::Color;
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct Text {
    content: String,
    font_size: f32,
    color: Color,
    weight: u32,
    italic: bool,
    line_height: f32,
}

pub fn text(content: impl Into<String>) -> Text {
    Text {
        content: content.into(),
        font_size: 15.0,
        color: Color::WHITE,
        weight: 400,
        italic: false,
        line_height: 1.4,
    }
}

impl Text {
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn bold(mut self) -> Self {
        self.weight = 700;
        self
    }

    pub fn weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn line_height(mut self, lh: f32) -> Self {
        self.line_height = lh;
        self
    }
}

impl Element for Text {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let text_style = mozui_text::TextStyle {
            font_size: self.font_size,
            weight: if self.weight >= 700 {
                mozui_text::FontWeight::Bold
            } else {
                mozui_text::FontWeight::Regular
            },
            slant: if self.italic {
                mozui_text::FontSlant::Italic
            } else {
                mozui_text::FontSlant::Normal
            },
            line_height: self.line_height,
            color: self.color,
            ..Default::default()
        };

        let measured_size = mozui_text::measure_text(&self.content, &text_style, None, font_system);

        engine.new_leaf(Style {
            size: Size {
                width: length(measured_size.width),
                height: length(measured_size.height),
            },
            ..Default::default()
        })
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        _interactions: &mut InteractionMap,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);

        draw_list.push(DrawCommand::Text {
            text: self.content.clone(),
            bounds,
            font_size: self.font_size,
            color: self.color,
            weight: self.weight,
            italic: self.italic,
        });
    }
}
