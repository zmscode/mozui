use mozui::prelude::*;
use mozui::{
    App, Context, Div, SharedString, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowOptions, div, hsla, platform::application, point, px, size,
};
use mozui_native::*;

struct DialogsDemo;

impl Render for DialogsDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(hsla(0.0, 0.0, 0.15, 1.0))
            .pt(px(52.0))
            .p(px(32.0))
            .gap(px(24.0))
            // Alert section
            .child(section("Alerts"))
            .child(
                button_row()
                    .child(
                        div().w(px(160.0)).h(px(28.0)).child(
                            NativeButton::new("alert-info", "Info Alert")
                                .symbol("info.circle")
                                .on_click(|| println!("Would show info alert")),
                        ),
                    )
                    .child(
                        div().w(px(180.0)).h(px(28.0)).child(
                            NativeButton::new("alert-confirm", "Confirm Dialog")
                                .symbol("questionmark.circle")
                                .on_click(|| println!("Would show confirm dialog")),
                        ),
                    )
                    .child(
                        div().w(px(180.0)).h(px(28.0)).child(
                            NativeButton::new("alert-destructive", "Delete Item")
                                .symbol("trash")
                                .on_click(|| println!("Would show destructive alert")),
                        ),
                    ),
            )
            // File dialogs section
            .child(section("File Dialogs"))
            .child(
                button_row()
                    .child(
                        div().w(px(160.0)).h(px(28.0)).child(
                            NativeButton::new("file-open", "Open File...")
                                .symbol("doc")
                                .on_click(|| println!("Would show open dialog")),
                        ),
                    )
                    .child(
                        div().w(px(160.0)).h(px(28.0)).child(
                            NativeButton::new("file-save", "Save As...")
                                .symbol("square.and.arrow.down")
                                .on_click(|| println!("Would show save dialog")),
                        ),
                    )
                    .child(
                        div().w(px(200.0)).h(px(28.0)).child(
                            NativeButton::new("file-folder", "Choose Folder...")
                                .symbol("folder")
                                .on_click(|| println!("Would show folder picker")),
                        ),
                    ),
            )
            // Menu section
            .child(section("Context Menus"))
            .child(
                button_row().child(
                    div().flex().gap(px(8.0)).child(
                        div().text_sm().text_color(hsla(0.0, 0.0, 0.6, 1.0)).child(
                            SharedString::from(
                                "Right-click anywhere in the window to see context menus in action",
                            ),
                        ),
                    ),
                ),
            )
            // Share section
            .child(section("Sharing"))
            .child(
                button_row().child(
                    div().w(px(160.0)).h(px(28.0)).child(
                        NativeButton::new("share-btn", "Share...")
                            .symbol("square.and.arrow.up")
                            .on_click(|| println!("Would show share picker")),
                    ),
                ),
            )
            // Drag & Drop section
            .child(section("Drag & Drop"))
            .child(
                div()
                    .w_full()
                    .h(px(120.0))
                    .border_2()
                    .border_color(hsla(0.0, 0.0, 0.3, 0.5))
                    .rounded(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div().w(px(32.0)).h(px(32.0)).child(
                                    NativeSymbol::new("drop-icon", "arrow.down.doc")
                                        .scale(SymbolScale::Large)
                                        .tint(0.5, 0.5, 0.5, 1.0),
                                ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                                    .child(SharedString::from("Drop files here")),
                            ),
                    ),
            )
            // Info text
            .child(
                div()
                    .w_full()
                    .pt(px(12.0))
                    .border_t_1()
                    .border_color(hsla(0.0, 0.0, 0.3, 0.3))
                    .child(div().text_xs().text_color(hsla(0.0, 0.0, 0.4, 1.0)).child(
                        SharedString::from(
                            "Note: Alerts, file dialogs, and popovers require a &Window reference. \
                                 In a real app, trigger them from window-level callbacks.",
                        ),
                    )),
            )
    }
}

fn section(title: &str) -> impl IntoElement {
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

fn button_row() -> Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(12.0))
        .items_center()
}

fn main() {
    application().run(|cx: &mut App| {
        let options = WindowOptions {
            window_bounds: Some(mozui::WindowBounds::Windowed(mozui::Bounds {
                origin: Default::default(),
                size: size(px(680.), px(640.)),
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("Dialogs & System Integration".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(16.0), px(16.0))),
            }),
            window_background: WindowBackgroundAppearance::Blurred,
            ..Default::default()
        };

        let window_handle = cx
            .open_window(options, |_window, cx| cx.new(|_cx| DialogsDemo))
            .unwrap();

        cx.update_window(window_handle.into(), |_view, window, _cx| {
            // Install a toolbar with common actions
            install_toolbar(
                window,
                &[
                    ToolbarItemId::FlexibleSpace,
                    ToolbarItemId::SymbolGroup {
                        id: "actions".into(),
                        items: vec![
                            ("plus".into(), "Add".into()),
                            ("minus".into(), "Remove".into()),
                        ],
                    },
                ],
            );

            // Register as a drop target for files
            register_drop_target(
                window,
                &[DragType::Files],
                Box::new(|items| {
                    for item in &items {
                        println!("Dropped: {}", item);
                    }
                }),
            );
        })
        .ok();
    });
}
