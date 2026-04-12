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
            // Content grid — pad top for toolbar, bottom for breadcrumb + status bar
            .child(
                div()
                    .flex_1()
                    .p(px(24.0))
                    .pt(px(52.0))
                    .pb(px(52.0))
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
        .w(px(100.0))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.0))
        .child(
            div().w(px(64.0)).h(px(56.0)).child(
                NativeSymbol::new(format!("folder-icon-{}", name), "folder.fill")
                    .weight(SymbolWeight::Regular)
                    .scale(SymbolScale::Large)
                    .tint(0.35, 0.65, 0.95, 1.0),
            ),
        )
        .child(
            div()
                .text_sm()
                .text_color(hsla(0.0, 0.0, 0.85, 1.0))
                .child(SharedString::from(name.to_string())),
        )
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

        cx.update_window(window_handle.into(), |_view, window, _cx| {
            // Toolbar: back/forward as navigational (own glass pill), rest grouped naturally
            install_toolbar(
                window,
                &[
                    ToolbarItemId::ToggleSidebar,
                    ToolbarItemId::SidebarTrackingSeparator,
                    ToolbarItemId::SymbolButton {
                        id: "back".into(),
                        symbol: "chevron.backward".into(),
                        label: "Back".into(),
                        navigational: true,
                    },
                    ToolbarItemId::SymbolButton {
                        id: "forward".into(),
                        symbol: "chevron.forward".into(),
                        label: "Forward".into(),
                        navigational: true,
                    },
                    ToolbarItemId::FlexibleSpace,
                    ToolbarItemId::SymbolButton {
                        id: "view-mode".into(),
                        symbol: "square.grid.2x2".into(),
                        label: "View".into(),
                        navigational: false,
                    },
                    ToolbarItemId::SymbolButton {
                        id: "group".into(),
                        symbol: "rectangle.3.group".into(),
                        label: "Group".into(),
                        navigational: false,
                    },
                    ToolbarItemId::SymbolButton {
                        id: "share".into(),
                        symbol: "square.and.arrow.up".into(),
                        label: "Share".into(),
                        navigational: false,
                    },
                    ToolbarItemId::SymbolButton {
                        id: "search".into(),
                        symbol: "magnifyingglass".into(),
                        label: "Search".into(),
                        navigational: false,
                    },
                ],
            );

            // Sidebar
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

            // Native path bar (breadcrumb)
            install_breadcrumb(
                window,
                BreadcrumbConfig {
                    items: vec![
                        BreadcrumbItem {
                            title: "Macintosh HD".into(),
                            symbol: Some("internaldrive".into()),
                        },
                        BreadcrumbItem {
                            title: "Users".into(),
                            symbol: Some("person.fill".into()),
                        },
                        BreadcrumbItem {
                            title: "zac".into(),
                            symbol: Some("house.fill".into()),
                        },
                        BreadcrumbItem {
                            title: "Documents".into(),
                            symbol: Some("folder.fill".into()),
                        },
                    ],
                    ..Default::default()
                },
            );
        })
        .ok();
    });
}
