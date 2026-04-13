mod support;

use mozui::prelude::*;
use mozui::{
    Context, GlassEffectStyle, SymbolScale, SymbolWeight, VisualEffectActiveState,
    VisualEffectBlending, VisualEffectMaterial, Window, div, hsla, linear_color_stop,
    linear_gradient, native_glass_effect, native_image_view, native_visual_effect, px, size,
};
use mozui_components::StyledExt as _;
use support::run_transparent_rooted_example;

/// A pure-native effects example.
///
/// Demonstrates the three new leaf elements added in the native-controls
/// migration:
///   - `native_visual_effect`  → NSVisualEffectView (blur / vibrancy)
///   - `native_glass_effect`   → NSGlassEffectView on macOS 26+, falls back to
///                               NSVisualEffectView on older systems
///   - `native_image_view`     → NSImageView with an SF Symbol and optional tint
///
/// Run with:
///   cargo run -p mozui-examples --example native_effects
fn main() {
    run_transparent_rooted_example(
        "Native Effects",
        mozui_components::theme::ThemeMode::Dark,
        size(px(960.0), px(780.0)),
        |window, cx| cx.new(|cx| NativeEffectsExample::new(window, cx)),
    );
}

struct NativeEffectsExample;

impl NativeEffectsExample {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for NativeEffectsExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("native-effects-root")
            .size_full()
            // Gradient background so the blur effects have something to show through.
            .bg(linear_gradient(
                135.,
                linear_color_stop(hsla(0.72, 0.45, 0.25, 1.0), 0.0),
                linear_color_stop(hsla(0.62, 0.40, 0.15, 1.0), 1.0),
            ))
            .flex()
            .flex_col()
            .gap(px(20.0))
            .p(px(28.0))
            .pt(px(52.0))
            // ---------------------------------------------------------------
            // Header
            // ---------------------------------------------------------------
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(hsla(0.0, 0.0, 1.0, 0.96))
                            .font_bold()
                            .text_sm()
                            .child("Native effects"),
                    )
                    .child(
                        div()
                            .text_color(hsla(0.0, 0.0, 1.0, 0.62))
                            .text_xs()
                            .child("NSVisualEffectView, NSGlassEffectView, and NSImageView (SF Symbols) rendered as native AppKit subviews inside the mozui window."),
                    ),
            )
            // ---------------------------------------------------------------
            // NSVisualEffectView — material variants
            // ---------------------------------------------------------------
            .child(
                section_header(
                    "native_visual_effect",
                    "NSVisualEffectView — each card is a different blur material.",
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(blur_card(
                        "native-effects-ve-sidebar",
                        VisualEffectMaterial::Sidebar,
                        VisualEffectBlending::BehindWindow,
                        VisualEffectActiveState::Active,
                        false,
                        "Sidebar",
                    ))
                    .child(blur_card(
                        "native-effects-ve-popover",
                        VisualEffectMaterial::Popover,
                        VisualEffectBlending::BehindWindow,
                        VisualEffectActiveState::Active,
                        false,
                        "Popover",
                    ))
                    .child(blur_card(
                        "native-effects-ve-menu",
                        VisualEffectMaterial::Menu,
                        VisualEffectBlending::BehindWindow,
                        VisualEffectActiveState::Active,
                        false,
                        "Menu",
                    ))
                    .child(blur_card(
                        "native-effects-ve-hud",
                        VisualEffectMaterial::HudWindow,
                        VisualEffectBlending::BehindWindow,
                        VisualEffectActiveState::Active,
                        false,
                        "HUD",
                    ))
                    .child(blur_card(
                        "native-effects-ve-selection",
                        VisualEffectMaterial::Selection,
                        VisualEffectBlending::WithinWindow,
                        VisualEffectActiveState::Active,
                        true,
                        "Selection (emphasized)",
                    )),
            )
            // ---------------------------------------------------------------
            // NSGlassEffectView (macOS 26+) / fallback
            // ---------------------------------------------------------------
            .child(
                section_header(
                    "native_glass_effect",
                    "NSGlassEffectView on macOS 26+, NSVisualEffectView on older systems.",
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(glass_card(
                        "native-effects-glass-regular",
                        GlassEffectStyle::Regular,
                        None,
                        None,
                        "Regular",
                    ))
                    .child(glass_card(
                        "native-effects-glass-clear",
                        GlassEffectStyle::Clear,
                        None,
                        None,
                        "Clear",
                    ))
                    .child(glass_card(
                        "native-effects-glass-rounded",
                        GlassEffectStyle::Regular,
                        Some(24.0),
                        None,
                        "Regular r=24",
                    ))
                    .child(glass_card(
                        "native-effects-glass-tinted",
                        GlassEffectStyle::Regular,
                        Some(16.0),
                        Some((0.4, 0.2, 0.8, 0.3)),
                        "Regular + tint",
                    )),
            )
            // ---------------------------------------------------------------
            // NSImageView — SF Symbols with weight / scale variations
            // ---------------------------------------------------------------
            .child(
                section_header(
                    "native_image_view",
                    "NSImageView rendering SF Symbols — weight and scale variations.",
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(16.0))
                    .child(symbol_row(
                        "native-effects-sym-folder",
                        "folder.fill",
                        SymbolWeight::Regular,
                        SymbolScale::Medium,
                        24.0,
                        None,
                        "folder.fill\nregular / medium",
                    ))
                    .child(symbol_row(
                        "native-effects-sym-bolt",
                        "bolt.fill",
                        SymbolWeight::Bold,
                        SymbolScale::Large,
                        32.0,
                        Some((1.0, 0.8, 0.2, 1.0)),
                        "bolt.fill\nbold / large / tinted",
                    ))
                    .child(symbol_row(
                        "native-effects-sym-wand",
                        "wand.and.stars",
                        SymbolWeight::Thin,
                        SymbolScale::Medium,
                        28.0,
                        Some((0.5, 0.9, 1.0, 1.0)),
                        "wand.and.stars\nthin / medium / tinted",
                    ))
                    .child(symbol_row(
                        "native-effects-sym-cloud",
                        "cloud.fill",
                        SymbolWeight::Semibold,
                        SymbolScale::Small,
                        20.0,
                        None,
                        "cloud.fill\nsemibold / small",
                    ))
                    .child(symbol_row(
                        "native-effects-sym-heart",
                        "heart.fill",
                        SymbolWeight::Black,
                        SymbolScale::Large,
                        36.0,
                        Some((1.0, 0.3, 0.4, 1.0)),
                        "heart.fill\nblack / large / tinted",
                    )),
            )
    }
}

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

