use mozui::prelude::*;
use mozui::{
    App, Context, Div, SharedString, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowOptions, div, hsla, platform::application, point, px, size,
};
use mozui_native::*;

struct ControlsGallery;

impl Render for ControlsGallery {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("gallery")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .bg(hsla(0.0, 0.0, 0.15, 1.0))
            .pt(px(52.0))
            .p(px(24.0))
            .gap(px(20.0))
            // Section: Buttons
            .child(section_header("Buttons"))
            .child(
                row()
                    .child(
                        control_cell("Push Button", 140.0, 28.0).child(
                            NativeButton::new("btn-push", "Click Me")
                                .style(NativeButtonStyle::Push)
                                .on_click(|| println!("Push button clicked")),
                        ),
                    )
                    .child(
                        control_cell("With Symbol", 160.0, 28.0).child(
                            NativeButton::new("btn-sym", "Download")
                                .symbol("arrow.down.circle")
                                .on_click(|| println!("Download clicked")),
                        ),
                    )
                    .child(
                        control_cell("Toolbar Style", 100.0, 28.0).child(
                            NativeButton::new("btn-toolbar", "Edit")
                                .style(NativeButtonStyle::Toolbar)
                                .on_click(|| println!("Toolbar button clicked")),
                        ),
                    )
                    .child(
                        control_cell("Disabled", 140.0, 28.0)
                            .child(NativeButton::new("btn-disabled", "Disabled").enabled(false)),
                    ),
            )
            // Section: Text Fields
            .child(section_header("Text Fields"))
            .child(
                row()
                    .child(
                        control_cell("Editable", 200.0, 24.0).child(
                            NativeTextField::new("tf-edit")
                                .placeholder("Type here...")
                                .on_change(|text| println!("Text: {}", text)),
                        ),
                    )
                    .child(
                        control_cell("With Value", 200.0, 24.0).child(
                            NativeTextField::new("tf-value")
                                .value("Hello, mozui!")
                                .font_size(13.0),
                        ),
                    )
                    .child(
                        control_cell("Label (Read-only)", 200.0, 20.0).child(
                            NativeTextField::label("tf-label", "This is a native label")
                                .font_size(13.0),
                        ),
                    ),
            )
            // Section: Toggles & Switches
            .child(section_header("Switches"))
            .child(
                row()
                    .child(control_cell("Off", 50.0, 24.0).child(
                        NativeSwitch::new("sw-off").on_toggle(|on| println!("Switch: {}", on)),
                    ))
                    .child(
                        control_cell("On", 50.0, 24.0).child(
                            NativeSwitch::new("sw-on")
                                .is_on(true)
                                .on_toggle(|on| println!("Switch: {}", on)),
                        ),
                    )
                    .child(
                        control_cell("Disabled", 50.0, 24.0)
                            .child(NativeSwitch::new("sw-disabled").is_on(true).enabled(false)),
                    ),
            )
            // Section: Picker
            .child(section_header("Picker (Dropdown)"))
            .child(
                row().child(
                    control_cell("Font", 180.0, 28.0).child(
                        NativePicker::new(
                            "picker-font",
                            vec![
                                "San Francisco".into(),
                                "Helvetica".into(),
                                "Monaco".into(),
                                "Menlo".into(),
                            ],
                        )
                        .selected(0)
                        .on_change(|idx| println!("Selected font index: {}", idx)),
                    ),
                ),
            )
            // Section: Slider
            .child(section_header("Sliders"))
            .child(
                row()
                    .child(
                        control_cell("Volume", 200.0, 24.0).child(
                            NativeSlider::new("slider-vol")
                                .range(0.0, 100.0)
                                .value(50.0)
                                .on_change(|v| println!("Volume: {:.0}", v)),
                        ),
                    )
                    .child(
                        control_cell("Opacity", 200.0, 24.0).child(
                            NativeSlider::new("slider-opacity")
                                .range(0.0, 1.0)
                                .value(0.8)
                                .on_change(|v| println!("Opacity: {:.2}", v)),
                        ),
                    ),
            )
            // Section: Progress
            .child(section_header("Progress Indicators"))
            .child(
                row()
                    .child(
                        control_cell("Determinate (60%)", 200.0, 20.0).child(
                            NativeProgress::new("prog-det")
                                .value(60.0)
                                .style(ProgressStyle::Bar),
                        ),
                    )
                    .child(
                        control_cell("Indeterminate", 200.0, 20.0)
                            .child(NativeProgress::new("prog-indet").style(ProgressStyle::Bar)),
                    )
                    .child(
                        control_cell("Spinner", 24.0, 24.0)
                            .child(NativeProgress::new("prog-spin").style(ProgressStyle::Spinning)),
                    ),
            )
            // Section: Stepper
            .child(section_header("Stepper"))
            .child(
                row().child(
                    control_cell("Count", 24.0, 40.0).child(
                        NativeStepper::new("stepper-count")
                            .range(0.0, 10.0)
                            .value(5.0)
                            .increment(1.0)
                            .on_change(|v| println!("Stepper: {:.0}", v)),
                    ),
                ),
            )
            // Section: Date Picker
            .child(section_header("Date Picker"))
            .child(
                row()
                    .child(
                        control_cell("Date & Time", 240.0, 28.0).child(
                            NativeDatePicker::new("dp-datetime")
                                .elements(DatePickerElements::DateAndTime)
                                .on_change(|| println!("Date changed")),
                        ),
                    )
                    .child(control_cell("Date Only", 180.0, 28.0).child(
                        NativeDatePicker::new("dp-date").elements(DatePickerElements::DateOnly),
                    )),
            )
            // Section: Color Picker
            .child(section_header("Color Picker"))
            .child(
                row().child(
                    control_cell("Pick a Color", 44.0, 28.0).child(
                        NativeColorPicker::new("color-pick")
                            .color(NativeColor::new(0.2, 0.5, 0.9, 1.0))
                            .on_change(|c| {
                                println!(
                                    "Color: r={:.2} g={:.2} b={:.2} a={:.2}",
                                    c.r, c.g, c.b, c.a
                                )
                            }),
                    ),
                ),
            )
            // Section: SF Symbols
            .child(section_header("SF Symbols"))
            .child(
                row()
                    .child(
                        control_cell("Small", 24.0, 24.0).child(
                            NativeSymbol::new("sym-small", "star.fill")
                                .scale(SymbolScale::Small)
                                .tint(1.0, 0.8, 0.0, 1.0),
                        ),
                    )
                    .child(
                        control_cell("Medium", 32.0, 32.0).child(
                            NativeSymbol::new("sym-medium", "heart.fill")
                                .scale(SymbolScale::Medium)
                                .tint(1.0, 0.3, 0.3, 1.0),
                        ),
                    )
                    .child(
                        control_cell("Large (20pt)", 40.0, 40.0).child(
                            NativeSymbol::new("sym-large", "cloud.fill")
                                .scale(SymbolScale::Large)
                                .point_size(20.0)
                                .tint(0.5, 0.7, 1.0, 1.0),
                        ),
                    )
                    .child(
                        control_cell("Bold", 32.0, 32.0).child(
                            NativeSymbol::new("sym-bold", "bolt.fill")
                                .weight(SymbolWeight::Bold)
                                .tint(1.0, 0.6, 0.0, 1.0),
                        ),
                    ),
            )
            // Section: Visual Effects
            .child(section_header("Visual Effects"))
            .child(
                row()
                    .child(
                        div()
                            .w(px(120.0))
                            .h(px(80.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 0.6, 1.0))
                                    .child(SharedString::from("Sidebar Material")),
                            )
                            .child(
                                div().flex_1().child(
                                    NativeVisualEffect::new("ve-sidebar")
                                        .material(VisualEffectMaterial::Sidebar)
                                        .blending(VisualEffectBlending::BehindWindow),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .w(px(120.0))
                            .h(px(80.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 0.6, 1.0))
                                    .child(SharedString::from("HUD Material")),
                            )
                            .child(
                                div().flex_1().child(
                                    NativeVisualEffect::new("ve-hud")
                                        .material(VisualEffectMaterial::HudWindow)
                                        .blending(VisualEffectBlending::BehindWindow),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .w(px(120.0))
                            .h(px(80.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 0.6, 1.0))
                                    .child(SharedString::from("Glass Effect")),
                            )
                            .child(
                                div().flex_1().child(
                                    NativeGlassEffect::new("glass-demo").corner_radius(12.0),
                                ),
                            ),
                    ),
            )
    }
}

