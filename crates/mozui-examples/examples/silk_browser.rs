mod support;

use mozui::prelude::*;
use mozui::{
    AnyElement, App, ClickEvent, Context, Entity, Hsla, NativeAnchor, NativeMenu, NativeMenuItem,
    NativeMenuKind, NativeSearchEvent, NativeToolbar, NativeToolbarButton,
    NativeToolbarDisplayMode, NativeToolbarGroup, NativeToolbarGroupEvent, NativeToolbarGroupItem,
    NativeToolbarItem, NativeToolbarSearchField, SharedString, Subscription, Window, div, hsla, px,
    size,
};
use mozui_components::{
    Sizable, StyledExt as _,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    progress::Progress,
    slider::{Slider, SliderEvent, SliderState, SliderValue},
    switch::Switch,
    theme::ThemeMode,
};
use mozui_webview::WebView;
use support::run_transparent_rooted_example;
use wry::WebViewBuilder;

fn main() {
    run_transparent_rooted_example(
        "Silk Browser",
        ThemeMode::Dark,
        size(px(1456.0), px(948.0)),
        |window, cx| cx.new(|cx| SilkBrowserExample::new(window, cx)),
    );
}

#[derive(Clone)]
struct BrowserTab {
    title: SharedString,
    caption: SharedString,
    url: SharedString,
    tint: Hsla,
    pinned: bool,
}

struct SilkBrowserExample {
    webview: Entity<WebView>,
    profile_name: Entity<InputState>,
    tab_density: Entity<SliderState>,
    tabs: Vec<BrowserTab>,
    selected_tab: usize,
    sidebar_collapsed: bool,
    settings_open: bool,
    compact_sidebar: bool,
    reader_ready: bool,
    focus_mode: bool,
    sync_progress: f32,
    active_space: SharedString,
    active_collection: SharedString,
    current_url: SharedString,
    address_draft: SharedString,
    last_native_action: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl SilkBrowserExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_background_appearance(mozui::WindowBackgroundAppearance::Blurred);

        let tabs = initial_tabs();
        let current_url = tabs[0].url.clone();
        let initial_url = current_url.to_string();

        let webview = cx.new(|cx| {
            let builder = WebViewBuilder::new().with_url(initial_url);
            let webview = builder
                .build_as_child(window)
                .expect("failed to build Wry child webview");

            WebView::new(webview, window, cx)
        });

