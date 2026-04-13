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
        "Mixed Compare",
        ThemeMode::Light,
        size(px(1040.0), px(860.0)),
        |window, cx| cx.new(|cx| MixedCompareExample::new(window, cx)),
    );
}

struct MixedCompareExample {
    custom_input: Entity<InputState>,
    native_input: Entity<InputState>,
    custom_slider: Entity<SliderState>,
    native_slider: Entity<SliderState>,
    custom_switch: bool,
    native_switch: bool,
    shared_progress: f32,
    button_taps: usize,
    _subscriptions: Vec<Subscription>,
}

impl MixedCompareExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let custom_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Custom input")
                .default_value("semantic custom path")
        });
        let native_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Native input")
                .default_value("semantic native path")
        });
        let custom_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(5.0)
                .default_value(42.0)
        });
        let native_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(5.0)
                .default_value(68.0)
        });

        let subscriptions = vec![
            cx.observe(&custom_input, |_, _, cx| cx.notify()),
            cx.observe(&native_input, |_, _, cx| cx.notify()),
            cx.subscribe(&custom_slider, |_, _, _: &SliderEvent, cx| cx.notify()),
            cx.subscribe(&native_slider, |_, _, _: &SliderEvent, cx| cx.notify()),
        ];

        Self {
            custom_input,
            native_input,
            custom_slider,
            native_slider,
            custom_switch: true,
            native_switch: true,
            shared_progress: 34.0,
            button_taps: 0,
            _subscriptions: subscriptions,
        }
    }

    fn tap_shared_button(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.button_taps += 1;
        self.shared_progress = (self.shared_progress + 8.0).min(100.0);
        cx.notify();
    }

    fn set_custom_switch(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.custom_switch = *checked;
        cx.notify();
    }

    fn set_native_switch(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.native_switch = *checked;
        cx.notify();
    }
}

impl Render for MixedCompareExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let custom_slider = match self.custom_slider.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, value) => value,
        };
        let native_slider = match self.native_slider.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, value) => value,
        };

        shell(
            "Mixed semantic comparison",
            "This example keeps the mozui-components API stable while flipping specific controls onto the native backend with `.native()`.",
        )
        .id("mixed-compare-scroll")
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(stat_tile("Shared taps", format!("{}", self.button_taps)))
                .child(stat_tile("Custom slider", format!("{custom_slider:.0}%")))
                .child(stat_tile("Native slider", format!("{native_slider:.0}%"))),
        )
        .child(
            panel(
                "Custom semantic path",
                "These controls render through mozui-components without opting into the native backend.",
            )
            .child(labeled_control(
                "Button",
                "Semantic custom button.",
                Button::new("mixed-compare-custom-button")
                    .label("Tap shared counter")
                    .primary()
                    .on_click(cx.listener(Self::tap_shared_button)),
            ))
            .child(labeled_control(
                "Input",
                "Custom semantic text field.",
                Input::new(&self.custom_input).small(),
            ))
            .child(labeled_control(
                "Switch",
                "Component switch using the custom visual language.",
                Switch::new("mixed-compare-custom-switch")
                    .checked(self.custom_switch)
                    .label("Enable custom accent path")
                    .on_click(cx.listener(Self::set_custom_switch)),
            ))
            .child(labeled_control(
                "Slider",
                "Custom slider path with mozui-managed visuals.",
                Slider::new(&self.custom_slider).horizontal(),
            ))
            .child(labeled_control(
                "Progress",
                "Custom progress bar tracking shared work.",
                Progress::new("mixed-compare-custom-progress")
                    .small()
                    .value(self.shared_progress),
            )),
        )
        .child(
            panel(
                "Native semantic path",
                "The API is still `mozui-components`, but these controls opt into the core native backend.",
            )
            .child(labeled_control(
                "Button",
                "Same semantic button API with `.native()` enabled.",
                Button::new("mixed-compare-native-button")
                    .label("Tap shared counter")
                    .primary()
                    .native()
                    .on_click(cx.listener(Self::tap_shared_button)),
            ))
            .child(labeled_control(
                "Input",
                "Single-line input on the native text field backend.",
                Input::new(&self.native_input).small().native(),
            ))
            .child(labeled_control(
                "Switch",
                "Semantic switch delegated to the native switch backend.",
                Switch::new("mixed-compare-native-switch")
                    .checked(self.native_switch)
                    .label("Enable native backend")
                    .native()
                    .on_click(cx.listener(Self::set_native_switch)),
            ))
            .child(labeled_control(
                "Slider",
                "Horizontal single-value slider on the native backend.",
                Slider::new(&self.native_slider).horizontal().native(),
            ))
            .child(labeled_control(
                "Progress",
                "Shared work meter rendered through the native progress backend.",
                Progress::new("mixed-compare-native-progress")
                    .small()
                    .value(self.shared_progress)
                    .native(),
            )),
        )
    }
}
