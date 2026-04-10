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
        let shadow_color = Color::rgba(0, 0, 0, 0.55);
        Self {
            mode: ThemeMode::Dark,

            // Colors sourced from 0x96f-zed-theme (theme.json)
            background: Color::hex("#262427"),          // editor/terminal background
            foreground: Color::hex("#FCFCFC"),           // text
            surface: Color::hex("#1C1B1C"),              // element.background, panels
            surface_variant: Color::hex("#322E32"),      // elevated_surface.background
            overlay: Color::rgba(0, 0, 0, 0.6),

            primary: Color::hex("#49CAE4"),              // text.accent
            primary_hover: Color::hex("#64D2E8"),        // terminal.ansi.bright_blue
            primary_active: Color::hex("#3AB8D0"),
            primary_foreground: Color::hex("#1C1B1C"),

            secondary: Color::hex("#353035"),            // element.selected
            secondary_hover: Color::hex("#454045"),      // element.hover
            secondary_active: Color::hex("#555055"),     // ghost_element.hover
            secondary_foreground: Color::hex("#FCFCFC"),

            accent: Color::hex("#A093E2"),               // terminal.ansi.magenta
            accent_foreground: Color::hex("#1C1B1C"),

            muted: Color::hex("#353035"),                // element.selected
            muted_foreground: Color::hex("#A6A6A5"),     // text.placeholder

            danger: Color::hex("#FF7272"),               // terminal.ansi.red
            danger_hover: Color::hex("#FF8787"),         // terminal.ansi.bright_red
            danger_active: Color::hex("#E25555"),
            danger_foreground: Color::hex("#1C1B1C"),

            success: Color::hex("#BCDF59"),              // terminal.ansi.green
            success_hover: Color::hex("#C6E472"),        // terminal.ansi.bright_green
            success_active: Color::hex("#A5C84E"),
            success_foreground: Color::hex("#1C1B1C"),

            warning: Color::hex("#FC9D6F"),              // warning
            warning_hover: Color::hex("#FFB088"),
            warning_active: Color::hex("#E08A5E"),
            warning_foreground: Color::hex("#1C1B1C"),

            info: Color::hex("#49CAE4"),                 // info
            info_hover: Color::hex("#64D2E8"),
            info_active: Color::hex("#3AB8D0"),
            info_foreground: Color::hex("#1C1B1C"),

            text_primary: Color::hex("#FCFCFC"),         // text
            text_secondary: Color::hex("#BCBCBC"),       // text.muted
            text_tertiary: Color::hex("#A6A6A5"),        // text.placeholder
            text_disabled: Color::hex("#8B8B8B"),        // text.disabled
            text_on_primary: Color::hex("#1C1B1C"),

            border: Color::hex("#151015"),               // border
            border_hover: Color::hex("#909090"),         // panel.focused_border
            border_focus: Color::hex("#49CAE4"),
            ring: Color::hex("#49CAE4"),

            input_border: Color::hex("#151015"),         // border
            input_background: Color::hex("#1C1B1C"),     // element.background

            selection: Color::hex("#B5A5B5").with_alpha(0.25), // players.selection
            caret: Color::hex("#FFBD3E"),                // players.cursor

            link: Color::hex("#49CAE4"),
            link_hover: Color::hex("#64D2E8"),
            link_active: Color::hex("#3AB8D0"),

            button_primary: Color::hex("#49CAE4"),
            button_primary_hover: Color::hex("#64D2E8"),
            button_primary_active: Color::hex("#3AB8D0"),
            button_primary_foreground: Color::hex("#1C1B1C"),

            list: Color::hex("#1C1B1C"),                 // panel.background
            list_hover: Color::hex("#454045"),           // element.hover
            list_active: Color::hex("#353035"),          // element.selected
            list_active_border: Color::hex("#49CAE4"),
            list_even: Color::hex("#252025"),            // ghost_element.background
            list_head: Color::hex("#1C1B1C"),

            table: Color::hex("#1C1B1C"),
            table_hover: Color::hex("#454045"),
            table_active: Color::hex("#353035"),
            table_active_border: Color::hex("#49CAE4"),
            table_even: Color::hex("#252025"),
            table_head: Color::hex("#1C1B1C"),
            table_head_foreground: Color::hex("#A6A6A5"),
            table_row_border: Color::hex("#151015"),

            tab: Color::hex("#1C1B1C"),                  // tab.inactive_background
            tab_active: Color::hex("#252425"),           // tab.active_background
            tab_active_foreground: Color::hex("#FCFCFC"),
            tab_foreground: Color::hex("#A6A6A5"),
            tab_bar: Color::hex("#1C1B1C"),              // tab_bar.background

            popover: Color::hex("#322E32"),              // elevated_surface.background
            popover_foreground: Color::hex("#FCFCFC"),

            sidebar: Color::hex("#1C1B1C"),              // panel.background
            sidebar_foreground: Color::hex("#FCFCFC"),
            sidebar_border: Color::hex("#151015"),
            sidebar_accent: Color::hex("#353035"),       // element.selected
            sidebar_accent_foreground: Color::hex("#FCFCFC"),

            title_bar: Color::hex("#1C1B1C"),            // title_bar.background
            title_bar_border: Color::hex("#151015"),

            scrollbar: Color::TRANSPARENT,
            scrollbar_thumb: Color::hex("#686368").with_alpha(0.44), // scrollbar.thumb.background
            scrollbar_thumb_hover: Color::hex("#686368").with_alpha(0.56), // scrollbar.thumb.hover_background

            accordion: Color::hex("#1C1B1C"),
            accordion_hover: Color::hex("#322E32"),

            switch: Color::hex("#353035"),               // element.selected
            switch_thumb: Color::hex("#FFFFFF"),

            slider_bar: Color::hex("#353035"),
            slider_thumb: Color::hex("#49CAE4"),

            progress_bar: Color::hex("#49CAE4"),

            skeleton: Color::hex("#322E32"),

            drag_border: Color::hex("#49CAE4"),
            drop_target: Color::hex("#383438").with_alpha(0.87), // drop_target.background

            window_border: Color::hex("#151015"),

            chart_1: Color::hex("#49CAE4"),              // blue
            chart_2: Color::hex("#A093E2"),              // magenta
            chart_3: Color::hex("#BCDF59"),              // green
            chart_4: Color::hex("#FFCA58"),              // yellow
            chart_5: Color::hex("#FF7272"),              // red

            base_red: Color::hex("#FF7272"),
            base_green: Color::hex("#BCDF59"),
            base_blue: Color::hex("#49CAE4"),
            base_yellow: Color::hex("#FFCA58"),
            base_cyan: Color::hex("#AEE8F4"),
            base_magenta: Color::hex("#A093E2"),

            shadow_sm: Shadow::new(0.0, 1.0, 3.0, 0.0, shadow_color),
            shadow_md: Shadow::new(0.0, 4.0, 12.0, 0.0, shadow_color),
            shadow_lg: Shadow::new(0.0, 10.0, 30.0, 0.0, shadow_color),
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

    /// Light variant derived from the 0x96f palette (inverted surface hierarchy).
    pub fn light() -> Self {
        let shadow_color = Color::rgba(0, 0, 0, 0.12);
        Self {
            mode: ThemeMode::Light,

            background: Color::hex("#F5F4F6"),
            foreground: Color::hex("#262427"),
            surface: Color::hex("#FFFFFF"),
            surface_variant: Color::hex("#EAE8EC"),
            overlay: Color::rgba(0, 0, 0, 0.4),

            primary: Color::hex("#2AAFC8"),              // darkened accent for light bg
            primary_hover: Color::hex("#49CAE4"),
            primary_active: Color::hex("#2299B0"),
            primary_foreground: Color::hex("#FFFFFF"),

            secondary: Color::hex("#E8E6EA"),
            secondary_hover: Color::hex("#DAD8DC"),
            secondary_active: Color::hex("#CCC9CF"),
            secondary_foreground: Color::hex("#262427"),

            accent: Color::hex("#8A7BD0"),               // darkened magenta
            accent_foreground: Color::hex("#FFFFFF"),

            muted: Color::hex("#E8E6EA"),
            muted_foreground: Color::hex("#757075"),

            danger: Color::hex("#E04545"),
            danger_hover: Color::hex("#FF7272"),
            danger_active: Color::hex("#C83A3A"),
            danger_foreground: Color::hex("#FFFFFF"),

            success: Color::hex("#7AAA30"),
            success_hover: Color::hex("#BCDF59"),
            success_active: Color::hex("#6A9528"),
            success_foreground: Color::hex("#FFFFFF"),

            warning: Color::hex("#D87A4A"),
            warning_hover: Color::hex("#FC9D6F"),
            warning_active: Color::hex("#C06A3E"),
            warning_foreground: Color::hex("#FFFFFF"),

            info: Color::hex("#2AAFC8"),
            info_hover: Color::hex("#49CAE4"),
            info_active: Color::hex("#2299B0"),
            info_foreground: Color::hex("#FFFFFF"),

            text_primary: Color::hex("#262427"),
            text_secondary: Color::hex("#555055"),
            text_tertiary: Color::hex("#757075"),
            text_disabled: Color::hex("#A6A6A5"),
            text_on_primary: Color::hex("#FFFFFF"),

            border: Color::hex("#D5D2D8"),
            border_hover: Color::hex("#A6A6A5"),
            border_focus: Color::hex("#2AAFC8"),
            ring: Color::hex("#2AAFC8"),

            input_border: Color::hex("#D5D2D8"),
            input_background: Color::hex("#FFFFFF"),

            selection: Color::hex("#49CAE4").with_alpha(0.2),
            caret: Color::hex("#262427"),

            link: Color::hex("#2AAFC8"),
            link_hover: Color::hex("#49CAE4"),
            link_active: Color::hex("#2299B0"),

            button_primary: Color::hex("#2AAFC8"),
            button_primary_hover: Color::hex("#49CAE4"),
            button_primary_active: Color::hex("#2299B0"),
            button_primary_foreground: Color::hex("#FFFFFF"),

            list: Color::hex("#F5F4F6"),
            list_hover: Color::hex("#EAE8EC"),
            list_active: Color::hex("#DAD8DC"),
            list_active_border: Color::hex("#2AAFC8"),
            list_even: Color::hex("#EFEEEF"),
            list_head: Color::hex("#E8E6EA"),

            table: Color::hex("#FFFFFF"),
            table_hover: Color::hex("#EAE8EC"),
            table_active: Color::hex("#DAD8DC"),
            table_active_border: Color::hex("#2AAFC8"),
            table_even: Color::hex("#F5F4F6"),
            table_head: Color::hex("#E8E6EA"),
            table_head_foreground: Color::hex("#757075"),
            table_row_border: Color::hex("#E8E6EA"),

            tab: Color::hex("#F5F4F6"),
            tab_active: Color::hex("#FFFFFF"),
            tab_active_foreground: Color::hex("#262427"),
            tab_foreground: Color::hex("#A6A6A5"),
            tab_bar: Color::hex("#EAE8EC"),

            popover: Color::hex("#FFFFFF"),
            popover_foreground: Color::hex("#262427"),

            sidebar: Color::hex("#EAE8EC"),
            sidebar_foreground: Color::hex("#262427"),
            sidebar_border: Color::hex("#D5D2D8"),
            sidebar_accent: Color::hex("#DAD8DC"),
            sidebar_accent_foreground: Color::hex("#262427"),

            title_bar: Color::hex("#EAE8EC"),
            title_bar_border: Color::hex("#D5D2D8"),

            scrollbar: Color::TRANSPARENT,
            scrollbar_thumb: Color::hex("#A6A6A5").with_alpha(0.4),
            scrollbar_thumb_hover: Color::hex("#A6A6A5").with_alpha(0.7),

            accordion: Color::hex("#FFFFFF"),
            accordion_hover: Color::hex("#EAE8EC"),

            switch: Color::hex("#D5D2D8"),
            switch_thumb: Color::hex("#FFFFFF"),

            slider_bar: Color::hex("#D5D2D8"),
            slider_thumb: Color::hex("#2AAFC8"),

            progress_bar: Color::hex("#2AAFC8"),

            skeleton: Color::hex("#EAE8EC"),

            drag_border: Color::hex("#2AAFC8"),
            drop_target: Color::hex("#49CAE4").with_alpha(0.1),

            window_border: Color::hex("#D5D2D8"),

            chart_1: Color::hex("#2AAFC8"),
            chart_2: Color::hex("#8A7BD0"),
            chart_3: Color::hex("#7AAA30"),
            chart_4: Color::hex("#D87A4A"),
            chart_5: Color::hex("#E04545"),

            base_red: Color::hex("#E04545"),
            base_green: Color::hex("#7AAA30"),
            base_blue: Color::hex("#2AAFC8"),
            base_yellow: Color::hex("#D8A030"),
            base_cyan: Color::hex("#49CAE4"),
            base_magenta: Color::hex("#8A7BD0"),

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

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
