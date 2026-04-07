use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::{Properties, Style, Weight};
use font_kit::source::SystemSource;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontId(pub u32);

pub struct FontSystem {
    source: SystemSource,
    fonts: Vec<font_kit::font::Font>,
    cache: FxHashMap<FontKey, FontId>,
    default_font: Option<FontId>,
    default_mono: Option<FontId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FontKey {
    family: String,
    weight: u32,
    italic: bool,
}

impl FontSystem {
    pub fn new() -> Self {
        let mut sys = Self {
            source: SystemSource::new(),
            fonts: Vec::new(),
            cache: FxHashMap::default(),
            default_font: None,
            default_mono: None,
        };

        // Pre-load system default
        sys.default_font = sys.load_family_inner("system-ui", 400, false).ok();
        sys.default_mono = sys.load_family_inner("monospace", 400, false).ok();

        sys
    }

    pub fn default_font(&self) -> FontId {
        self.default_font.unwrap_or(FontId(0))
    }

    pub fn default_mono(&self) -> FontId {
        self.default_mono.unwrap_or(FontId(0))
    }

    pub fn load_family(&mut self, family: &str, weight: u32, italic: bool) -> FontId {
        self.load_family_inner(family, weight, italic)
            .unwrap_or_else(|_| self.default_font())
    }

    fn load_family_inner(&mut self, family: &str, weight: u32, italic: bool) -> Result<FontId, ()> {
        let key = FontKey {
            family: family.to_string(),
            weight,
            italic,
        };

        if let Some(&id) = self.cache.get(&key) {
            return Ok(id);
        }

        let family_name = match family {
            "system-ui" | "sans-serif" => FamilyName::SansSerif,
            "monospace" => FamilyName::Monospace,
            "serif" => FamilyName::Serif,
            name => FamilyName::Title(name.to_string()),
        };

        let mut props = Properties::new();
        props.weight = Weight(weight as f32);
        if italic {
            props.style = Style::Italic;
        }

        let handle = self
            .source
            .select_best_match(&[family_name, FamilyName::SansSerif], &props)
            .map_err(|_| ())?;

        let font = match handle {
            Handle::Path { path, font_index } => {
                font_kit::font::Font::from_path(&path, font_index).map_err(|_| ())?
            }
            Handle::Memory { bytes, font_index } => {
                font_kit::font::Font::from_bytes(bytes, font_index).map_err(|_| ())?
            }
        };

        let id = FontId(self.fonts.len() as u32);
        self.fonts.push(font);
        self.cache.insert(key, id);
        Ok(id)
    }

    pub fn get_font(&self, id: FontId) -> &font_kit::font::Font {
        &self.fonts[id.0 as usize]
    }

    pub fn font_metrics(&self, id: FontId, size: f32) -> FontMetrics {
        let font = self.get_font(id);
        let metrics = font.metrics();
        let scale = size / metrics.units_per_em as f32;
        FontMetrics {
            ascent: metrics.ascent * scale,
            descent: metrics.descent * scale,
            line_gap: metrics.line_gap * scale,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}