fn section_header(title: &'static str, subtitle: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .text_xs()
                .font_bold()
                .text_color(hsla(0.0, 0.0, 1.0, 0.88))
                .child(title),
        )
        .child(
            div()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 1.0, 0.56))
                .child(subtitle),
        )
}

/// A card whose background is a `native_visual_effect` view.
///
/// Note: native NSViews render as AppKit subviews on top of the Metal surface,
/// so labels must live outside the effect container as siblings, not overlaid inside it.
fn blur_card(
    id: &'static str,
    material: VisualEffectMaterial,
    blending: VisualEffectBlending,
    active_state: VisualEffectActiveState,
    emphasized: bool,
    label: &'static str,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.0))
        .child(
            // The effect fills this container.
            div()
                .w(px(152.0))
                .h(px(80.0))
                .rounded(px(14.0))
                .overflow_hidden()
                .child(
                    native_visual_effect(id)
                        .material(material)
                        .blending(blending)
                        .active_state(active_state)
                        .is_emphasized(emphasized),
                ),
        )
        .child(
            div()
                .text_xs()
                .font_bold()
                .text_color(hsla(0.0, 0.0, 1.0, 0.82))
                .child(label),
        )
}

/// A card whose background is a `native_glass_effect` view.
///
/// Note: native NSViews render above the Metal surface, so labels must live
/// outside the effect container as siblings, not overlaid inside it.
fn glass_card(
    id: &'static str,
    style: GlassEffectStyle,
    corner_radius: Option<f64>,
    tint: Option<(f64, f64, f64, f64)>,
    label: &'static str,
) -> impl IntoElement {
    let mut glass = native_glass_effect(id).style(style);
    if let Some(r) = corner_radius {
        glass = glass.corner_radius(r);
    }
    if let Some((r, g, b, a)) = tint {
        glass = glass.tint_color(r, g, b, a);
    }

    // native_glass_effect requests Size::full() internally — it fills its parent.
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.0))
        .child(
            div()
                .w(px(152.0))
                .h(px(90.0))
                .rounded(px(16.0))
                .overflow_hidden()
                .child(glass),
        )
        .child(
            div()
                .text_xs()
                .font_bold()
                .text_color(hsla(0.0, 0.0, 1.0, 0.82))
                .child(label),
        )
}

/// A small card showing an SF Symbol via `native_image_view`.
fn symbol_row(
    id: &'static str,
    symbol: &'static str,
    weight: SymbolWeight,
    scale: SymbolScale,
    point_size: f64,
    tint: Option<(f64, f64, f64, f64)>,
    label: &'static str,
) -> impl IntoElement {
    let mut view = native_image_view(id, symbol)
        .weight(weight)
        .scale(scale)
        .point_size(point_size);
    if let Some((r, g, b, a)) = tint {
        view = view.tint_color(r, g, b, a);
    }

    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.0))
        .w(px(130.0))
        .rounded(px(16.0))
        .border_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.10))
        .bg(hsla(0.0, 0.0, 1.0, 0.05))
        .p(px(14.0))
        // native_image_view fills its parent — wrap in a sized div.
        .child(
            div()
                .w(px(point_size as f32 + 8.0))
                .h(px(point_size as f32 + 8.0))
                .child(view),
        )
        .child(
            div()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                .text_center()
                .child(label),
        )
}
