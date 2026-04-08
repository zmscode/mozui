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

/// Whether the theme is light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
}

impl ThemeMode {
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark)
    }
}

/// Complete theme definition with semantic color tokens for every component.
///
/// Modeled after gpui-component's ThemeColor with ~100 semantic tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub mode: ThemeMode,

    // ── Core surface colors ────────────────────────────────────────
    pub background: Color,
    pub foreground: Color,
    pub surface: Color,
    pub surface_variant: Color,
    pub overlay: Color,

    // ── Primary ────────────────────────────────────────────────────
    pub primary: Color,
    pub primary_hover: Color,
    pub primary_active: Color,
    pub primary_foreground: Color,

    // ── Secondary ──────────────────────────────────────────────────
    pub secondary: Color,
    pub secondary_hover: Color,
    pub secondary_active: Color,
    pub secondary_foreground: Color,

    // ── Accent ─────────────────────────────────────────────────────
    pub accent: Color,
    pub accent_foreground: Color,

    // ── Muted ──────────────────────────────────────────────────────
    pub muted: Color,
    pub muted_foreground: Color,

    // ── Semantic: danger ────────────────────────────────────────────
    pub danger: Color,
    pub danger_hover: Color,
    pub danger_active: Color,
    pub danger_foreground: Color,

    // ── Semantic: success ───────────────────────────────────────────
    pub success: Color,
    pub success_hover: Color,
    pub success_active: Color,
    pub success_foreground: Color,

    // ── Semantic: warning ───────────────────────────────────────────
    pub warning: Color,
    pub warning_hover: Color,
    pub warning_active: Color,
    pub warning_foreground: Color,

    // ── Semantic: info ──────────────────────────────────────────────
    pub info: Color,
    pub info_hover: Color,
    pub info_active: Color,
    pub info_foreground: Color,

    // ── Text colors ─────────────────────────────────────────────────
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub text_disabled: Color,
    pub text_on_primary: Color,

    // ── Border ──────────────────────────────────────────────────────
    pub border: Color,
    pub border_hover: Color,
    pub border_focus: Color,
    pub ring: Color,

    // ── Input ───────────────────────────────────────────────────────
    pub input_border: Color,
    pub input_background: Color,

    // ── Selection ───────────────────────────────────────────────────
    pub selection: Color,
    pub caret: Color,

    // ── Link ────────────────────────────────────────────────────────
    pub link: Color,
    pub link_hover: Color,
    pub link_active: Color,

    // ── Button primary ──────────────────────────────────────────────
    pub button_primary: Color,
    pub button_primary_hover: Color,
    pub button_primary_active: Color,
    pub button_primary_foreground: Color,

    // ── List ────────────────────────────────────────────────────────
    pub list: Color,
    pub list_hover: Color,
    pub list_active: Color,
    pub list_active_border: Color,
    pub list_even: Color,
    pub list_head: Color,

    // ── Table ───────────────────────────────────────────────────────
    pub table: Color,
    pub table_hover: Color,
    pub table_active: Color,
    pub table_active_border: Color,
    pub table_even: Color,
    pub table_head: Color,
    pub table_head_foreground: Color,
    pub table_row_border: Color,

    // ── Tab ─────────────────────────────────────────────────────────
    pub tab: Color,
    pub tab_active: Color,
    pub tab_active_foreground: Color,
    pub tab_foreground: Color,
    pub tab_bar: Color,

    // ── Popover ─────────────────────────────────────────────────────
    pub popover: Color,
    pub popover_foreground: Color,

    // ── Sidebar ─────────────────────────────────────────────────────
    pub sidebar: Color,
    pub sidebar_foreground: Color,
    pub sidebar_border: Color,
    pub sidebar_accent: Color,
    pub sidebar_accent_foreground: Color,

    // ── Title bar ───────────────────────────────────────────────────
    pub title_bar: Color,
    pub title_bar_border: Color,

    // ── Scrollbar ───────────────────────────────────────────────────
    pub scrollbar: Color,
    pub scrollbar_thumb: Color,
    pub scrollbar_thumb_hover: Color,

    // ── Accordion ───────────────────────────────────────────────────
    pub accordion: Color,
    pub accordion_hover: Color,

    // ── Switch ──────────────────────────────────────────────────────
    pub switch: Color,
    pub switch_thumb: Color,

    // ── Slider ──────────────────────────────────────────────────────
    pub slider_bar: Color,
    pub slider_thumb: Color,

    // ── Progress ────────────────────────────────────────────────────
    pub progress_bar: Color,

    // ── Skeleton ────────────────────────────────────────────────────
    pub skeleton: Color,

    // ── Drag/drop ───────────────────────────────────────────────────
    pub drag_border: Color,
    pub drop_target: Color,

    // ── Window ──────────────────────────────────────────────────────
    pub window_border: Color,

    // ── Charts ──────────────────────────────────────────────────────
    pub chart_1: Color,
    pub chart_2: Color,
    pub chart_3: Color,
    pub chart_4: Color,
    pub chart_5: Color,

    // ── Base terminal colors ────────────────────────────────────────
    pub base_red: Color,
    pub base_green: Color,
    pub base_blue: Color,
    pub base_yellow: Color,
    pub base_cyan: Color,
    pub base_magenta: Color,

    // ── Shadows ─────────────────────────────────────────────────────
    pub shadow_sm: Shadow,
    pub shadow_md: Shadow,
    pub shadow_lg: Shadow,
    pub shadow_enabled: bool,

    // ── Spacing ─────────────────────────────────────────────────────
    pub spacing: Spacing,

    // ── Typography ──────────────────────────────────────────────────
    pub font_family: FontFamily,
    pub font_mono: FontFamily,
    pub font_size_xs: f32,
    pub font_size_sm: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    pub font_size_2xl: f32,

    // ── Radii ───────────────────────────────────────────────────────
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub radius_xl: f32,

    // ── Animation ───────────────────────────────────────────────────
    pub transition_fast: Duration,
    pub transition_normal: Duration,
    pub transition_slow: Duration,
}

