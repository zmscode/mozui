mod support;

use mozui::prelude::*;
use mozui::{
    ClickEvent, Context, Entity, NativeAnchor, NativeMenu, NativeMenuItem, NativeMenuKind,
    NativeSearchEvent, NativeToolbar, NativeToolbarButton, NativeToolbarDisplayMode,
    NativeToolbarItem, NativeToolbarSearchField, SharedString, Subscription, Window, div,
    native_button, px, size,
};
use mozui_components::{
    Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    progress::Progress,
    slider::{Slider, SliderEvent, SliderState, SliderValue},
    switch::Switch,
    theme::ThemeMode,
};
use support::{labeled_control, panel, run_rooted_example, shell, stat_tile};

fn main() {
    run_rooted_example(
        "Mixed Workspace",
        ThemeMode::Dark,
        size(px(1080.0), px(820.0)),
        |window, cx| cx.new(|cx| MixedWorkspaceExample::new(window, cx)),
    );
}

struct MixedWorkspaceExample {
    filter: Entity<InputState>,
    quality: Entity<SliderState>,
    live_preview: bool,
    build_progress: f32,
    toolbar_query: SharedString,
    last_toolbar_action: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl MixedWorkspaceExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Filter the document list")
                .default_value("native controls")
        });
        let quality = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(5.0)
                .default_value(72.0)
        });

        let subscriptions = vec![
            cx.observe(&filter, |_, _, cx| cx.notify()),
            cx.subscribe(&quality, |_, _, _: &SliderEvent, cx| cx.notify()),
        ];

        let view = cx.entity();
        let change_view = view.clone();
        let submit_view = view.clone();
        let action_view = view.clone();

        window.set_native_toolbar(
            NativeToolbar::new()
                .display_mode(NativeToolbarDisplayMode::IconAndLabel)
                .shows_title(false)
                .item(NativeToolbarItem::Button(
                    NativeToolbarButton::new(
                        "mixed-workspace-actions",
                        "wand.and.stars",
                        "Actions",
                    )
                    .on_activate(move |window, cx| {
                        action_view.update(cx, |this, cx| {
                            this.last_toolbar_action = "Opened native workspace actions".into();
                            cx.notify();
                        });

                        let action_focus_view = action_view.clone();
                        let _ = window.show_native_menu(
                            NativeMenu::new(NativeAnchor::ToolbarItem(
                                "mixed-workspace-actions".into(),
                            ))
                            .kind(NativeMenuKind::Popup)
                            .item(
                                NativeMenuItem::new("run-build", "Run Build")
                                    .symbol("hammer")
                                    .on_activate({
                                        let action_view = action_view.clone();
                                        move |_window, cx| {
                                            action_view.update(cx, |this, cx| {
                                                this.build_progress =
                                                    (this.build_progress + 14.0).min(100.0);
                                                this.last_toolbar_action =
                                                    "Triggered build from native menu".into();
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .item(
                                NativeMenuItem::new("focus-filter", "Focus Toolbar Search")
                                    .symbol("magnifyingglass")
                                    .on_activate(move |window, cx| {
                                        let _ = window
                                            .focus_native_search_item("mixed-workspace-search");
                                        action_focus_view.update(cx, |this, cx| {
                                            this.last_toolbar_action =
                                                "Focused toolbar search from menu".into();
                                            cx.notify();
                                        });
                                    }),
                            ),
                        );
                    }),
                ))
                .item(NativeToolbarItem::FlexibleSpace)
                .item(NativeToolbarItem::SearchField(
                    NativeToolbarSearchField::new("mixed-workspace-search")
                        .placeholder("Native toolbar search")
                        .on_change(move |event: NativeSearchEvent, _window, cx| {
                            change_view.update(cx, |this, cx| {
                                this.toolbar_query = event.text;
                                cx.notify();
                            });
                        })
                        .on_submit(move |event: NativeSearchEvent, _window, cx| {
                            submit_view.update(cx, |this, cx| {
                                this.toolbar_query = event.text;
                                this.last_toolbar_action = "Submitted toolbar search".into();
                                cx.notify();
                            });
                        }),
                )),
        );

        Self {
            filter,
            quality,
            live_preview: true,
            build_progress: 41.0,
            toolbar_query: "native()".into(),
            last_toolbar_action: "Toolbar ready".into(),
            _subscriptions: subscriptions,
        }
    }

    fn toggle_preview(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.live_preview = *checked;
        cx.notify();
    }

    fn focus_toolbar_search(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.focus_native_search_item("mixed-workspace-search") {
            self.last_toolbar_action = "Focused toolbar search from custom content".into();
            cx.notify();
        }
    }

    fn queue_build(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.build_progress = (self.build_progress + 9.0).min(100.0);
        cx.notify();
    }
}

impl Render for MixedWorkspaceExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let quality = match self.quality.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, value) => value,
        };

        shell(
            "Mixed custom + native workspace",
            "Custom content stays in mozui-components while the window chrome uses mozui's core-native toolbar and search path.",
        )
        .id("mixed-workspace-scroll")
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(stat_tile("Toolbar query", self.toolbar_query.clone()))
                .child(stat_tile("Preview", if self.live_preview { "On" } else { "Off" }))
                .child(stat_tile("Quality", format!("{quality:.0}%"))),
        )
        .child(
            panel(
                "Custom content region",
                "These controls are semantic mozui-components, with one inline native action mixed in on purpose.",
            )
            .child(labeled_control(
                "Filter List",
                "Custom semantic input in the content region.",
                Input::new(&self.filter).small(),
            ))
            .child(labeled_control(
                "Build Quality",
                "Custom slider controlling a semantic build threshold.",
                Slider::new(&self.quality).horizontal(),
            ))
            .child(labeled_control(
                "Live Preview",
                "Custom switch while the window chrome stays native.",
                Switch::new("mixed-workspace-preview")
                    .checked(self.live_preview)
                    .label("Update previews as source files change")
                    .on_click(cx.listener(Self::toggle_preview)),
            ))
            .child(labeled_control(
                "Build Progress",
                "Custom progress bar in the content layer.",
                Progress::new("mixed-workspace-progress")
                    .small()
                    .value(self.build_progress),
            ))
            .child(
                div()
                    .flex()
                    .gap(px(10.0))
                    .child(
                        Button::new("mixed-workspace-focus-search")
                            .label("Focus Native Search")
                            .secondary()
                            .on_click(cx.listener(Self::focus_toolbar_search)),
                    )
                    .child(
                        Button::new("mixed-workspace-queue-build")
                            .label("Queue Build")
                            .primary()
                            .on_click(cx.listener(Self::queue_build)),
                    )
                    .child(
                        native_button("mixed-workspace-inline-native", "Native Apply")
                            .button_style(mozui::NativeButtonStyle::Inline)
                            .on_click(cx.listener(Self::queue_build)),
                    ),
            ),
        )
        .child(
            panel(
                "Native chrome state",
                "The toolbar search field and native menu feed back into the custom content region through the framework event loop.",
            )
            .child(
                div()
                    .text_xs()
                    .text_color(mozui::hsla(0.0, 0.0, 1.0, 0.72))
                    .child(format!("Last toolbar action: {}", self.last_toolbar_action)),
            )
            .child(
                Button::new("mixed-workspace-native-progress")
                    .label("Native Progress Variant")
                    .ghost()
                    .native()
                    .on_click(cx.listener(Self::queue_build)),
            ),
        )
    }
}
