#![allow(dead_code)]

use mozui::prelude::*;
use mozui::{
    current_platform, div, hsla, px, App as MozuiApp, Application, Div, Entity, Pixels, Render,
    SharedString, Size, TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds,
    WindowOptions,
};
use mozui_components::{
    theme::{Theme, ThemeMode},
    Root, StyledExt as _,
};

pub fn run_rooted_example<V: 'static + Render>(
    title: &'static str,
    mode: ThemeMode,
    window_size: Size<Pixels>,
    build_view: impl FnOnce(&mut Window, &mut MozuiApp) -> Entity<V> + 'static,
) {
    let platform = current_platform(false);
    Application::with_platform(platform).run(move |cx: &mut MozuiApp| {
        mozui_components::init(cx);
        Theme::change(mode, None, cx);
        cx.open_window(window_options(title, window_size, cx), move |window, cx| {
            let view = build_view(window, cx);
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("failed to open rooted example window");
    });
}

pub fn run_transparent_rooted_example<V: 'static + Render>(
    title: &'static str,
    mode: ThemeMode,
    window_size: Size<Pixels>,
    build_view: impl FnOnce(&mut Window, &mut MozuiApp) -> Entity<V> + 'static,
) {
    let platform = current_platform(false);
    Application::with_platform(platform).run(move |cx: &mut MozuiApp| {
        mozui_components::init(cx);
        Theme::change(mode, None, cx);
        cx.open_window(
            transparent_window_options(title, window_size, cx),
            move |window, cx| {
                let view = build_view(window, cx);
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("failed to open rooted example window");
    });
}

pub fn transparent_window_options(
    title: &'static str,
    window_size: Size<Pixels>,
    cx: &MozuiApp,
) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::centered(window_size, cx)),
        window_min_size: Some(window_size),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(mozui::point(px(12.0), px(12.0))),
        }),
        focus: true,
        show: true,
        window_background: WindowBackgroundAppearance::Blurred,
        ..window_options(title, window_size, cx)
    }
}

pub fn run_plain_example<V: 'static + Render>(
    title: &'static str,
    window_size: Size<Pixels>,
    build_view: impl FnOnce(&mut Window, &mut MozuiApp) -> Entity<V> + 'static,
) {
    let platform = current_platform(false);
    Application::with_platform(platform).run(move |cx: &mut MozuiApp| {
        cx.open_window(window_options(title, window_size, cx), build_view)
            .expect("failed to open plain example window");
    });
}

fn window_options(title: &'static str, window_size: Size<Pixels>, cx: &MozuiApp) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::centered(window_size, cx)),
        window_min_size: Some(window_size),
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            ..Default::default()
        }),
        focus: true,
        show: true,
        ..Default::default()
    }
}

pub fn shell(title: &'static str, subtitle: &'static str) -> Div {
    div()
        .size_full()
        .bg(hsla(0.60, 0.18, 0.10, 1.0))
        .flex()
        .flex_col()
        .gap(px(20.0))
        .p(px(24.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .text_color(hsla(0.0, 0.0, 1.0, 0.96))
                        .font_semibold()
                        .text_sm()
                        .child(title),
                )
                .child(
                    div()
                        .text_color(hsla(0.0, 0.0, 1.0, 0.62))
                        .text_xs()
                        .child(subtitle),
                ),
        )
}

pub fn panel(title: &'static str, subtitle: &'static str) -> Div {
    div()
        .w_full()
        .rounded(px(24.0))
        .border_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.10))
        .bg(hsla(0.0, 0.0, 1.0, 0.05))
        .flex()
        .flex_col()
        .gap(px(14.0))
        .p(px(18.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .text_color(hsla(0.0, 0.0, 1.0, 0.92))
                        .font_semibold()
                        .text_sm()
                        .child(title),
                )
                .child(
                    div()
                        .text_color(hsla(0.0, 0.0, 1.0, 0.60))
                        .text_xs()
                        .child(subtitle),
                ),
        )
}

pub fn stat_tile(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Div {
    let label = label.into();
    let value = value.into();

    div()
        .w(px(148.0))
        .rounded(px(18.0))
        .border_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.08))
        .bg(hsla(0.0, 0.0, 1.0, 0.04))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(14.0))
        .child(
            div()
                .text_color(hsla(0.0, 0.0, 1.0, 0.56))
                .text_xs()
                .child(label),
        )
        .child(
            div()
                .text_color(hsla(0.0, 0.0, 1.0, 0.96))
                .font_semibold()
                .text_sm()
                .child(value),
        )
}

pub fn labeled_control(label: &'static str, note: &'static str, control: impl IntoElement) -> Div {
    div()
        .w_full()
        .rounded(px(18.0))
        .border_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.08))
        .bg(hsla(0.0, 0.0, 1.0, 0.03))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .p(px(14.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .text_color(hsla(0.0, 0.0, 1.0, 0.88))
                        .text_xs()
                        .font_semibold()
                        .child(label),
                )
                .child(
                    div()
                        .text_color(hsla(0.0, 0.0, 1.0, 0.56))
                        .text_xs()
                        .child(note),
                ),
        )
        .child(control)
}

pub fn slider_value_label(value: f32) -> SharedString {
    format!("{value:.0}").into()
}
