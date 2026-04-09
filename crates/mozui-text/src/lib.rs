#![forbid(unsafe_code)]

mod font_system;
pub mod shaping;

pub use font_system::FontSystem;
pub use shaping::{FontSlant, FontWeight, ShapedGlyph, ShapedRun, TextStyle};

use mozui_style::Size;

/// Measure text size without full layout — used for Taffy leaf sizing.
pub fn measure_text(
    text: &str,
    style: &TextStyle,
    max_width: Option<f32>,
    font_system: &FontSystem,
) -> Size {
    let run = shaping::shape_text(text, style, font_system);
    let line_height = style.font_size * style.line_height;

    if let Some(max_w) = max_width {
        // Simple word-wrap measurement
        let mut width: f32 = 0.0;
        let mut line_width: f32 = 0.0;
        let mut lines = 1u32;

        for glyph in &run.glyphs {
            line_width += glyph.advance;
            if line_width > max_w && line_width > glyph.advance {
                lines += 1;
                line_width = glyph.advance;
            }
        }
        width = width.max(line_width);

        Size::new(width.min(max_w), lines as f32 * line_height)
    } else {
        Size::new(run.width, line_height)
    }
}