impl Theme {
    pub fn is_dark(&self) -> bool {
        self.mode.is_dark()
    }

    pub fn dark() -> Self {
        let shadow_color = Color::rgba(0, 0, 0, 0.5);
        Self {
            mode: ThemeMode::Dark,

            background: Color::hex("#1e1e2e"),
            foreground: Color::hex("#cdd6f4"),
            surface: Color::hex("#313244"),
            surface_variant: Color::hex("#45475a"),
            overlay: Color::rgba(0, 0, 0, 0.6),

            primary: Color::hex("#cba6f7"),
            primary_hover: Color::hex("#b4befe"),
            primary_active: Color::hex("#9399f5"),
            primary_foreground: Color::hex("#1e1e2e"),

            secondary: Color::hex("#45475a"),
            secondary_hover: Color::hex("#585b70"),
            secondary_active: Color::hex("#6c7086"),
            secondary_foreground: Color::hex("#cdd6f4"),

            accent: Color::hex("#f5c2e7"),
            accent_foreground: Color::hex("#1e1e2e"),

            muted: Color::hex("#313244"),
            muted_foreground: Color::hex("#a6adc8"),

            danger: Color::hex("#f38ba8"),
            danger_hover: Color::hex("#f17497"),
            danger_active: Color::hex("#e65d86"),
            danger_foreground: Color::hex("#1e1e2e"),

            success: Color::hex("#a6e3a1"),
            success_hover: Color::hex("#94d88e"),
            success_active: Color::hex("#82cd7b"),
            success_foreground: Color::hex("#1e1e2e"),

            warning: Color::hex("#f9e2af"),
            warning_hover: Color::hex("#f5d99a"),
            warning_active: Color::hex("#f0cf84"),
            warning_foreground: Color::hex("#1e1e2e"),

            info: Color::hex("#89dceb"),
            info_hover: Color::hex("#74d4e6"),
            info_active: Color::hex("#5fcce0"),
            info_foreground: Color::hex("#1e1e2e"),

            text_primary: Color::hex("#cdd6f4"),
            text_secondary: Color::hex("#a6adc8"),
            text_tertiary: Color::hex("#6c7086"),
            text_disabled: Color::hex("#585b70"),
            text_on_primary: Color::hex("#1e1e2e"),

            border: Color::hex("#45475a"),
            border_hover: Color::hex("#585b70"),
            border_focus: Color::hex("#cba6f7"),
            ring: Color::hex("#cba6f7"),

            input_border: Color::hex("#45475a"),
            input_background: Color::hex("#313244"),

            selection: Color::hex("#cba6f7").with_alpha(0.3),
            caret: Color::hex("#cdd6f4"),

            link: Color::hex("#89b4fa"),
            link_hover: Color::hex("#74a8f7"),
            link_active: Color::hex("#5f9cf4"),

            button_primary: Color::hex("#cba6f7"),
            button_primary_hover: Color::hex("#b4befe"),
            button_primary_active: Color::hex("#9399f5"),
            button_primary_foreground: Color::hex("#1e1e2e"),

            list: Color::hex("#1e1e2e"),
            list_hover: Color::hex("#313244"),
            list_active: Color::hex("#45475a"),
            list_active_border: Color::hex("#cba6f7"),
            list_even: Color::hex("#262637"),
            list_head: Color::hex("#181825"),

            table: Color::hex("#1e1e2e"),
            table_hover: Color::hex("#313244"),
            table_active: Color::hex("#45475a"),
            table_active_border: Color::hex("#cba6f7"),
            table_even: Color::hex("#262637"),
            table_head: Color::hex("#181825"),
            table_head_foreground: Color::hex("#a6adc8"),
            table_row_border: Color::hex("#313244"),

            tab: Color::hex("#1e1e2e"),
            tab_active: Color::hex("#313244"),
            tab_active_foreground: Color::hex("#cdd6f4"),
            tab_foreground: Color::hex("#6c7086"),
            tab_bar: Color::hex("#181825"),

            popover: Color::hex("#313244"),
            popover_foreground: Color::hex("#cdd6f4"),

            sidebar: Color::hex("#181825"),
            sidebar_foreground: Color::hex("#cdd6f4"),
            sidebar_border: Color::hex("#45475a"),
            sidebar_accent: Color::hex("#313244"),
            sidebar_accent_foreground: Color::hex("#cdd6f4"),

            title_bar: Color::hex("#181825"),
            title_bar_border: Color::hex("#45475a"),

            scrollbar: Color::TRANSPARENT,
            scrollbar_thumb: Color::hex("#585b70").with_alpha(0.5),
            scrollbar_thumb_hover: Color::hex("#585b70"),

            accordion: Color::hex("#313244"),
            accordion_hover: Color::hex("#45475a"),

            switch: Color::hex("#45475a"),
            switch_thumb: Color::hex("#cdd6f4"),

            slider_bar: Color::hex("#45475a"),
            slider_thumb: Color::hex("#cba6f7"),

            progress_bar: Color::hex("#cba6f7"),

            skeleton: Color::hex("#313244"),

            drag_border: Color::hex("#cba6f7"),
            drop_target: Color::hex("#cba6f7").with_alpha(0.15),

            window_border: Color::hex("#45475a"),

            chart_1: Color::hex("#cba6f7"),
            chart_2: Color::hex("#89b4fa"),
            chart_3: Color::hex("#a6e3a1"),
            chart_4: Color::hex("#f9e2af"),
            chart_5: Color::hex("#f38ba8"),

            base_red: Color::hex("#f38ba8"),
            base_green: Color::hex("#a6e3a1"),
            base_blue: Color::hex("#89b4fa"),
            base_yellow: Color::hex("#f9e2af"),
            base_cyan: Color::hex("#89dceb"),
            base_magenta: Color::hex("#f5c2e7"),

            shadow_sm: Shadow::new(0.0, 1.0, 2.0, 0.0, shadow_color),
            shadow_md: Shadow::new(0.0, 4.0, 8.0, 0.0, shadow_color),
            shadow_lg: Shadow::new(0.0, 8.0, 24.0, 0.0, shadow_color),
            shadow_enabled: true,

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
            mode: ThemeMode::Light,

            background: Color::hex("#eff1f5"),
            foreground: Color::hex("#4c4f69"),
            surface: Color::hex("#ffffff"),
            surface_variant: Color::hex("#e6e9ef"),
            overlay: Color::rgba(0, 0, 0, 0.4),

            primary: Color::hex("#8839ef"),
            primary_hover: Color::hex("#7030d4"),
            primary_active: Color::hex("#5b27b0"),
            primary_foreground: Color::hex("#ffffff"),

            secondary: Color::hex("#e6e9ef"),
            secondary_hover: Color::hex("#ccd0da"),
            secondary_active: Color::hex("#bcc0cc"),
            secondary_foreground: Color::hex("#4c4f69"),

            accent: Color::hex("#ea76cb"),
            accent_foreground: Color::hex("#ffffff"),

            muted: Color::hex("#e6e9ef"),
            muted_foreground: Color::hex("#6c6f85"),

            danger: Color::hex("#d20f39"),
            danger_hover: Color::hex("#b80d32"),
            danger_active: Color::hex("#9e0b2b"),
            danger_foreground: Color::hex("#ffffff"),

            success: Color::hex("#40a02b"),
            success_hover: Color::hex("#378c25"),
            success_active: Color::hex("#2e781f"),
            success_foreground: Color::hex("#ffffff"),

            warning: Color::hex("#df8e1d"),
            warning_hover: Color::hex("#c57e1a"),
            warning_active: Color::hex("#ab6e17"),
            warning_foreground: Color::hex("#ffffff"),

            info: Color::hex("#04a5e5"),
            info_hover: Color::hex("#0393cc"),
            info_active: Color::hex("#0381b3"),
            info_foreground: Color::hex("#ffffff"),

            text_primary: Color::hex("#4c4f69"),
            text_secondary: Color::hex("#6c6f85"),
            text_tertiary: Color::hex("#9ca0b0"),
            text_disabled: Color::hex("#bcc0cc"),
            text_on_primary: Color::hex("#ffffff"),

            border: Color::hex("#ccd0da"),
            border_hover: Color::hex("#9ca0b0"),
            border_focus: Color::hex("#8839ef"),
            ring: Color::hex("#8839ef"),

            input_border: Color::hex("#ccd0da"),
            input_background: Color::hex("#ffffff"),

            selection: Color::hex("#8839ef").with_alpha(0.2),
            caret: Color::hex("#4c4f69"),

            link: Color::hex("#1e66f5"),
            link_hover: Color::hex("#1a5ad6"),
            link_active: Color::hex("#164eb7"),

            button_primary: Color::hex("#8839ef"),
            button_primary_hover: Color::hex("#7030d4"),
            button_primary_active: Color::hex("#5b27b0"),
            button_primary_foreground: Color::hex("#ffffff"),

            list: Color::hex("#eff1f5"),
            list_hover: Color::hex("#e6e9ef"),
            list_active: Color::hex("#ccd0da"),
            list_active_border: Color::hex("#8839ef"),
            list_even: Color::hex("#e6e9ef"),
            list_head: Color::hex("#dce0e8"),

            table: Color::hex("#ffffff"),
            table_hover: Color::hex("#e6e9ef"),
            table_active: Color::hex("#ccd0da"),
            table_active_border: Color::hex("#8839ef"),
            table_even: Color::hex("#eff1f5"),
            table_head: Color::hex("#dce0e8"),
            table_head_foreground: Color::hex("#6c6f85"),
            table_row_border: Color::hex("#e6e9ef"),

            tab: Color::hex("#eff1f5"),
            tab_active: Color::hex("#ffffff"),
            tab_active_foreground: Color::hex("#4c4f69"),
            tab_foreground: Color::hex("#9ca0b0"),
            tab_bar: Color::hex("#e6e9ef"),

            popover: Color::hex("#ffffff"),
            popover_foreground: Color::hex("#4c4f69"),

            sidebar: Color::hex("#e6e9ef"),
            sidebar_foreground: Color::hex("#4c4f69"),
            sidebar_border: Color::hex("#ccd0da"),
            sidebar_accent: Color::hex("#ccd0da"),
            sidebar_accent_foreground: Color::hex("#4c4f69"),

            title_bar: Color::hex("#e6e9ef"),
            title_bar_border: Color::hex("#ccd0da"),

            scrollbar: Color::TRANSPARENT,
            scrollbar_thumb: Color::hex("#9ca0b0").with_alpha(0.5),
            scrollbar_thumb_hover: Color::hex("#9ca0b0"),

            accordion: Color::hex("#ffffff"),
            accordion_hover: Color::hex("#e6e9ef"),

            switch: Color::hex("#ccd0da"),
            switch_thumb: Color::hex("#ffffff"),

            slider_bar: Color::hex("#ccd0da"),
            slider_thumb: Color::hex("#8839ef"),

            progress_bar: Color::hex("#8839ef"),

            skeleton: Color::hex("#e6e9ef"),

            drag_border: Color::hex("#8839ef"),
            drop_target: Color::hex("#8839ef").with_alpha(0.1),

            window_border: Color::hex("#ccd0da"),

            chart_1: Color::hex("#8839ef"),
            chart_2: Color::hex("#1e66f5"),
            chart_3: Color::hex("#40a02b"),
            chart_4: Color::hex("#df8e1d"),
            chart_5: Color::hex("#d20f39"),

            base_red: Color::hex("#d20f39"),
            base_green: Color::hex("#40a02b"),
            base_blue: Color::hex("#1e66f5"),
            base_yellow: Color::hex("#df8e1d"),
            base_cyan: Color::hex("#04a5e5"),
            base_magenta: Color::hex("#ea76cb"),

            shadow_sm: Shadow::new(0.0, 1.0, 2.0, 0.0, shadow_color),
            shadow_md: Shadow::new(0.0, 4.0, 8.0, 0.0, shadow_color),
            shadow_lg: Shadow::new(0.0, 8.0, 24.0, 0.0, shadow_color),
            shadow_enabled: true,

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
