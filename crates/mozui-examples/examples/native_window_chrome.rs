mod support;

use mozui::prelude::*;
use mozui::{
    ClickEvent, Context, NativeAnchor, NativeMenu, NativeMenuItem, NativeMenuKind,
    NativeSearchEvent, NativeTextField, NativeToolbar, NativeToolbarButton,
    NativeToolbarDisplayMode, NativeToolbarGroup, NativeToolbarGroupEvent, NativeToolbarGroupItem,
    NativeToolbarItem, NativeToolbarSearchField, SharedString, Window, div, native_button,
    native_progress, px, size,
};
use support::{panel, run_plain_example, shell, stat_tile};

fn main() {
    run_plain_example(
        "Native Window Chrome",
        size(px(980.0), px(760.0)),
        |window, cx| cx.new(|cx| NativeWindowChromeExample::new(window, cx)),
    );
}

struct NativeWindowChromeExample {
    source_mode: SharedString,
    toolbar_query: SharedString,
    last_action: SharedString,
    menu_opens: usize,
}

impl NativeWindowChromeExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let view = cx.entity();
        let search_view = view.clone();
        let submit_view = view.clone();
        let source_view = view.clone();
        let menu_view = view.clone();
        let refresh_view = view.clone();
        let outline_view = view.clone();

        window.set_native_toolbar(
            NativeToolbar::new()
                .display_mode(NativeToolbarDisplayMode::IconAndLabel)
                .shows_title(false)
                .item(NativeToolbarItem::Button(
                    NativeToolbarButton::new("native-window-actions", "ellipsis.circle", "Actions")
                        .on_activate(move |window, cx| {
                            menu_view.update(cx, |this, cx| {
                                this.menu_opens += 1;
                                this.last_action = "Opened toolbar actions menu".into();
                                cx.notify();
                            });

                            let _ = window.show_native_menu(
                                NativeMenu::new(NativeAnchor::ToolbarItem(
                                    "native-window-actions".into(),
                                ))
                                .kind(NativeMenuKind::Popup)
                                .item(
                                    NativeMenuItem::new("refresh-index", "Refresh Index")
                                        .symbol("arrow.clockwise")
                                        .on_activate({
                                            let refresh_view = refresh_view.clone();
                                            move |_window, cx| {
                                                refresh_view.update(cx, |this, cx| {
                                                    this.last_action =
                                                        "Refreshed the native index".into();
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                )
                                .item(NativeMenuItem::separator())
                                .item(
                                    NativeMenuItem::new("toggle-outline", "Toggle Outline")
                                        .symbol("sidebar.left")
                                        .on_activate({
                                            let outline_view = outline_view.clone();
                                            move |_window, cx| {
                                                outline_view.update(cx, |this, cx| {
                                                    this.last_action =
                                                        "Toggled native outline host".into();
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                ),
                            );
                        }),
                ))
                .item(NativeToolbarItem::FlexibleSpace)
                .item(NativeToolbarItem::Group(
                    NativeToolbarGroup::new("native-window-sources")
                        .item(NativeToolbarGroupItem::new("sidebar.left", "Project"))
                        .item(NativeToolbarGroupItem::new(
                            "doc.text.magnifyingglass",
                            "Search",
                        ))
                        .item(NativeToolbarGroupItem::new(
                            "clock.arrow.circlepath",
                            "History",
                        ))
                        .on_activate(move |event: NativeToolbarGroupEvent, _window, cx| {
                            source_view.update(cx, |this, cx| {
                                this.source_mode = match event.selected_index {
                                    0 => "Project".into(),
                                    1 => "Search".into(),
                                    _ => "History".into(),
                                };
                                cx.notify();
                            });
                        }),
                ))
                .item(NativeToolbarItem::FlexibleSpace)
                .item(NativeToolbarItem::SearchField(
                    NativeToolbarSearchField::new("native-window-search")
                        .placeholder("Jump to a file or symbol")
                        .on_change(move |event: NativeSearchEvent, _window, cx| {
                            search_view.update(cx, |this, cx| {
                                this.toolbar_query = event.text;
                                cx.notify();
                            });
                        })
                        .on_submit(move |event: NativeSearchEvent, _window, cx| {
                            submit_view.update(cx, |this, cx| {
                                this.toolbar_query = event.text;
                                this.last_action = "Submitted toolbar search".into();
                                cx.notify();
                            });
                        }),
                )),
        );

        Self {
            source_mode: "Project".into(),
            toolbar_query: "native_controls.rs".into(),
            last_action: "Toolbar installed".into(),
            menu_opens: 0,
        }
    }

    fn focus_toolbar_search(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.focus_native_search_item("native-window-search") {
            self.last_action = "Focused toolbar search field".into();
            cx.notify();
        }
    }

    fn show_search_suggestions(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let _ = window.show_native_menu(
            NativeMenu::new(NativeAnchor::ToolbarItem("native-window-search".into()))
                .kind(NativeMenuKind::Suggestions)
                .item(
                    NativeMenuItem::new("suggestion-tab-strip", "tab_strip.rs")
                        .symbol("doc.text")
                        .on_activate(|_window, _cx| {}),
                )
                .item(
                    NativeMenuItem::new("suggestion-navigation", "browser_view/navigation.rs")
                        .symbol("magnifyingglass")
                        .on_activate(|_window, _cx| {}),
                ),
        );

        self.last_action = "Opened native suggestions menu".into();
        cx.notify();
    }
}

impl Render for NativeWindowChromeExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        shell(
            "Pure native window chrome",
            "This example stays on the core-native path for toolbar items, toolbar search, popup menus, and the content controls that drive them.",
        )
        .id("native-window-chrome-scroll")
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(stat_tile("Source mode", self.source_mode.clone()))
                .child(stat_tile("Toolbar query", self.toolbar_query.clone()))
                .child(stat_tile("Menus opened", format!("{}", self.menu_opens))),
        )
        .child(
            panel(
                "Window-native chrome",
                "The toolbar, grouped source selector, search field, and actions menu are all installed through mozui's core native window API.",
            )
            .child(
                native_button("native-window-focus-search", "Focus Toolbar Search")
                    .button_style(mozui::NativeButtonStyle::Inline)
                    .on_click(cx.listener(Self::focus_toolbar_search)),
            )
            .child(
                native_button("native-window-open-suggestions", "Show Suggestions")
                    .button_style(mozui::NativeButtonStyle::Inline)
                    .on_click(cx.listener(Self::show_search_suggestions)),
            )
            .child(
                NativeTextField::label(
                    "native-window-status",
                    format!(
                        "Last action: {} | Active source: {} | Query: {}",
                        self.last_action, self.source_mode, self.toolbar_query
                    ),
                )
                .font_size(12.0)
                .bezeled(false),
            )
            .child(
                native_progress("native-window-activity")
                    .range(0.0, 10.0)
                    .value(self.menu_opens as f64),
            ),
        )
    }
}
