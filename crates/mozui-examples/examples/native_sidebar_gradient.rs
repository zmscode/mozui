mod support;

use mozui::prelude::*;
use mozui::{
    Context, NativeHostedSurfaceTarget, NativeSidebarHost, NativeToolbar, NativeToolbarButton,
    NativeToolbarDisplayMode, NativeToolbarItem, Render, SharedString, Window, div, hsla,
    linear_color_stop, linear_gradient, px, size,
};
use mozui_components::{
    StyledExt as _,
    button::{Button, ButtonVariants},
    theme::ThemeMode,
};
use support::run_transparent_rooted_example;

const SIDEBAR_IDENTIFIER: &str = "mozui-examples.native-sidebar";
const FAVORITES: [(&str, &str); 3] = [
    ("home", "Home"),
    ("today", "Today"),
    ("reading-list", "Reading List"),
];
const COLLECTIONS: [(&str, &str); 3] = [
    ("browser", "Browser"),
    ("references", "References"),
    ("archive", "Archive"),
];

fn main() {
    run_transparent_rooted_example(
        "Native Sidebar Gradient",
        ThemeMode::Dark,
        size(px(1280.0), px(820.0)),
        |window, cx| cx.new(|cx| NativeSidebarGradientExample::new(window, cx)),
    );
}

struct NativeSidebarGradientExample {
    sidebar_visible: bool,
    last_action: SharedString,
}

impl NativeSidebarGradientExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_background_appearance(mozui::WindowBackgroundAppearance::Blurred);
        install_native_sidebar(window, cx);

        let toggle_view = cx.entity().clone();
        window.set_native_toolbar(
            NativeToolbar::new()
                .display_mode(NativeToolbarDisplayMode::IconOnly)
                .shows_title(false)
                .item(NativeToolbarItem::Button(
                    NativeToolbarButton::new(
                        "native-sidebar-gradient-toggle",
                        "sidebar.left",
                        "Sidebar",
                    )
                    .on_activate(move |window, cx| {
                        toggle_view.update(cx, |this, cx| {
                            this.sidebar_visible = !this.sidebar_visible;
                            let _ = window
                                .set_native_host_visibility(SIDEBAR_IDENTIFIER, this.sidebar_visible);
                            this.last_action = if this.sidebar_visible {
                                "Expanded native mozui sidebar surface".into()
                            } else {
                                "Collapsed native mozui sidebar surface".into()
                            };
                            cx.notify();
                        });
                    }),
                )),
        );

        Self {
            sidebar_visible: true,
            last_action: "Mounted native mozui sidebar surface".into(),
        }
    }
}

impl Render for NativeSidebarGradientExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("native-sidebar-gradient-root")
            .size_full()
            .bg(linear_gradient(
                180.,
                linear_color_stop(hsla(0.79, 0.42, 0.24, 1.0), 0.0),
                linear_color_stop(hsla(0.67, 0.26, 0.12, 1.0), 1.0),
            ))
            .child(
                div()
                    .size_full()
                    .pt(px(74.0))
                    .px(px(36.0))
                    .pb(px(30.0))
                    .flex()
                    .justify_end()
                    .child(
                        div()
                            .w(px(360.0))
                            .rounded(px(18.0))
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, 0.12))
                            .bg(hsla(0.0, 0.0, 0.08, 0.24))
                            .px(px(18.0))
                            .py(px(16.0))
                            .flex()
                            .flex_col()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.95))
                                    .child("Actual native sidebar"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.72))
                                    .child(
                                        "The left column is now a real mozui-rendered sidebar surface mounted inside mozui's native macOS sidebar shell.",
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.64))
                                    .child(format!("Last action: {}", self.last_action)),
                            )
                            .child(
                                Button::new("native-sidebar-gradient-toggle-inline")
                                    .label(if self.sidebar_visible {
                                        "Hide Sidebar"
                                    } else {
                                        "Show Sidebar"
                                    })
                                    .secondary()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.sidebar_visible = !this.sidebar_visible;
                                        let _ = window.set_native_host_visibility(
                                            SIDEBAR_IDENTIFIER,
                                            this.sidebar_visible,
                                        );
                                        this.last_action = if this.sidebar_visible {
                                            "Expanded native mozui sidebar surface".into()
                                        } else {
                                            "Collapsed native mozui sidebar surface".into()
                                        };
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }
}

#[cfg(not(target_os = "macos"))]
fn install_native_sidebar(_window: &mut Window, _cx: &mut mozui::App) {}

#[cfg(target_os = "macos")]
fn install_native_sidebar(window: &mut Window, cx: &mut mozui::App) {
    if !window.install_native_sidebar_host(
        NativeSidebarHost::new(SIDEBAR_IDENTIFIER, px(236.0))
            .min_width(px(180.0))
            .max_width(px(320.0))
            .visible(true),
    ) {
        return;
    }

    let sidebar_surface = cx.new(|_| SidebarSurfaceDemo::new());
    let sidebar_handle = window.register_surface(sidebar_surface, cx);
    let _ = window.attach_native_hosted_surface(
        SIDEBAR_IDENTIFIER,
        sidebar_handle.native_view_ptr(),
        NativeHostedSurfaceTarget::Sidebar,
    );
}

#[cfg(target_os = "macos")]
struct SidebarSurfaceDemo {
    selected_item: &'static str,
}

#[cfg(target_os = "macos")]
impl SidebarSurfaceDemo {
    fn new() -> Self {
        Self {
            selected_item: "browser",
        }
    }

    fn render_section(
        &self,
        title: &'static str,
        items: &[(&'static str, &'static str)],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut section = div().flex().flex_col().gap(px(6.0)).w_full().child(
            div()
                .text_xs()
                .font_semibold()
                .text_color(hsla(0.0, 0.0, 1.0, 0.44))
                .px(px(8.0))
                .child(title),
        );

        for &(item_id, label) in items {
            section = section.child(self.render_row(item_id, label, cx));
        }

        section
    }

    fn render_row(
        &self,
        item_id: &'static str,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected_item == item_id;
        div()
            .id(format!("native-sidebar-surface-row-{item_id}"))
            .w_full()
            .h(px(30.0))
            .px(px(10.0))
            .rounded(px(10.0))
            .cursor_pointer()
            .flex()
            .items_center()
            .gap(px(10.0))
            .text_sm()
            .font_semibold()
            .text_color(if selected {
                hsla(0.0, 0.0, 1.0, 0.96)
            } else {
                hsla(0.0, 0.0, 1.0, 0.72)
            })
            .bg(if selected {
                hsla(0.0, 0.0, 1.0, 0.12)
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(hsla(0.0, 0.0, 1.0, 0.08)))
            .child(
                div()
                    .size(px(8.0))
                    .rounded_full()
                    .bg(if selected {
                        hsla(0.88, 0.75, 0.70, 1.0)
                    } else {
                        hsla(0.0, 0.0, 1.0, 0.18)
                    }),
            )
            .child(div().child(label))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_item = item_id;
                cx.notify();
            }))
    }
}

#[cfg(target_os = "macos")]
impl Render for SidebarSurfaceDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("native-sidebar-gradient-surface")
            .size_full()
            .overflow_y_scroll()
            .pt(px(74.0))
            .px(px(10.0))
            .pb(px(12.0))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(18.0))
                    .child(self.render_section("Favorites", &FAVORITES, cx))
                    .child(self.render_section("Collections", &COLLECTIONS, cx)),
            )
    }
}
