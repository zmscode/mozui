#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Color = Color::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Color = Color::new(0.0, 0.0, 1.0, 1.0);
    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn hex(hex: &str) -> Self {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        let len = hex.len();

        let (r, g, b, a) = match len {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                (r, g, b, 255u8)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
                (r, g, b, a)
            }
            _ => (0, 0, 0, 255),
        };

        Self::rgb(r, g, b).with_alpha(a as f32 / 255.0)
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a,
        }
    }

    pub fn hsl(h: f32, s: f32, l: f32) -> Self {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let h_prime = h / 60.0;
        let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r, g, b) = if h_prime < 1.0 {
            (c, x, 0.0)
        } else if h_prime < 2.0 {
            (x, c, 0.0)
        } else if h_prime < 3.0 {
            (0.0, c, x)
        } else if h_prime < 4.0 {
            (0.0, x, c)
        } else if h_prime < 5.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Self {
            r: r + m,
            g: g + m,
            b: b + m,
            a: 1.0,
        }
    }

    pub fn with_alpha(self, alpha: f32) -> Self {
        Self { a: alpha, ..self }
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_wgpu(self) -> wgpu_color_stub::Color {
        wgpu_color_stub::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }

    /// Mix two colors in linear RGB space. `factor` 0.0 = self, 1.0 = other.
    pub fn mix(&self, other: &Color, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        let inv = 1.0 - f;
        Self {
            r: self.r * inv + other.r * f,
            g: self.g * inv + other.g * f,
            b: self.b * inv + other.b * f,
            a: self.a * inv + other.a * f,
        }
    }

    /// Blend this color over a base using standard alpha compositing.
    pub fn blend(&self, base: &Color) -> Self {
        let sa = self.a;
        let da = base.a * (1.0 - sa);
        let a = sa + da;
        if a == 0.0 {
            return Color::TRANSPARENT;
        }
        Self {
            r: (self.r * sa + base.r * da) / a,
            g: (self.g * sa + base.g * da) / a,
            b: (self.b * sa + base.b * da) / a,
            a,
        }
    }

    /// Convert to HSL. Returns (h: 0..360, s: 0..1, l: 0..1).
    pub fn to_hsl(&self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let l = (max + min) / 2.0;
        if (max - min).abs() < f32::EPSILON {
            return (0.0, 0.0, l);
        }
        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = if (max - self.r).abs() < f32::EPSILON {
            ((self.g - self.b) / d + if self.g < self.b { 6.0 } else { 0.0 }) * 60.0
        } else if (max - self.g).abs() < f32::EPSILON {
            ((self.b - self.r) / d + 2.0) * 60.0
        } else {
            ((self.r - self.g) / d + 4.0) * 60.0
        };
        (h, s, l)
    }

    /// Lighten by adjusting in HSL space. `amount` is added to lightness (0..1).
    pub fn lighten(&self, amount: f32) -> Self {
        let (h, s, l) = self.to_hsl();
        Color::hsl(h, s, (l + amount).min(1.0)).with_alpha(self.a)
    }

    /// Darken by adjusting in HSL space. `amount` is subtracted from lightness (0..1).
    pub fn darken(&self, amount: f32) -> Self {
        let (h, s, l) = self.to_hsl();
        Color::hsl(h, s, (l - amount).max(0.0)).with_alpha(self.a)
    }

    /// Return a hex string like "#rrggbb" or "#rrggbbaa".
    pub fn to_hex(&self) -> String {
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        if (self.a - 1.0).abs() < f32::EPSILON {
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        } else {
            let a = (self.a * 255.0).round() as u8;
            format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        }
    }
}

impl From<&str> for Color {
    fn from(hex: &str) -> Self {
        Color::hex(hex)
    }
}

// Minimal stub so mozui-style doesn't depend on wgpu
pub mod wgpu_color_stub {
    pub struct Color {
        pub r: f64,
        pub g: f64,
        pub b: f64,
        pub a: f64,
    }
}

// ---------------------------------------------------------------------------
// Tailwind-style color palette
// ---------------------------------------------------------------------------

/// Named color from the Tailwind palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorName {
    Slate,
    Gray,
    Zinc,
    Neutral,
    Stone,
    Red,
    Orange,
    Amber,
    Yellow,
    Lime,
    Green,
    Emerald,
    Teal,
    Cyan,
    Sky,
    Blue,
    Indigo,
    Violet,
    Purple,
    Fuchsia,
    Pink,
    Rose,
}

