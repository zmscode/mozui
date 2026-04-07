use crate::color::Color;
use crate::shadow::Shadow;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum FontFamily {
    System,
    Monospace,
    Named(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    // Surface colors
    pub background: Color,
    pub surface: Color,
    pub surface_variant: Color,
    pub overlay: Color,

    // Brand colors
    pub primary: Color,
    pub primary_hover: Color,
    pub primary_active: Color,
    pub secondary: Color,
    pub accent: Color,

    // Semantic colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub text_disabled: Color,
    pub text_on_primary: Color,

    // Border colors
    pub border: Color,
    pub border_hover: Color,
    pub border_focus: Color,

    // Window chrome
    pub title_bar_background: Color,
    pub title_bar_text: Color,

    // Shadows
    pub shadow_sm: Shadow,
    pub shadow_md: Shadow,
    pub shadow_lg: Shadow,

    // Spacing
    pub spacing: Spacing,

    // Typography
    pub font_family: FontFamily,
    pub font_mono: FontFamily,
    pub font_size_xs: f32,
    pub font_size_sm: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    pub font_size_2xl: f32,

    // Radii
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub radius_xl: f32,

    // Animation
    pub transition_fast: Duration,
    pub transition_normal: Duration,
    pub transition_slow: Duration,
}

impl Theme {
    pub fn dark() -> Self {
        let shadow_color = Color::rgba(0, 0, 0, 0.5);
        Self {
            background: Color::hex("#1e1e2e"),
            surface: Color::hex("#313244"),
            surface_variant: Color::hex("#45475a"),
            overlay: Color::rgba(0, 0, 0, 0.6),

            primary: Color::hex("#cba6f7"),
            primary_hover: Color::hex("#b4befe"),
            primary_active: Color::hex("#9399f5"),
            secondary: Color::hex("#89b4fa"),
            accent: Color::hex("#f5c2e7"),

            success: Color::hex("#a6e3a1"),
            warning: Color::hex("#f9e2af"),
            error: Color::hex("#f38ba8"),
            info: Color::hex("#89dceb"),

            text_primary: Color::hex("#cdd6f4"),
            text_secondary: Color::hex("#a6adc8"),
            text_tertiary: Color::hex("#6c7086"),
            text_disabled: Color::hex("#585b70"),
            text_on_primary: Color::hex("#1e1e2e"),

            border: Color::hex("#45475a"),
            border_hover: Color::hex("#585b70"),
            border_focus: Color::hex("#cba6f7"),

            title_bar_background: Color::hex("#181825"),
            title_bar_text: Color::hex("#cdd6f4"),

            shadow_sm: Shadow::new(0.0, 1.0, 2.0, 0.0, shadow_color),
            shadow_md: Shadow::new(0.0, 4.0, 8.0, 0.0, shadow_color),
            shadow_lg: Shadow::new(0.0, 8.0, 24.0, 0.0, shadow_color),

            spacing: Spacing {
                xs: 4.0,
                sm: 8.0,
                md: 16.0,
                lg: 24.0,
                xl: 32.0,
                xxl: 48.0,
            },

            font_family: FontFamily::System,
            font_mono: FontFamily::Monospace,
            font_size_xs: 11.0,
            font_size_sm: 13.0,
            font_size_md: 15.0,
            font_size_lg: 18.0,
            font_size_xl: 24.0,
            font_size_2xl: 32.0,

            radius_sm: 4.0,
            radius_md: 6.0,
            radius_lg: 8.0,
            radius_xl: 12.0,

            transition_fast: Duration::from_millis(100),
            transition_normal: Duration::from_millis(200),
            transition_slow: Duration::from_millis(300),
        }
    }

    pub fn light() -> Self {
        let shadow_color = Color::rgba(0, 0, 0, 0.15);
        Self {
            background: Color::hex("#eff1f5"),
            surface: Color::hex("#ffffff"),
            surface_variant: Color::hex("#e6e9ef"),
            overlay: Color::rgba(0, 0, 0, 0.4),

            primary: Color::hex("#8839ef"),
            primary_hover: Color::hex("#7030d4"),
            primary_active: Color::hex("#5b27b0"),
            secondary: Color::hex("#1e66f5"),
            accent: Color::hex("#ea76cb"),

            success: Color::hex("#40a02b"),
            warning: Color::hex("#df8e1d"),
            error: Color::hex("#d20f39"),
            info: Color::hex("#04a5e5"),

            text_primary: Color::hex("#4c4f69"),
            text_secondary: Color::hex("#6c6f85"),
            text_tertiary: Color::hex("#9ca0b0"),
            text_disabled: Color::hex("#bcc0cc"),
            text_on_primary: Color::hex("#ffffff"),

            border: Color::hex("#ccd0da"),
            border_hover: Color::hex("#9ca0b0"),
            border_focus: Color::hex("#8839ef"),

            title_bar_background: Color::hex("#e6e9ef"),
            title_bar_text: Color::hex("#4c4f69"),

            shadow_sm: Shadow::new(0.0, 1.0, 2.0, 0.0, shadow_color),
            shadow_md: Shadow::new(0.0, 4.0, 8.0, 0.0, shadow_color),
            shadow_lg: Shadow::new(0.0, 8.0, 24.0, 0.0, shadow_color),

            spacing: Spacing {
                xs: 4.0,
                sm: 8.0,
                md: 16.0,
                lg: 24.0,
                xl: 32.0,
                xxl: 48.0,
            },

            font_family: FontFamily::System,
            font_mono: FontFamily::Monospace,
            font_size_xs: 11.0,
            font_size_sm: 13.0,
            font_size_md: 15.0,
            font_size_lg: 18.0,
            font_size_xl: 24.0,
            font_size_2xl: 32.0,

            radius_sm: 4.0,
            radius_md: 6.0,
            radius_lg: 8.0,
            radius_xl: 12.0,

            transition_fast: Duration::from_millis(100),
            transition_normal: Duration::from_millis(200),
            transition_slow: Duration::from_millis(300),
        }
    }
}
