mod support;

use mozui::prelude::*;
use mozui::{
    ClickEvent, Context, GlassEffectStyle, NativeTextField, ProgressStyle, SharedString,
    SymbolScale, SymbolWeight, VisualEffectMaterial, Window, div, native_button,
    native_glass_effect, native_image_view, native_progress, native_search_field, native_slider,
    native_switch, native_text_field, native_visual_effect, px, size,
};
use support::{labeled_control, panel, run_plain_example, shell, stat_tile};

fn main() {
    run_plain_example(
        "Native Controls",
        size(px(920.0), px(760.0)),
        |window, cx| cx.new(|cx| NativeControlsExample::new(window, cx)),
    );
}

struct NativeControlsExample {
    display_name: SharedString,
    query: SharedString,
    sync_enabled: bool,
    intensity: f64,
    job_progress: f64,
    launches: usize,
}

impl NativeControlsExample {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            display_name: "Aurora".into(),
            query: "native toolbar".into(),
            sync_enabled: true,
            intensity: 62.0,
            job_progress: 45.0,
            launches: 0,
        }
    }

    fn set_display_name(
        &mut self,
        event: &mozui::TextFieldChangeEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.display_name = event.text.clone();
        cx.notify();
    }

    fn set_query(
        &mut self,
        event: &mozui::TextFieldChangeEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.query = event.text.clone();
        cx.notify();
    }

    fn submit_query(
        &mut self,
        event: &mozui::TextFieldSubmitEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.query = event.text.clone();
        self.job_progress = (self.job_progress + 9.0).min(100.0);
        cx.notify();
    }

    fn set_sync_enabled(
        &mut self,
        event: &mozui::SwitchChangeEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync_enabled = event.checked;
        cx.notify();
    }

    fn set_intensity(
        &mut self,
        event: &mozui::SliderChangeEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.intensity = event.value;
        cx.notify();
    }

    fn launch_job(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.launches += 1;
        self.job_progress = (self.job_progress + 13.0).min(100.0);
        cx.notify();
    }
}