impl ColorName {
    /// All named colors (useful for iteration).
    pub const ALL: &[ColorName] = &[
        Self::Slate,
        Self::Gray,
        Self::Zinc,
        Self::Neutral,
        Self::Stone,
        Self::Red,
        Self::Orange,
        Self::Amber,
        Self::Yellow,
        Self::Lime,
        Self::Green,
        Self::Emerald,
        Self::Teal,
        Self::Cyan,
        Self::Sky,
        Self::Blue,
        Self::Indigo,
        Self::Violet,
        Self::Purple,
        Self::Fuchsia,
        Self::Pink,
        Self::Rose,
    ];

    /// Get a color at a specific scale (50, 100..900, 950).
    pub fn scale(&self, scale: u16) -> Color {
        palette(*self, scale)
    }
}

/// Look up a Tailwind palette color by name and scale.
/// Scales: 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950.
/// Returns the closest valid scale for out-of-range values.
pub fn palette(name: ColorName, scale: u16) -> Color {
    use ColorName::*;
    // Each row: [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950]
    let hex = match (name, scale) {
        (Slate, 50) => "#f8fafc",
        (Slate, 100) => "#f1f5f9",
        (Slate, 200) => "#e2e8f0",
        (Slate, 300) => "#cbd5e1",
        (Slate, 400) => "#94a3b8",
        (Slate, 500) => "#64748b",
        (Slate, 600) => "#475569",
        (Slate, 700) => "#334155",
        (Slate, 800) => "#1e293b",
        (Slate, 900) => "#0f172a",
        (Slate, 950) => "#020617",

        (Gray, 50) => "#f9fafb",
        (Gray, 100) => "#f3f4f6",
        (Gray, 200) => "#e5e7eb",
        (Gray, 300) => "#d1d5db",
        (Gray, 400) => "#9ca3af",
        (Gray, 500) => "#6b7280",
        (Gray, 600) => "#4b5563",
        (Gray, 700) => "#374151",
        (Gray, 800) => "#1f2937",
        (Gray, 900) => "#111827",
        (Gray, 950) => "#030712",

        (Zinc, 50) => "#fafafa",
        (Zinc, 100) => "#f4f4f5",
        (Zinc, 200) => "#e4e4e7",
        (Zinc, 300) => "#d4d4d8",
        (Zinc, 400) => "#a1a1aa",
        (Zinc, 500) => "#71717a",
        (Zinc, 600) => "#52525b",
        (Zinc, 700) => "#3f3f46",
        (Zinc, 800) => "#27272a",
        (Zinc, 900) => "#18181b",
        (Zinc, 950) => "#09090b",

        (Neutral, 50) => "#fafafa",
        (Neutral, 100) => "#f5f5f5",
        (Neutral, 200) => "#e5e5e5",
        (Neutral, 300) => "#d4d4d4",
        (Neutral, 400) => "#a3a3a3",
        (Neutral, 500) => "#737373",
        (Neutral, 600) => "#525252",
        (Neutral, 700) => "#404040",
        (Neutral, 800) => "#262626",
        (Neutral, 900) => "#171717",
        (Neutral, 950) => "#0a0a0a",

        (Stone, 50) => "#fafaf9",
        (Stone, 100) => "#f5f5f4",
        (Stone, 200) => "#e7e5e4",
        (Stone, 300) => "#d6d3d1",
        (Stone, 400) => "#a8a29e",
        (Stone, 500) => "#78716c",
        (Stone, 600) => "#57534e",
        (Stone, 700) => "#44403c",
        (Stone, 800) => "#292524",
        (Stone, 900) => "#1c1917",
        (Stone, 950) => "#0c0a09",

        (Red, 50) => "#fef2f2",
        (Red, 100) => "#fee2e2",
        (Red, 200) => "#fecaca",
        (Red, 300) => "#fca5a5",
        (Red, 400) => "#f87171",
        (Red, 500) => "#ef4444",
        (Red, 600) => "#dc2626",
        (Red, 700) => "#b91c1c",
        (Red, 800) => "#991b1b",
        (Red, 900) => "#7f1d1d",
        (Red, 950) => "#450a0a",

        (Orange, 50) => "#fff7ed",
        (Orange, 100) => "#ffedd5",
        (Orange, 200) => "#fed7aa",
        (Orange, 300) => "#fdba74",
        (Orange, 400) => "#fb923c",
        (Orange, 500) => "#f97316",
        (Orange, 600) => "#ea580c",
        (Orange, 700) => "#c2410c",
        (Orange, 800) => "#9a3412",
        (Orange, 900) => "#7c2d12",
        (Orange, 950) => "#431407",

        (Amber, 50) => "#fffbeb",
        (Amber, 100) => "#fef3c7",
        (Amber, 200) => "#fde68a",
        (Amber, 300) => "#fcd34d",
        (Amber, 400) => "#fbbf24",
        (Amber, 500) => "#f59e0b",
        (Amber, 600) => "#d97706",
        (Amber, 700) => "#b45309",
        (Amber, 800) => "#92400e",
        (Amber, 900) => "#78350f",
        (Amber, 950) => "#451a03",

        (Yellow, 50) => "#fefce8",
        (Yellow, 100) => "#fef9c3",
        (Yellow, 200) => "#fef08a",
        (Yellow, 300) => "#fde047",
        (Yellow, 400) => "#facc15",
        (Yellow, 500) => "#eab308",
        (Yellow, 600) => "#ca8a04",
        (Yellow, 700) => "#a16207",
        (Yellow, 800) => "#854d0e",
        (Yellow, 900) => "#713f12",
        (Yellow, 950) => "#422006",

        (Lime, 50) => "#f7fee7",
        (Lime, 100) => "#ecfccb",
        (Lime, 200) => "#d9f99d",
        (Lime, 300) => "#bef264",
        (Lime, 400) => "#a3e635",
        (Lime, 500) => "#84cc16",
        (Lime, 600) => "#65a30d",
        (Lime, 700) => "#4d7c0f",
        (Lime, 800) => "#3f6212",
        (Lime, 900) => "#365314",
        (Lime, 950) => "#1a2e05",

        (Green, 50) => "#f0fdf4",
        (Green, 100) => "#dcfce7",
        (Green, 200) => "#bbf7d0",
        (Green, 300) => "#86efac",
        (Green, 400) => "#4ade80",
        (Green, 500) => "#22c55e",
        (Green, 600) => "#16a34a",
        (Green, 700) => "#15803d",
        (Green, 800) => "#166534",
        (Green, 900) => "#14532d",
        (Green, 950) => "#052e16",

        (Emerald, 50) => "#ecfdf5",
        (Emerald, 100) => "#d1fae5",
        (Emerald, 200) => "#a7f3d0",
        (Emerald, 300) => "#6ee7b7",
        (Emerald, 400) => "#34d399",
        (Emerald, 500) => "#10b981",
        (Emerald, 600) => "#059669",
        (Emerald, 700) => "#047857",
        (Emerald, 800) => "#065f46",
        (Emerald, 900) => "#064e3b",
        (Emerald, 950) => "#022c22",

        (Teal, 50) => "#f0fdfa",
        (Teal, 100) => "#ccfbf1",
        (Teal, 200) => "#99f6e4",
        (Teal, 300) => "#5eead4",
        (Teal, 400) => "#2dd4bf",
        (Teal, 500) => "#14b8a6",
        (Teal, 600) => "#0d9488",
        (Teal, 700) => "#0f766e",
        (Teal, 800) => "#115e59",
        (Teal, 900) => "#134e4a",
        (Teal, 950) => "#042f2e",

        (Cyan, 50) => "#ecfeff",
        (Cyan, 100) => "#cffafe",
        (Cyan, 200) => "#a5f3fc",
        (Cyan, 300) => "#67e8f9",
        (Cyan, 400) => "#22d3ee",
        (Cyan, 500) => "#06b6d4",
        (Cyan, 600) => "#0891b2",
        (Cyan, 700) => "#0e7490",
        (Cyan, 800) => "#155e75",
        (Cyan, 900) => "#164e63",
        (Cyan, 950) => "#083344",

        (Sky, 50) => "#f0f9ff",
        (Sky, 100) => "#e0f2fe",
        (Sky, 200) => "#bae6fd",
        (Sky, 300) => "#7dd3fc",
        (Sky, 400) => "#38bdf8",
        (Sky, 500) => "#0ea5e9",
        (Sky, 600) => "#0284c7",
        (Sky, 700) => "#0369a1",
        (Sky, 800) => "#075985",
        (Sky, 900) => "#0c4a6e",
        (Sky, 950) => "#082f49",

        (Blue, 50) => "#eff6ff",
        (Blue, 100) => "#dbeafe",
        (Blue, 200) => "#bfdbfe",
        (Blue, 300) => "#93c5fd",
        (Blue, 400) => "#60a5fa",
        (Blue, 500) => "#3b82f6",
        (Blue, 600) => "#2563eb",
        (Blue, 700) => "#1d4ed8",
        (Blue, 800) => "#1e40af",
        (Blue, 900) => "#1e3a8a",
        (Blue, 950) => "#172554",

        (Indigo, 50) => "#eef2ff",
        (Indigo, 100) => "#e0e7ff",
        (Indigo, 200) => "#c7d2fe",
        (Indigo, 300) => "#a5b4fc",
        (Indigo, 400) => "#818cf8",
        (Indigo, 500) => "#6366f1",
        (Indigo, 600) => "#4f46e5",
        (Indigo, 700) => "#4338ca",
        (Indigo, 800) => "#3730a3",
        (Indigo, 900) => "#312e81",
        (Indigo, 950) => "#1e1b4b",

        (Violet, 50) => "#f5f3ff",
        (Violet, 100) => "#ede9fe",
        (Violet, 200) => "#ddd6fe",
        (Violet, 300) => "#c4b5fd",
        (Violet, 400) => "#a78bfa",
        (Violet, 500) => "#8b5cf6",
        (Violet, 600) => "#7c3aed",
        (Violet, 700) => "#6d28d9",
        (Violet, 800) => "#5b21b6",
        (Violet, 900) => "#4c1d95",
        (Violet, 950) => "#2e1065",

        (Purple, 50) => "#faf5ff",
        (Purple, 100) => "#f3e8ff",
        (Purple, 200) => "#e9d5ff",
        (Purple, 300) => "#d8b4fe",
        (Purple, 400) => "#c084fc",
        (Purple, 500) => "#a855f7",
        (Purple, 600) => "#9333ea",
        (Purple, 700) => "#7e22ce",
        (Purple, 800) => "#6b21a8",
        (Purple, 900) => "#581c87",
        (Purple, 950) => "#3b0764",

        (Fuchsia, 50) => "#fdf4ff",
        (Fuchsia, 100) => "#fae8ff",
        (Fuchsia, 200) => "#f5d0fe",
        (Fuchsia, 300) => "#f0abfc",
        (Fuchsia, 400) => "#e879f9",
        (Fuchsia, 500) => "#d946ef",
        (Fuchsia, 600) => "#c026d3",
        (Fuchsia, 700) => "#a21caf",
        (Fuchsia, 800) => "#86198f",
        (Fuchsia, 900) => "#701a75",
        (Fuchsia, 950) => "#4a044e",

        (Pink, 50) => "#fdf2f8",
        (Pink, 100) => "#fce7f3",
        (Pink, 200) => "#fbcfe8",
        (Pink, 300) => "#f9a8d4",
        (Pink, 400) => "#f472b6",
        (Pink, 500) => "#ec4899",
        (Pink, 600) => "#db2777",
        (Pink, 700) => "#be185d",
        (Pink, 800) => "#9d174d",
        (Pink, 900) => "#831843",
        (Pink, 950) => "#500724",

        (Rose, 50) => "#fff1f2",
        (Rose, 100) => "#ffe4e6",
        (Rose, 200) => "#fecdd3",
        (Rose, 300) => "#fda4af",
        (Rose, 400) => "#fb7185",
        (Rose, 500) => "#f43f5e",
        (Rose, 600) => "#e11d48",
        (Rose, 700) => "#be123c",
        (Rose, 800) => "#9f1239",
        (Rose, 900) => "#881337",
        (Rose, 950) => "#4c0519",

        // Fallback to 500 for unrecognized scales
        (_, _) => return palette(name, 500),
    };
    Color::hex(hex)
}

