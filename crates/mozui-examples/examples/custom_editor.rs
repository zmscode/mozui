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
use support::{labeled_control, panel, run_rooted_example, shell, stat_tile};

fn main() {
    run_rooted_example(
        "Custom Editor",
        ThemeMode::Dark,
        size(px(980.0), px(800.0)),
        |window, cx| cx.new(|cx| CustomEditorExample::new(window, cx)),
    );
}

struct CustomEditorExample {
    title: Entity<InputState>,
    notes: Entity<InputState>,
    review_threshold: Entity<SliderState>,
    auto_save: bool,
    live_preview: bool,
    publish_progress: f32,
    _subscriptions: Vec<Subscription>,
}

impl CustomEditorExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Document title")
                .default_value("Native Controls Migration Plan")
        });
        let notes = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(10)
                .default_value(
                    "Phase notes:\n- Move callbacks into mozui core\n- Keep component semantics stable\n- Validate runtime toolbar search behavior\n",
                )
        });
        let review_threshold = cx.new(|_| {
            SliderState::new()
                .min(40.0)
                .max(100.0)
                .step(5.0)
                .default_value(80.0)
        });

        let subscriptions = vec![
            cx.observe(&title, |_, _, cx| cx.notify()),
            cx.observe(&notes, |_, _, cx| cx.notify()),
            cx.subscribe(&review_threshold, |_, _, _: &SliderEvent, cx| cx.notify()),
        ];

        Self {
            title,
            notes,
            review_threshold,
            auto_save: true,
            live_preview: true,
            publish_progress: 54.0,
            _subscriptions: subscriptions,
        }
    }

    fn toggle_auto_save(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.auto_save = *checked;
        cx.notify();
    }

    fn toggle_preview(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.live_preview = *checked;
        cx.notify();
    }

    fn publish_draft(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.publish_progress = (self.publish_progress + 11.0).min(100.0);
        cx.notify();
    }

    fn restore_brief(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.publish_progress = 54.0;
        self.auto_save = true;
        self.live_preview = true;
        self.title.update(cx, |state, cx| {
            state.set_value("Native Controls Migration Plan", window, cx);
        });
        self.notes.update(cx, |state, cx| {
            state.set_value(
                "Phase notes:\n- Move callbacks into mozui core\n- Keep component semantics stable\n- Validate runtime toolbar search behavior\n",
                window,
                cx,
            );
        });
        self.review_threshold.update(cx, |state, cx| {
            state.set_value(80.0, window, cx);
        });
        cx.notify();
    }
}

impl Render for CustomEditorExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self.title.read(cx).value();
        let notes = self.notes.read(cx).value();
        let review_threshold = match self.review_threshold.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, value) => value,
        };

        shell(
            "Pure mozui-components",
            "A second custom-only example focused on multiline editing, semantic actions, and richer component-only behavior.",
        )
        .id("custom-editor-scroll")
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(stat_tile("Title", title))
                .child(stat_tile("Review threshold", format!("{review_threshold:.0}%")))
                .child(stat_tile("Draft length", format!("{} chars", notes.len()))),
        )
        .child(
            panel(
                "Editor Surface",
                "This stays intentionally on the custom path because multiline editing is not yet modeled by the native leaf backend.",
            )
            .child(labeled_control(
                "Title",
                "Single-line custom input that keeps the semantic component path.",
                Input::new(&self.title),
            ))
            .child(labeled_control(
                "Draft Notes",
                "Multiline input is a clear custom-only case today.",
                Input::new(&self.notes).h(px(220.0)),
            )),
        )
        .child(
            panel(
                "Publish Controls",
                "This panel combines custom switches, slider tuning, and semantic action buttons.",
            )
            .child(labeled_control(
                "Auto Save",
                "Keep the draft synced in the background.",
                Switch::new("custom-editor-autosave")
                    .checked(self.auto_save)
                    .label("Write changes every few seconds")
                    .on_click(cx.listener(Self::toggle_auto_save)),
            ))
            .child(labeled_control(
                "Live Preview",
                "Keep the preview surface updating while typing.",
                Switch::new("custom-editor-preview")
                    .checked(self.live_preview)
                    .label("Update preview continuously")
                    .on_click(cx.listener(Self::toggle_preview)),
            ))
            .child(labeled_control(
                "Review Threshold",
                "Custom slider used for a semantic quality gate.",
                Slider::new(&self.review_threshold).horizontal(),
            ))
            .child(labeled_control(
                "Publish Progress",
                "Custom progress is still the right fit for this component-only surface.",
                Progress::new("custom-editor-progress")
                    .small()
                    .value(self.publish_progress),
            ))
            .child(
                div()
                    .flex()
                    .gap(px(10.0))
                    .child(
                        Button::new("custom-editor-publish")
                            .label("Publish Draft")
                            .primary()
                            .on_click(cx.listener(Self::publish_draft)),
                    )
                    .child(
                        Button::new("custom-editor-restore")
                            .label("Restore Brief")
                            .ghost()
                            .on_click(cx.listener(Self::restore_brief)),
                    ),
            ),
        )
    }
}