impl Render for NativeControlsExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        shell(
            "Pure core-native controls",
            "This example uses the mozui native leaf controls directly instead of semantic mozui-components wrappers.",
        )
        .id("native-controls-scroll")
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(stat_tile("Display name", self.display_name.clone()))
                .child(stat_tile("Intensity", format!("{:.0}%", self.intensity)))
                .child(stat_tile("Launches", format!("{}", self.launches))),
        )
        .child(
            panel(
                "Native Form Controls",
                "Every interactive control below is backed by the platform-native control layer in mozui core.",
            )
            .child(labeled_control(
                "Display Name",
                "Plain native text field with change callbacks routed through the mozui event loop.",
                native_text_field("native-controls-name")
                    .placeholder("Display name")
                    .value(self.display_name.clone())
                    .on_change(cx.listener(Self::set_display_name)),
            ))
            .child(labeled_control(
                "Find Anything",
                "Search-styled native field with both change and submit callbacks.",
                native_search_field("native-controls-query")
                    .placeholder("Search files, tabs, and symbols")
                    .value(self.query.clone())
                    .on_change(cx.listener(Self::set_query))
                    .on_submit(cx.listener(Self::submit_query)),
            ))
            .child(labeled_control(
                "Sync State",
                "Platform-native switch rendered directly from mozui core.",
                native_switch("native-controls-sync")
                    .checked(self.sync_enabled)
                    .on_change(cx.listener(Self::set_sync_enabled)),
            ))
            .child(labeled_control(
                "Pipeline Intensity",
                "Native slider bound to a live percentage summary.",
                native_slider("native-controls-slider")
                    .range(0.0, 100.0)
                    .value(self.intensity)
                    .on_change(cx.listener(Self::set_intensity)),
            ))
            .child(labeled_control(
                "Native Progress",
                "Determinate native progress indicator for the current background job.",
                native_progress("native-controls-progress")
                    .range(0.0, 100.0)
                    .value(self.job_progress),
            ))
            .child(
                native_button("native-controls-launch", "Schedule Backup")
                    .button_style(mozui::NativeButtonStyle::Filled)
                    .on_click(cx.listener(Self::launch_job)),
            ),
        )
        .child(
            panel(
                "Read-only Status",
                "Readonly labels can also stay on the native path when you want the AppKit/UIKit text field treatment.",
            )
            .child(
                NativeTextField::label(
                    "native-controls-status",
                    format!(
                        "Search query: {} | Sync enabled: {} | Progress: {:.0}%",
                        self.query,
                        if self.sync_enabled { "yes" } else { "no" },
                        self.job_progress
                    ),
                )
                .font_size(12.0)
                .bezeled(false),
            )
            .child(
                native_progress("native-controls-spinner")
                    .progress_style(ProgressStyle::Spinning)
                    .w(px(24.0))
                    .h(px(24.0)),
            ),
        )
        .child(
            panel(
                "Visual Effects",
                "native_visual_effect renders an NSVisualEffectView directly inside the mozui window hierarchy.",
            )
            .child(labeled_control(
                "Sidebar blur",
                "NSVisualEffectView with Sidebar material and BehindWindow blending.",
                native_visual_effect("native-controls-ve-sidebar")
                    .material(VisualEffectMaterial::Sidebar)
                    .w_full()
                    .h(px(48.0)),
            ))
            .child(labeled_control(
                "Popover blur",
                "NSVisualEffectView with Popover material.",
                native_visual_effect("native-controls-ve-popover")
                    .material(VisualEffectMaterial::Popover)
                    .w_full()
                    .h(px(48.0)),
            ))
            .child(labeled_control(
                "Glass effect",
                "native_glass_effect uses NSGlassEffectView on macOS 26+ and falls back to NSVisualEffectView.",
                native_glass_effect("native-controls-glass")
                    .style(GlassEffectStyle::Regular)
                    .corner_radius(12.0)
                    .w_full()
                    .h(px(48.0)),
            )),
        )
        .child(
            panel(
                "SF Symbol Image Views",
                "native_image_view wraps NSImageView and renders SF Symbols with configurable weight, scale, and tint.",
            )
            .child(labeled_control(
                "Symbol row",
                "Five symbols with varying weight, scale, and tint settings.",
                div()
                    .flex()
                    .gap(px(16.0))
                    .items_center()
                    .child(
                        native_image_view("native-controls-sym-folder", "folder.fill")
                            .weight(SymbolWeight::Regular)
                            .scale(SymbolScale::Medium)
                            .point_size(20.0)
                            .w(px(28.0))
                            .h(px(28.0)),
                    )
                    .child(
                        native_image_view("native-controls-sym-bolt", "bolt.fill")
                            .weight(SymbolWeight::Bold)
                            .scale(SymbolScale::Large)
                            .point_size(24.0)
                            .tint_color(1.0, 0.75, 0.2, 1.0)
                            .w(px(32.0))
                            .h(px(32.0)),
                    )
                    .child(
                        native_image_view("native-controls-sym-wand", "wand.and.stars")
                            .weight(SymbolWeight::Thin)
                            .scale(SymbolScale::Medium)
                            .point_size(22.0)
                            .tint_color(0.5, 0.9, 1.0, 1.0)
                            .w(px(30.0))
                            .h(px(30.0)),
                    )
                    .child(
                        native_image_view("native-controls-sym-heart", "heart.fill")
                            .weight(SymbolWeight::Black)
                            .scale(SymbolScale::Large)
                            .point_size(28.0)
                            .tint_color(1.0, 0.3, 0.4, 1.0)
                            .w(px(36.0))
                            .h(px(36.0)),
                    )
                    .child(
                        native_image_view("native-controls-sym-cloud", "cloud.fill")
                            .weight(SymbolWeight::Semibold)
                            .scale(SymbolScale::Small)
                            .point_size(18.0)
                            .w(px(26.0))
                            .h(px(26.0)),
                    ),
            )),
        )
    }
}