// Convenience functions for each color name
macro_rules! color_fn {
    ($name:ident, $variant:ident) => {
        pub fn $name(scale: u16) -> Color {
            palette(ColorName::$variant, scale)
        }
    };
}

color_fn!(slate, Slate);
color_fn!(gray, Gray);
color_fn!(zinc, Zinc);
color_fn!(neutral, Neutral);
color_fn!(stone, Stone);
color_fn!(red, Red);
color_fn!(orange, Orange);
color_fn!(amber, Amber);
color_fn!(yellow, Yellow);
color_fn!(lime, Lime);
color_fn!(green, Green);
color_fn!(emerald, Emerald);
color_fn!(teal, Teal);
color_fn!(cyan, Cyan);
color_fn!(sky, Sky);
color_fn!(blue, Blue);
color_fn!(indigo, Indigo);
color_fn!(violet, Violet);
color_fn!(purple, Purple);
color_fn!(fuchsia, Fuchsia);
color_fn!(pink, Pink);
color_fn!(rose, Rose);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parsing() {
        let c = Color::hex("#ff0000");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn hex_parsing_with_alpha() {
        let c = Color::hex("#ff000080");
        assert_eq!(c.r, 1.0);
        assert!((c.a - 0.502).abs() < 0.01);
    }

    #[test]
    fn lighten_darken() {
        let c = Color::rgb(100, 100, 100);
        let lighter = c.lighten(0.1);
        assert!(lighter.r > c.r);
        let darker = c.darken(0.1);
        assert!(darker.r < c.r);
    }
}