        let profile_name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Profile")
                .default_value("Silk / Browser Workspace")
        });
        let tab_density = cx.new(|_| {
            SliderState::new()
                .min(24.0)
                .max(40.0)
                .step(1.0)
                .default_value(28.0)
        });

        let subscriptions = vec![
            cx.observe(&profile_name, |_, _, cx| cx.notify()),
            cx.subscribe(&tab_density, |_, _, _: &SliderEvent, cx| cx.notify()),
        ];

        let view = cx.entity();
        Self::install_native_chrome(window, view.clone(), current_url.clone());

        Self {
            webview,
            profile_name,
            tab_density,
            tabs,
            selected_tab: 0,
            sidebar_collapsed: false,
            settings_open: false,
            compact_sidebar: false,
            reader_ready: true,
            focus_mode: false,
            sync_progress: 0.72,
            active_space: "Today".into(),
            active_collection: "Browser".into(),
            current_url: current_url.clone(),
            address_draft: current_url,
            last_native_action: "Native browser chrome ready".into(),
            _subscriptions: subscriptions,
        }
    }

    fn install_native_chrome(window: &mut Window, view: Entity<Self>, current_url: SharedString) {
        let placeholder = toolbar_placeholder(&current_url);

        let sidebar_view = view.clone();
        let back_view = view.clone();
        let forward_view = view.clone();
        let reload_view = view.clone();
        let collection_view = view.clone();
        let search_change_view = view.clone();
        let search_submit_view = view.clone();
        let settings_view = view.clone();

        window.set_native_toolbar(
            NativeToolbar::new()
                .display_mode(NativeToolbarDisplayMode::IconOnly)
                .shows_title(false)
                .item(NativeToolbarItem::Button(
                    NativeToolbarButton::new("silk-browser-sidebar", "sidebar.left", "Sidebar")
                        .on_activate(move |window, cx| {
                            sidebar_view.update(cx, |this, cx| {
                                this.sidebar_collapsed = !this.sidebar_collapsed;
                                this.last_native_action = if this.sidebar_collapsed {
                                    "Collapsed Glass-style sidebar".into()
                                } else {
                                    "Expanded Glass-style sidebar".into()
                                };
                                cx.notify();
                            });
                            let _ = window.focus_native_search_item("silk-browser-address");
                        }),
                ))
                .item(NativeToolbarItem::Button(
                    NativeToolbarButton::new("silk-browser-back", "chevron.left", "Back")
                        .navigational(true)
                        .on_activate(move |_window, cx| {
                            back_view.update(cx, |this, cx| {
                                this.webview.update(cx, |webview, _| {
                                    let _ = webview.back();
                                });
                                this.last_native_action = "Went back".into();
                                cx.notify();
                            });
                        }),
                ))
                .item(NativeToolbarItem::Button(
                    NativeToolbarButton::new("silk-browser-forward", "chevron.right", "Forward")
                        .navigational(true)
                        .on_activate(move |_window, cx| {
                            forward_view.update(cx, |this, cx| {
                                this.webview.update(cx, |webview, _| {
                                    let _ = webview.raw().evaluate_script("history.forward();");
                                });
                                this.last_native_action = "Went forward".into();
                                cx.notify();
                            });
                        }),
                ))
                .item(NativeToolbarItem::Button(
                    NativeToolbarButton::new("silk-browser-reload", "arrow.clockwise", "Reload")
                        .on_activate(move |_window, cx| {
                            reload_view.update(cx, |this, cx| {
                                this.webview.update(cx, |webview, _| {
                                    let _ = webview.raw().evaluate_script("location.reload();");
                                });
                                this.last_native_action = "Reloaded current page".into();
                                cx.notify();
                            });
                        }),
                ))
                .item(NativeToolbarItem::Group(
                    NativeToolbarGroup::new("silk-browser-collection")
                        .item(NativeToolbarGroupItem::new("globe", "Browser"))
                        .item(NativeToolbarGroupItem::new("book", "Library"))
                        .item(NativeToolbarGroupItem::new("clock", "History"))
                        .on_activate(move |event: NativeToolbarGroupEvent, _window, cx| {
                            collection_view.update(cx, |this, cx| {
                                this.active_collection = match event.selected_index {
                                    1 => "Library".into(),
                                    2 => "History".into(),
                                    _ => "Browser".into(),
                                };
                                this.last_native_action =
                                    format!("Switched to {} collection", this.active_collection)
                                        .into();
                                cx.notify();
                            });
                        }),
                ))
                .item(NativeToolbarItem::FlexibleSpace)
                .item(NativeToolbarItem::SearchField(
                    NativeToolbarSearchField::new("silk-browser-address")
                        .placeholder(placeholder)
                        .on_change(move |event: NativeSearchEvent, window, cx| {
                            let query = event.text.clone();
                            let suggestions_view = search_change_view.clone();
                            search_change_view.update(cx, |this, cx| {
                                this.handle_address_change(
                                    query.clone(),
                                    window,
                                    cx,
                                    suggestions_view.clone(),
                                );
                            });
                        })
                        .on_submit(move |event: NativeSearchEvent, window, cx| {
                            let submitted = event.text.clone();
                            search_submit_view.update(cx, |this, cx| {
                                this.navigate_to(submitted.clone(), window, cx);
                            });
                        }),
                ))
                .item(NativeToolbarItem::FlexibleSpace)
                .item(NativeToolbarItem::Button(
                    NativeToolbarButton::new(
                        "silk-browser-settings",
                        "slider.horizontal.3",
                        "Settings",
                    )
                    .on_activate(move |_window, cx| {
                        settings_view.update(cx, |this, cx| {
                            this.settings_open = !this.settings_open;
                            this.last_native_action = if this.settings_open {
                                "Opened custom settings overlay".into()
                            } else {
                                "Closed custom settings overlay".into()
                            };
                            cx.notify();
                        });
                    }),
                )),
        );
    }

    fn handle_address_change(
        &mut self,
        query: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
        view: Entity<Self>,
    ) {
        self.address_draft = query.clone();
        self.last_native_action = "Edited native address field".into();

        let query = query.trim().to_string();
        if query.is_empty() {
            cx.notify();
            return;
        }

        let mut menu = NativeMenu::new(NativeAnchor::ToolbarItem("silk-browser-address".into()))
            .kind(NativeMenuKind::Suggestions)
            .item(
                NativeMenuItem::new(
                    "address-open",
                    format!("Open {}", short_url(&normalize_address(&query))),
                )
                .symbol("globe")
                .on_activate({
                    let view = view.clone();
                    let normalized = normalize_address(&query);
                    move |window, cx| {
                        view.update(cx, |this, cx| {
                            this.navigate_to(normalized.clone(), window, cx);
                        });
                    }
                }),
            );

        let mut added_matches = false;
        for (ix, tab) in self.tabs.iter().enumerate() {
            let title = tab.title.to_string().to_lowercase();
            let caption = tab.caption.to_string().to_lowercase();
            let url = tab.url.to_string().to_lowercase();
            let query_lower = query.to_lowercase();
            if !(title.contains(&query_lower)
                || caption.contains(&query_lower)
                || url.contains(&query_lower))
            {
                continue;
            }

            if !added_matches {
                menu = menu.item(NativeMenuItem::separator());
                added_matches = true;
            }

            let target_url = tab.url.clone();
            menu = menu.item(
                NativeMenuItem::new(
                    format!("tab-suggestion-{ix}"),
                    format!("{}  ·  {}", tab.title, short_url(&target_url)),
                )
                .symbol("rectangle.stack")
                .on_activate({
                    let view = view.clone();
                    move |window, cx| {
                        view.update(cx, |this, cx| {
                            this.selected_tab = ix;
                            this.navigate_to(target_url.clone(), window, cx);
                        });
                    }
                }),
            );
        }

        let _ = window.show_native_menu(menu);
        cx.notify();
    }

    fn navigate_to(
        &mut self,
        destination: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let normalized = normalize_address(destination.into().as_ref());
        let label = title_from_url(&normalized);
        self.current_url = normalized.clone().into();
        self.address_draft = self.current_url.clone();
        self.last_native_action = format!("Loaded {}", short_url(&normalized)).into();

        if let Some(tab) = self.tabs.get_mut(self.selected_tab) {
            tab.url = normalized.clone().into();
            tab.title = label.into();
            tab.caption = short_url(&normalized).into();
        }

        self.webview.update(cx, |webview, _| {
            webview.load_url(&normalized);
        });

        Self::install_native_chrome(window, cx.entity().clone(), self.current_url.clone());
        cx.notify();
    }

    fn tab_density_value(&self, cx: &App) -> f32 {
        match self.tab_density.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, value) => value,
        }
    }

    fn open_new_tab(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let new_tab = BrowserTab {
            title: "New Tab".into(),
            caption: "glassapp.dev".into(),
            url: "https://glassapp.dev".into(),
            tint: hsla(187.0 / 360.0, 0.47, 0.55, 1.0),
            pinned: false,
        };
        self.tabs.insert(1, new_tab);
        self.selected_tab = 1;
        self.navigate_to("https://glassapp.dev", window, cx);
    }

    fn focus_search(&mut self, _: &ClickEvent, window: &mut Window, _cx: &mut Context<Self>) {
        let _ = window.focus_native_search_item("silk-browser-address");
    }

    fn toggle_settings_from_button(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings_open = !self.settings_open;
        cx.notify();
    }

    fn toggle_compact_sidebar(
        &mut self,
        checked: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.compact_sidebar = *checked;
        self.last_native_action = if *checked {
            "Enabled compact sidebar density".into()
        } else {
            "Restored default sidebar density".into()
        };
        cx.notify();
    }

    fn toggle_reader_ready(
        &mut self,
        checked: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.reader_ready = *checked;
        self.last_native_action = if *checked {
            "Reader mode previews enabled".into()
        } else {
            "Reader mode previews disabled".into()
        };
        cx.notify();
    }

    fn toggle_focus_mode(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.focus_mode = *checked;
        self.last_native_action = if *checked {
            "Focus mode enabled".into()
        } else {
            "Focus mode disabled".into()
        };
        cx.notify();
    }

    fn save_space(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.sync_progress = (self.sync_progress + 0.08).min(1.0);
        self.last_native_action = "Saved browser space snapshot".into();
        cx.notify();
    }
}

