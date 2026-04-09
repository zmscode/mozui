use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::{LayoutId, MeasureContext};
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Rect};
use taffy::prelude::*;

pub struct Text {
    layout_id: LayoutId,
    content: String,
    font_size: f32,
    color: Color,
    weight: u32,
    italic: bool,
    line_height: f32,
}

pub fn text(content: impl Into<String>) -> Text {
    Text {
        layout_id: LayoutId::NONE,
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

impl Text {
    fn text_style(&self) -> mozui_text::TextStyle {
        mozui_text::TextStyle {
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
        }
    }
}

impl Element for Text {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        let truncated: String = self.content.chars().take(50).collect();
        Some(mozui_devtools::ElementInfo {
            type_name: "Text",
            layout_id: self.layout_id,
            properties: vec![
                ("content", truncated),
                ("font_size", format!("{}", self.font_size)),
                ("color", format!("{:?}", self.color)),
            ],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Use a measured leaf so taffy calls our measure function with
        // the actual available width — text wraps automatically.
        self.layout_id = cx.new_measured_leaf(
            Style {
                min_size: Size {
                    width: length(0.0),
                    height: auto(),
                },
                ..Default::default()
            },
            MeasureContext::Text {
                text: self.content.clone(),
                style: self.text_style(),
            },
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        cx.collect_debug_info(self, bounds);
        cx.draw_list.push(DrawCommand::Text {
            text: self.content.clone(),
            bounds,
            font_size: self.font_size,
            color: self.color,
            weight: self.weight,
            italic: self.italic,
        });
    }
}