fn section_header(title: &str) -> impl IntoElement {
    div()
        .w_full()
        .pb(px(4.0))
        .border_b_1()
        .border_color(hsla(0.0, 0.0, 0.3, 0.3))
        .child(
            div()
                .text_sm()
                .text_color(hsla(0.0, 0.0, 0.7, 1.0))
                .child(SharedString::from(title.to_string())),
        )
}

fn row() -> Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(16.0))
        .items_end()
}

fn control_cell(label: &str, width: f32, height: f32) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                .child(SharedString::from(label.to_string())),
        )
        .child(div().w(px(width)).h(px(height)))
}

fn main() {
    application().run(|cx: &mut App| {
        let options = WindowOptions {
            window_bounds: Some(mozui::WindowBounds::Windowed(mozui::Bounds {
                origin: Default::default(),
                size: size(px(720.), px(800.)),
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("Native Controls Gallery".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(16.0), px(16.0))),
            }),
            window_background: WindowBackgroundAppearance::Blurred,
            ..Default::default()
        };

        let window_handle = cx
            .open_window(options, |_window, cx| cx.new(|_cx| ControlsGallery))
            .unwrap();

        cx.update_window(window_handle.into(), |_view, window, _cx| {
            install_toolbar(
                window,
                &[
                    ToolbarItemId::FlexibleSpace,
                    ToolbarItemId::SymbolButton {
                        id: "settings".into(),
                        symbol: "gearshape".into(),
                        label: "Settings".into(),
                        navigational: false,
                    },
                ],
            );
        })
        .ok();
    });
}
