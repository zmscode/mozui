use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::Color;
use mozui_text::FontSystem;
use taffy::prelude::*;

use crate::styled::{ComponentSize, Sizable};

/// How to highlight matched text ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelHighlightMode {
    /// Highlight from the start of the text.
    Prefix,
    /// Highlight exact byte ranges.
    Full,
}

/// A highlighted range within label text.
#[derive(Debug, Clone)]
pub struct LabelHighlight {
    pub range: std::ops::Range<usize>,
    pub color: Color,
}

pub struct Label {
    text: String,
    font_size: f32,
    color: Color,
    weight: u32,
    italic: bool,
    line_height: f32,
    masked: bool,
    single_line: bool,
    highlights: Vec<LabelHighlight>,
}

pub fn label(text: impl Into<String>) -> Label {
    Label {
        text: text.into(),
        font_size: 14.0,
        color: Color::WHITE,
        weight: 400,
        italic: false,
        line_height: 1.4,
        masked: false,
        single_line: true,
        highlights: Vec::new(),
    }
}

impl Label {
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

    /// Replace text with bullet characters (for passwords).
    pub fn masked(mut self) -> Self {
        self.masked = true;
        self
    }

    /// Allow multi-line wrapping.
    pub fn multi_line(mut self) -> Self {
        self.single_line = false;
        self
    }

    /// Add highlight ranges for search matching.
    pub fn highlights(mut self, highlights: Vec<LabelHighlight>) -> Self {
        self.highlights = highlights;
        self
    }

    /// Convenience: highlight with a default accent color.
    pub fn highlight_ranges(mut self, ranges: Vec<std::ops::Range<usize>>, color: Color) -> Self {
        self.highlights = ranges
            .into_iter()
            .map(|range| LabelHighlight { range, color })
            .collect();
        self
    }

    fn display_text(&self) -> String {
        if self.masked {
            "\u{2022}".repeat(self.text.len())
        } else {
            self.text.clone()
        }
    }

    fn font_size_for_component(size: ComponentSize) -> f32 {
        match size {
            ComponentSize::XSmall => 11.0,
            ComponentSize::Small => 12.0,
            ComponentSize::Medium => 14.0,
            ComponentSize::Large => 16.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }
}

impl Sizable for Label {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.font_size = Self::font_size_for_component(size.into());
        self
    }
}

impl Element for Label {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let display_text = self.display_text();

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

        let max_width = if self.single_line { None } else { None }; // TODO: constrained width
        let measured = mozui_text::measure_text(&display_text, &text_style, max_width, font_system);

        engine.new_leaf(Style {
            size: Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            min_size: Size {
                width: length(0.0),
                height: auto(),
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
        _font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);
        let display_text = self.display_text();

        // TODO: render highlight background rects behind matched ranges

        draw_list.push(DrawCommand::Text {
            text: display_text,
            bounds,
            font_size: self.font_size,
            color: self.color,
            weight: self.weight,
            italic: self.italic,
        });
    }
}
