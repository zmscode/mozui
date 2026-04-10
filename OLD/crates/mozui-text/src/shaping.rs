use crate::font_system::FontSystem;
use cosmic_text as ct;
use mozui_style::Color;

// Re-export for renderer access
pub use ct::{CacheKey, PhysicalGlyph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Regular,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSlant {
    Normal,
    Italic,
}

#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font_size: f32,
    pub weight: FontWeight,
    pub slant: FontSlant,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 15.0,
            weight: FontWeight::Regular,
            slant: FontSlant::Normal,
            line_height: 1.4,
            letter_spacing: 0.0,
            color: Color::WHITE,
        }
    }
}

/// A positioned glyph from cosmic-text shaping.
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub glyph_id: u32,
    /// Logical x offset (pre-scale)
    pub x_offset: f32,
    pub y_offset: f32,
    pub advance: f32,
    /// The underlying cosmic-text layout glyph data (for physical() calls).
    pub layout_glyph: ct::LayoutGlyph,
}

#[derive(Debug, Clone)]
pub struct ShapedRun {
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f32,
    /// Maximum ascent from the layout run (positive, above baseline).
    pub max_ascent: f32,
    /// Maximum descent from the layout run (positive, below baseline).
    pub max_descent: f32,
}

/// Shape text using cosmic-text's shaper (harfrust / HarfBuzz-compatible).
/// Returns positioned glyphs with proper complex script, BiDi, and ligature support.
pub fn shape_text(text: &str, style: &TextStyle, font_system: &FontSystem) -> ShapedRun {
    let ct_weight = match style.weight {
        FontWeight::Regular => ct::Weight::NORMAL,
        FontWeight::Bold => ct::Weight::BOLD,
    };
    let ct_style = match style.slant {
        FontSlant::Normal => ct::Style::Normal,
        FontSlant::Italic => ct::Style::Italic,
    };

    let line_height = style.font_size * style.line_height;
    let metrics = ct::Metrics::new(style.font_size, line_height);
    let attrs = ct::Attrs::new().weight(ct_weight).style(ct_style);

    let mut fs = font_system.borrow_mut();
    let mut buffer = ct::Buffer::new(&mut fs, metrics);
    buffer.set_text(&mut fs, text, &attrs, ct::Shaping::Advanced, None);

    let mut glyphs = Vec::new();
    let mut max_ascent: f32 = 0.0;
    let mut max_descent: f32 = 0.0;

    for run in buffer.layout_runs() {
        // Derive ascent/descent from LayoutRun's line geometry:
        // line_y = baseline offset from buffer top
        // line_top = top of line from buffer top
        // line_height = total height of the line
        let run_ascent = run.line_y - run.line_top;
        let run_descent = run.line_top + run.line_height - run.line_y;
        max_ascent = max_ascent.max(run_ascent);
        max_descent = max_descent.max(run_descent);
        for glyph in run.glyphs.iter() {
            glyphs.push(ShapedGlyph {
                glyph_id: glyph.glyph_id as u32,
                x_offset: glyph.x,
                y_offset: glyph.y,
                advance: glyph.w,
                layout_glyph: glyph.clone(),
            });
        }
    }

    let width = glyphs.last().map(|g| g.x_offset + g.advance).unwrap_or(0.0)
        + style.letter_spacing * glyphs.len().saturating_sub(1) as f32;
    ShapedRun {
        glyphs,
        width,
        max_ascent,
        max_descent,
    }
}
