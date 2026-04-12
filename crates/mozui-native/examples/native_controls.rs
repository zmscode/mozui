use mozui::prelude::*;
use mozui::{
    App, Context, SharedString, TitlebarOptions, Window, WindowBackgroundAppearance, WindowOptions,
    div, hsla, platform::application, point, px, size,
};
use mozui_native::*;

struct FinderDemo;

impl Render for FinderDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(hsla(0.0, 0.0, 0.12, 1.0))
            // Content grid
            .child(
                div()
                    .flex_1()
                    .p(px(24.0))
                    .pt(px(12.0))
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(24.0))
                    .items_start()
                    .content_start()
                    .child(folder_item("Desktop"))
                    .child(folder_item("Downloads"))
                    .child(folder_item("Documents"))
                    .child(folder_item("Pictures"))
                    .child(folder_item("Music"))
                    .child(folder_item("Movies")),
            )
            // Path bar
            .child(
                div()
                    .w_full()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(4.0))
                    .border_t_1()
                    .border_color(hsla(0.0, 0.0, 0.3, 0.3))
                    .bg(hsla(0.0, 0.0, 0.1, 1.0))
                    .child(path_segment("Macintosh HD"))
                    .child(path_separator())
                    .child(path_segment("Users"))
                    .child(path_separator())
                    .child(path_segment("zac"))
                    .child(path_separator())
                    .child(path_segment("Documents")),
            )
            // Status bar
            .child(
                div()
                    .w_full()
                    .h(px(22.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_t_1()
                    .border_color(hsla(0.0, 0.0, 0.3, 0.3))
                    .bg(hsla(0.0, 0.0, 0.1, 1.0))
                    .child(
                        div()
                            .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                            .text_xs()
                            .child(SharedString::from("6 items")),
                    ),
            )
    }
}

fn folder_item(name: &str) -> impl IntoElement {
    div()
        .w(px(80.0))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.0))
        .child(
            div().w(px(48.0)).h(px(40.0)).child(
                NativeSymbol::new(format!("folder-icon-{}", name), "folder.fill")
                    .weight(SymbolWeight::Regular)
                    .scale(SymbolScale::Large)
                    .tint(0.35, 0.65, 0.95, 1.0),
            ),
        )
        .child(
            div()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 0.85, 1.0))
                .child(SharedString::from(name.to_string())),
        )
}

fn path_segment(name: &str) -> impl IntoElement {
    div()
        .px(px(8.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(hsla(0.0, 0.0, 0.2, 1.0))
        .text_xs()
        .text_color(hsla(0.0, 0.0, 0.75, 1.0))
        .child(SharedString::from(name.to_string()))
}

fn path_separator() -> impl IntoElement {
    div()
        .text_xs()
        .text_color(hsla(0.0, 0.0, 0.4, 1.0))
        .child(SharedString::from("\u{203A}"))
}

fn main() {
    application().run(|cx: &mut App| {
        let options = WindowOptions {
            window_bounds: Some(mozui::WindowBounds::Windowed(mozui::Bounds {
                origin: Default::default(),
                size: size(px(900.), px(550.)),
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("Documents".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(16.0), px(16.0))),
            }),
            window_background: WindowBackgroundAppearance::Blurred,
            ..Default::default()
        };

        let window_handle = cx
            .open_window(options, |_window, cx| cx.new(|_cx| FinderDemo))
            .unwrap();

        // Install native toolbar and sidebar after window creation
        cx.update_window(window_handle.into(), |_view, window, _cx| {
            // Install native toolbar with Finder-like items
            install_toolbar(
                window,
                &[
                    ToolbarItemId::ToggleSidebar,
                    ToolbarItemId::SidebarTrackingSeparator,
                    ToolbarItemId::SymbolButton {
                        id: "back".into(),
                        symbol: "chevron.backward".into(),
                        label: "Back".into(),
                    },
                    ToolbarItemId::Space,
                    ToolbarItemId::SymbolButton {
                        id: "forward".into(),
                        symbol: "chevron.forward".into(),
                        label: "Forward".into(),
                    },
                    ToolbarItemId::FlexibleSpace,
                    ToolbarItemId::SymbolButton {
                        id: "view-mode".into(),
                        symbol: "square.grid.2x2".into(),
                        label: "View".into(),
                    },
                    ToolbarItemId::Space,
                    ToolbarItemId::SymbolButton {
                        id: "group".into(),
                        symbol: "rectangle.3.group".into(),
                        label: "Group".into(),
                    },
                    ToolbarItemId::Space,
                    ToolbarItemId::SymbolButton {
                        id: "share".into(),
                        symbol: "square.and.arrow.up".into(),
                        label: "Share".into(),
                    },
                    ToolbarItemId::Space,
                    ToolbarItemId::SymbolButton {
                        id: "search".into(),
                        symbol: "magnifyingglass".into(),
                        label: "Search".into(),
                    },
                ],
            );

            // Install NSSplitViewController-based sidebar
            install_sidebar(
                window,
                SidebarConfig {
                    sections: vec![
                        SidebarSection {
                            title: "Favourites".into(),
                            items: vec![
                                SidebarItem {
                                    title: "AirDrop".into(),
                                    symbol: "wifi".into(),
                                },
                                SidebarItem {
                                    title: "Recents".into(),
                                    symbol: "clock".into(),
                                },
                                SidebarItem {
                                    title: "Applications".into(),
                                    symbol: "square.grid.2x2".into(),
                                },
                                SidebarItem {
                                    title: "Desktop".into(),
                                    symbol: "menubar.dock.rectangle".into(),
                                },
                                SidebarItem {
                                    title: "Documents".into(),
                                    symbol: "doc".into(),
                                },
                                SidebarItem {
                                    title: "Downloads".into(),
                                    symbol: "arrow.down.circle".into(),
                                },
                            ],
                        },
                        SidebarSection {
                            title: "iCloud".into(),
                            items: vec![
                                SidebarItem {
                                    title: "iCloud Drive".into(),
                                    symbol: "icloud".into(),
                                },
                                SidebarItem {
                                    title: "Shared".into(),
                                    symbol: "person.2".into(),
                                },
                            ],
                        },
                        SidebarSection {
                            title: "Locations".into(),
                            items: vec![SidebarItem {
                                title: "Macintosh HD".into(),
                                symbol: "internaldrive".into(),
                            }],
                        },
                    ],
                    ..Default::default()
                },
            );
        })
        .ok();
    });
}
