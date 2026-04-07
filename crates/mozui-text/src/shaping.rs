use crate::font_system::{FontId, FontSystem};
use mozui_style::Color;

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

#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub glyph_id: u32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub advance: f32,
    pub font_id: FontId,
}

#[derive(Debug, Clone)]
pub struct ShapedRun {
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f32,
}

/// Simple left-to-right shaping using font-kit glyph advances.
pub fn shape_text(text: &str, style: &TextStyle, font_system: &FontSystem) -> ShapedRun {
    let _weight_val = match style.weight {
        FontWeight::Regular => 400,
        FontWeight::Bold => 700,
    };
    let _italic = style.slant == FontSlant::Italic;

    // For now, always use the default font (we'll resolve by family later)
    let font_id = font_system.default_font();
    let font = font_system.get_font(font_id);
    let metrics = font.metrics();
    let scale = style.font_size / metrics.units_per_em as f32;

    let mut glyphs = Vec::with_capacity(text.len());
    let mut x_offset = 0.0f32;

    for ch in text.chars() {
        let glyph_id = font.glyph_for_char(ch).unwrap_or(0);
        let advance = font
            .advance(glyph_id)
            .map(|a| a.x() * scale)
            .unwrap_or(style.font_size * 0.5);

        glyphs.push(ShapedGlyph {
            glyph_id,
            x_offset,
            y_offset: 0.0,
            advance,
            font_id,
        });

        x_offset += advance + style.letter_spacing;
    }

    ShapedRun {
        width: x_offset,
        glyphs,
    }
}
