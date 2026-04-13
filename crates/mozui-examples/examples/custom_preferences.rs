mod support;

use mozui::prelude::*;
use mozui::{ClickEvent, Context, Entity, Subscription, Window, div, px, size};
use mozui_components::{
    Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    progress::Progress,
    slider::{Slider, SliderEvent, SliderState, SliderValue},
    switch::Switch,
    theme::ThemeMode,
};
use support::{labeled_control, panel, run_rooted_example, shell, slider_value_label, stat_tile};

fn main() {
    run_rooted_example(
        "Custom Preferences",
        ThemeMode::Light,
        size(px(960.0), px(760.0)),
        |window, cx| cx.new(|cx| CustomPreferencesExample::new(window, cx)),
    );
}

struct CustomPreferencesExample {
    workspace_name: Entity<InputState>,
    cache_size: Entity<SliderState>,
    notifications_enabled: bool,
    sync_on_cellular: bool,
    sync_progress: f32,
    _subscriptions: Vec<Subscription>,
}

impl CustomPreferencesExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspace_name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Workspace name")
                .default_value("Aurora Workspace")
        });
        let cache_size = cx.new(|_| {
            SliderState::new()
                .min(32.0)
                .max(512.0)
                .step(16.0)
                .default_value(128.0)
        });

        let subscriptions = vec![
            cx.observe(&workspace_name, |_, _, cx| cx.notify()),
            cx.subscribe(&cache_size, |_, _, _: &SliderEvent, cx| cx.notify()),
        ];

        Self {
            workspace_name,
            cache_size,
            notifications_enabled: true,
            sync_on_cellular: false,
            sync_progress: 38.0,
            _subscriptions: subscriptions,
        }
    }

    fn toggle_notifications(
        &mut self,
        checked: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.notifications_enabled = *checked;
        cx.notify();
    }

    fn toggle_cellular(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.sync_on_cellular = *checked;
        cx.notify();
    }

    fn run_sync(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.sync_progress = (self.sync_progress + 17.0).min(100.0);
        cx.notify();
    }

    fn reset_defaults(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.notifications_enabled = true;
        self.sync_on_cellular = false;
        self.sync_progress = 38.0;
        self.workspace_name.update(cx, |state, cx| {
            state.set_value("Aurora Workspace", window, cx);
        });
        self.cache_size.update(cx, |state, cx| {
            state.set_value(128.0, window, cx);
        });
        cx.notify();
    }
}

impl Render for CustomPreferencesExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace_name = self.workspace_name.read(cx).value();
        let cache_size = match self.cache_size.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, value) => value,
        };

        shell(
            "Pure mozui-components",
            "A custom-only preferences surface with semantic inputs, switches, buttons, and progress.",
        )
        .id("custom-preferences-scroll")
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(stat_tile("Workspace", workspace_name))
                .child(stat_tile("Cache budget", format!("{} MB", slider_value_label(cache_size))))
                .child(stat_tile("Sync", format!("{:.0}%", self.sync_progress))),
        )
        .child(
            panel(
                "Profile",
                "Everything in this window is rendered through mozui-components rather than the native backend.",
            )
            .child(labeled_control(
                "Workspace Name",
                "Single-line custom input with mozui-managed focus and styling.",
                Input::new(&self.workspace_name).small(),
            ))
            .child(labeled_control(
                "Desktop Notifications",
                "Component switch with the current theme styling applied.",
                Switch::new("custom-pref-notifications")
                    .checked(self.notifications_enabled)
                    .label("Notify when syncs complete")
                    .on_click(cx.listener(Self::toggle_notifications)),
            ))
            .child(labeled_control(
                "Cellular Sync",
                "A second semantic switch to show the shared custom interaction language.",
                Switch::new("custom-pref-cellular")
                    .checked(self.sync_on_cellular)
                    .label("Allow sync on metered networks")
                    .on_click(cx.listener(Self::toggle_cellular)),
            ))
            .child(labeled_control(
                "Offline Cache",
                "Custom slider with a wider range than the native leaf demos.",
                Slider::new(&self.cache_size).horizontal(),
            )),
        )
        .child(
            panel(
                "Sync Controls",
                "The action row and progress indicator stay fully inside mozui-components.",
            )
            .child(labeled_control(
                "Progress",
                "Custom progress tracks the background sync pipeline.",
                Progress::new("custom-pref-progress")
                    .small()
                    .value(self.sync_progress),
            ))
            .child(
                div()
                    .flex()
                    .gap(px(10.0))
                    .child(
                        Button::new("custom-pref-sync")
                            .label("Run Sync")
                            .primary()
                            .on_click(cx.listener(Self::run_sync)),
                    )
                    .child(
                        Button::new("custom-pref-reset")
                            .label("Reset Defaults")
                            .secondary()
                            .on_click(cx.listener(Self::reset_defaults)),
                    ),
            ),
        )
    }
}