impl Render for SilkBrowserExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let density = self.tab_density_value(cx);
        let selected_tab = self.tabs[self.selected_tab].clone();
        let view = cx.entity().clone();
        let pinned_tabs: Vec<(usize, BrowserTab)> = self
            .tabs
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, tab)| tab.pinned)
            .collect();
        let recent_tabs: Vec<(usize, BrowserTab)> = self
            .tabs
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, tab)| !tab.pinned)
            .collect();

        div()
            .id("silk-browser-root")
            .size_full()
            .bg(glass_background())
            .overflow_hidden()
            .child(
                div()
                    .size_full()
                    .flex()
                    .min_h_0()
                    .child(
                        div()
                            .id("silk-browser-sidebar")
                            .h_full()
                            .w(if self.sidebar_collapsed {
                                px(70.0)
                            } else if self.compact_sidebar {
                                px(210.0)
                            } else {
                                px(236.0)
                            })
                            .flex_shrink_0()
                            .bg(glass_sidebar_background().opacity(0.96))
                            .border_r_1()
                            .border_color(glass_border())
                            .px(px(8.0))
                            .pt(px(52.0))
                            .pb(px(10.0))
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(sidebar_header(
                                self.active_space.clone(),
                                self.active_collection.clone(),
                                self.sidebar_collapsed,
                            ))
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .gap(px(8.0))
                                    .when(self.sidebar_collapsed, |this| {
                                        this.flex_col().items_center()
                                    })
                                    .child(
                                        div()
                                            .id("silk-browser-new-tab")
                                            .h(px(30.0))
                                            .rounded(px(10.0))
                                            .border_1()
                                            .border_color(glass_border())
                                            .bg(glass_surface())
                                            .px(px(10.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_xs()
                                            .text_color(glass_text())
                                            .child(if self.sidebar_collapsed {
                                                "＋"
                                            } else {
                                                "+ New Tab"
                                            })
                                            .hover(|style| style.bg(glass_hover_row()))
                                            .cursor_pointer()
                                            .on_click(cx.listener(Self::open_new_tab)),
                                    )
                                    .when(!self.sidebar_collapsed, |this| {
                                        this.child(
                                            div()
                                                .id("silk-browser-focus-search")
                                                .h(px(30.0))
                                                .rounded(px(10.0))
                                                .border_1()
                                                .border_color(glass_border())
                                                .bg(glass_surface())
                                                .px(px(10.0))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_xs()
                                                .text_color(glass_text())
                                                .child("Focus Search")
                                                .hover(|style| style.bg(glass_hover_row()))
                                                .cursor_pointer()
                                                .on_click(cx.listener(Self::focus_search)),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .id("silk-browser-tab-scroll")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .flex()
                                    .flex_col()
                                    .gap(px(10.0))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .when(!self.sidebar_collapsed, |this| {
                                                this.child(sidebar_section_label("Pinned"))
                                            })
                                            .children(pinned_tabs.into_iter().map(
                                                |(index, tab)| {
                                                    browser_tab_row(
                                                        view.clone(),
                                                        index,
                                                        tab,
                                                        index == self.selected_tab,
                                                        self.sidebar_collapsed,
                                                        density,
                                                    )
                                                },
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .when(!self.sidebar_collapsed, |this| {
                                                this.child(sidebar_section_label("Today"))
                                            })
                                            .children(recent_tabs.into_iter().map(
                                                |(index, tab)| {
                                                    browser_tab_row(
                                                        view.clone(),
                                                        index,
                                                        tab,
                                                        index == self.selected_tab,
                                                        self.sidebar_collapsed,
                                                        density,
                                                    )
                                                },
                                            )),
                                    ),
                            )
                            .child(sidebar_footer(
                                self.profile_name.read(cx).value(),
                                self.sidebar_collapsed,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .relative()
                            .bg(glass_background())
                            .child(
                                div()
                                    .size_full()
                                    .bg(glass_background())
                                    .child(self.webview.clone()),
                            )
                            .when(self.settings_open, |this| {
                                this.child(div().absolute().top(px(52.0)).right(px(18.0)).child(
                                    settings_overlay(
                                        &self.profile_name,
                                        &self.tab_density,
                                        self.compact_sidebar,
                                        self.reader_ready,
                                        self.focus_mode,
                                        self.sync_progress,
                                        selected_tab.tint.opacity(0.78),
                                        cx,
                                    ),
                                ))
                            }),
                    ),
            )
    }
}

fn initial_tabs() -> Vec<BrowserTab> {
    vec![
        BrowserTab {
            title: "Glass".into(),
            caption: "glassapp.dev".into(),
            url: "https://glassapp.dev".into(),
            tint: hsla(207.8 / 360.0, 0.81, 0.66, 1.0),
            pinned: true,
        },
        BrowserTab {
            title: "Browser".into(),
            caption: "glassapp.dev".into(),
            url: "https://glassapp.dev".into(),
            tint: hsla(187.0 / 360.0, 0.47, 0.55, 1.0),
            pinned: true,
        },
        BrowserTab {
            title: "Rust".into(),
            caption: "rust-lang.org".into(),
            url: "https://www.rust-lang.org".into(),
            tint: hsla(29.0 / 360.0, 0.54, 0.61, 1.0),
            pinned: false,
        },
        BrowserTab {
            title: "Mozilla".into(),
            caption: "mozilla.org".into(),
            url: "https://www.mozilla.org".into(),
            tint: hsla(355.0 / 360.0, 0.65, 0.65, 1.0),
            pinned: false,
        },
        BrowserTab {
            title: "WebGPU".into(),
            caption: "wgpu.rs".into(),
            url: "https://wgpu.rs".into(),
            tint: hsla(286.0 / 360.0, 0.51, 0.64, 1.0),
            pinned: false,
        },
        BrowserTab {
            title: "Zed".into(),
            caption: "zed.dev".into(),
            url: "https://zed.dev".into(),
            tint: hsla(95.0 / 360.0, 0.38, 0.62, 1.0),
            pinned: false,
        },
    ]
}

fn sidebar_header(
    active_space: SharedString,
    active_collection: SharedString,
    collapsed: bool,
) -> impl IntoElement {
    div()
        .w_full()
        .px(px(8.0))
        .py(px(6.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .when(collapsed, |this| this.items_center())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().w(px(18.0)).h(px(18.0)).rounded(px(5.0)).bg(hsla(
                    207.8 / 360.0,
                    0.81,
                    0.66,
                    0.92,
                )))
                .when(!collapsed, |this| {
                    this.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(glass_text())
                                    .child("Silk"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(glass_muted_text())
                                    .child(format!("{active_space} · {active_collection}")),
                            ),
                    )
                }),
        )
}

fn sidebar_section_label(label: &'static str) -> impl IntoElement {
    div()
        .w_full()
        .px(px(8.0))
        .text_xs()
        .font_semibold()
        .text_color(glass_muted_text())
        .child(label)
}

fn sidebar_footer(profile: SharedString, collapsed: bool) -> impl IntoElement {
    div()
        .w_full()
        .px(px(8.0))
        .py(px(8.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .when(collapsed, |this| this.justify_center())
        .child(div().w(px(22.0)).h(px(22.0)).rounded(px(999.0)).bg(hsla(
            286.0 / 360.0,
            0.51,
            0.64,
            0.48,
        )))
        .when(!collapsed, |this| {
            this.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .text_color(glass_text())
                    .child(profile),
            )
        })
}

fn browser_tab_row(
    view: Entity<SilkBrowserExample>,
    index: usize,
    tab: BrowserTab,
    selected: bool,
    collapsed: bool,
    density: f32,
) -> AnyElement {
    let url = tab.url.clone();
    let title = tab.title.clone();
    let action_title = title.clone();
    let tint = tab.tint;
    let pinned = tab.pinned;
    let height = if density < 28.0 { px(26.0) } else { px(28.0) };

    div()
        .id(SharedString::from(format!("silk-browser-tab-{index}")))
        .w_full()
        .h(height)
        .px(px(8.0))
        .rounded(px(8.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .bg(if selected {
            glass_selected_row()
        } else {
            hsla(0.0, 0.0, 0.0, 0.0)
        })
        .hover(|style| style.bg(glass_hover_row()))
        .cursor_pointer()
        .child(
            div()
                .w(px(14.0))
                .h(px(14.0))
                .rounded(px(4.0))
                .bg(if selected { tint } else { tint.opacity(0.78) }),
        )
        .when(!collapsed, |this| {
            this.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_xs()
                    .text_color(if selected {
                        glass_text()
                    } else {
                        glass_muted_text()
                    })
                    .child(title),
            )
            .child(
                div()
                    .w(px(8.0))
                    .h(px(8.0))
                    .rounded(px(999.0))
                    .bg(if pinned {
                        glass_muted_text().opacity(0.80)
                    } else {
                        hsla(0.0, 0.0, 0.0, 0.0)
                    }),
            )
        })
        .on_click(move |_, window, cx| {
            view.update(cx, |this, cx| {
                this.selected_tab = index;
                this.last_native_action = format!("Opened {action_title}").into();
                this.navigate_to(url.clone(), window, cx);
            });
        })
        .into_any_element()
}

fn settings_overlay(
    profile_name: &Entity<InputState>,
    tab_density: &Entity<SliderState>,
    compact_sidebar: bool,
    reader_ready: bool,
    focus_mode: bool,
    sync_progress: f32,
    tint: Hsla,
    cx: &mut Context<SilkBrowserExample>,
) -> impl IntoElement {
    div()
        .id("silk-browser-settings-overlay")
        .w(px(320.0))
        .rounded(px(18.0))
        .border_1()
        .border_color(glass_border())
        .bg(glass_surface().opacity(0.98))
        .flex()
        .flex_col()
        .gap(px(14.0))
        .px(px(16.0))
        .py(px(16.0))
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(10.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .text_color(glass_text())
                                .child("Settings"),
                        )
                        .child(div().text_xs().text_color(glass_muted_text()).child(
                            "Custom mozui-components controls layered over native browser chrome.",
                        )),
                )
                .child(div().w(px(10.0)).h(px(10.0)).rounded(px(999.0)).bg(tint)),
        )
        .child(glass_setting_block(
            "Profile",
            Input::new(profile_name).small().into_any_element(),
        ))
        .child(glass_setting_block(
            "Tab Density",
            Slider::new(tab_density).horizontal().into_any_element(),
        ))
        .child(glass_setting_block(
            "Compact Sidebar",
            Switch::new("silk-browser-compact-sidebar")
                .checked(compact_sidebar)
                .label("Reduce sidebar width and row density")
                .on_click(cx.listener(SilkBrowserExample::toggle_compact_sidebar))
                .into_any_element(),
        ))
        .child(glass_setting_block(
            "Reader Ready",
            Switch::new("silk-browser-reader-ready")
                .checked(reader_ready)
                .label("Prefer calm pages and lighter chrome")
                .on_click(cx.listener(SilkBrowserExample::toggle_reader_ready))
                .into_any_element(),
        ))
        .child(glass_setting_block(
            "Focus Mode",
            Switch::new("silk-browser-focus-mode")
                .checked(focus_mode)
                .label("Dim distractions around the active page")
                .on_click(cx.listener(SilkBrowserExample::toggle_focus_mode))
                .into_any_element(),
        ))
        .child(glass_setting_block(
            "Space Sync",
            Progress::new("silk-browser-sync-progress")
                .small()
                .value(sync_progress)
                .into_any_element(),
        ))
        .child(
            div()
                .flex()
                .gap(px(10.0))
                .child(
                    Button::new("silk-browser-save-space")
                        .label("Save Space")
                        .primary()
                        .on_click(cx.listener(SilkBrowserExample::save_space)),
                )
                .child(
                    Button::new("silk-browser-close-settings")
                        .label("Done")
                        .secondary()
                        .on_click(cx.listener(SilkBrowserExample::toggle_settings_from_button)),
                ),
        )
}

fn glass_setting_block(label: &'static str, control: AnyElement) -> impl IntoElement {
    div()
        .rounded(px(12.0))
        .border_1()
        .border_color(glass_border())
        .bg(glass_editor_surface())
        .px(px(12.0))
        .py(px(12.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .text_xs()
                .font_semibold()
                .text_color(glass_text())
                .child(label),
        )
        .child(control)
}

fn normalize_address(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.contains("://") {
        trimmed.to_string()
    } else if trimmed.contains('.') && !trimmed.contains(' ') {
        format!("https://{trimmed}")
    } else {
        format!(
            "https://www.google.com/search?q={}",
            trimmed.replace(' ', "+")
        )
    }
}

fn short_url(value: &str) -> String {
    value
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string()
}

fn title_from_url(value: &str) -> String {
    short_url(value)
        .split('/')
        .next()
        .unwrap_or("Page")
        .split('.')
        .next()
        .unwrap_or("Page")
        .to_string()
}

fn toolbar_placeholder(current_url: &str) -> SharedString {
    format!("Search or enter address · {}", short_url(current_url)).into()
}

fn glass_background() -> Hsla {
    hsla(215.0 / 360.0, 0.12, 0.15, 1.0)
}

fn glass_sidebar_background() -> Hsla {
    hsla(220.0 / 360.0, 0.12, 0.18, 1.0)
}

fn glass_editor_surface() -> Hsla {
    hsla(220.0 / 360.0, 0.12, 0.18, 1.0)
}

fn glass_surface() -> Hsla {
    hsla(225.0 / 360.0, 0.12, 0.17, 1.0)
}

fn glass_border() -> Hsla {
    hsla(225.0 / 360.0, 0.13, 0.12, 1.0)
}

fn glass_hover_row() -> Hsla {
    hsla(225.0 / 360.0, 0.118, 0.267, 0.62)
}

fn glass_selected_row() -> Hsla {
    hsla(224.0 / 360.0, 0.113, 0.261, 1.0)
}

fn glass_text() -> Hsla {
    hsla(221.0 / 360.0, 0.11, 0.86, 1.0)
}

fn glass_muted_text() -> Hsla {
    hsla(218.0 / 360.0, 0.07, 0.46, 1.0)
}
