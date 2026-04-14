mod support;

use mozui::prelude::*;
use mozui::{ClickEvent, Context, Entity, Subscription, Window, div, px, size};
use mozui_components::{
    Sizable,
    button::{Button, ButtonVariants},
    progress::Progress,
    slider::{Slider, SliderEvent, SliderState, SliderValue},
    switch::Switch,
    theme::ThemeMode,
};
use support::{labeled_control, panel, run_rooted_example, shell, stat_tile};

fn main() {
    run_rooted_example(
        "Components Data Panel",
        ThemeMode::Dark,
        size(px(1000.0), px(840.0)),
        |window, cx| cx.new(|cx| ComponentsDataPanelExample::new(window, cx)),
    );
}

struct ComponentsDataPanelExample {
    cpu_threshold: Entity<SliderState>,
    memory_threshold: Entity<SliderState>,
    network_limit: Entity<SliderState>,
    alerts_enabled: bool,
    auto_throttle: bool,
    disk_compression: bool,
    cpu_usage: f32,
    memory_usage: f32,
    disk_usage: f32,
    network_tx: f32,
    event_count: usize,
    _subscriptions: Vec<Subscription>,
}

impl ComponentsDataPanelExample {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let cpu_threshold = cx.new(|_| {
            SliderState::new()
                .min(10.0)
                .max(100.0)
                .step(5.0)
                .default_value(80.0)
        });
        let memory_threshold = cx.new(|_| {
            SliderState::new()
                .min(10.0)
                .max(100.0)
                .step(5.0)
                .default_value(75.0)
        });
        let network_limit = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1000.0)
                .step(50.0)
                .default_value(500.0)
        });

        let subscriptions = vec![
            cx.subscribe(&cpu_threshold, |_, _, _: &SliderEvent, cx| cx.notify()),
            cx.subscribe(&memory_threshold, |_, _, _: &SliderEvent, cx| cx.notify()),
            cx.subscribe(&network_limit, |_, _, _: &SliderEvent, cx| cx.notify()),
        ];

        Self {
            cpu_threshold,
            memory_threshold,
            network_limit,
            alerts_enabled: true,
            auto_throttle: false,
            disk_compression: true,
            cpu_usage: 42.0,
            memory_usage: 61.0,
            disk_usage: 78.0,
            network_tx: 230.0,
            event_count: 0,
            _subscriptions: subscriptions,
        }
    }

    fn toggle_alerts(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.alerts_enabled = *checked;
        cx.notify();
    }

    fn toggle_throttle(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.auto_throttle = *checked;
        cx.notify();
    }

    fn toggle_compression(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.disk_compression = *checked;
        cx.notify();
    }

    fn simulate_spike(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.cpu_usage = (self.cpu_usage + 12.0).min(100.0);
        self.memory_usage = (self.memory_usage + 8.0).min(100.0);
        self.network_tx = (self.network_tx + 85.0).min(1000.0);
        self.event_count += 1;
        cx.notify();
    }

    fn reset_metrics(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.cpu_usage = 42.0;
        self.memory_usage = 61.0;
        self.disk_usage = 78.0;
        self.network_tx = 230.0;
        cx.notify();
    }
}

impl Render for ComponentsDataPanelExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cpu_threshold = match self.cpu_threshold.read(cx).value() {
            SliderValue::Single(v) => v,
            SliderValue::Range(_, v) => v,
        };
        let memory_threshold = match self.memory_threshold.read(cx).value() {
            SliderValue::Single(v) => v,
            SliderValue::Range(_, v) => v,
        };
        let network_limit = match self.network_limit.read(cx).value() {
            SliderValue::Single(v) => v,
            SliderValue::Range(_, v) => v,
        };

        let cpu_alert = self.alerts_enabled && self.cpu_usage >= cpu_threshold as f32;
        let mem_alert = self.alerts_enabled && self.memory_usage >= memory_threshold as f32;

        shell(
            "Components data panel",
            "Pure mozui-components: live metrics, threshold sliders, feature toggles, and action buttons — no native backend.",
        )
        .id("data-panel-scroll")
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(stat_tile("CPU", format!("{:.0}%", self.cpu_usage)))
                .child(stat_tile("Memory", format!("{:.0}%", self.memory_usage)))
                .child(stat_tile("Disk", format!("{:.0}%", self.disk_usage)))
                .child(stat_tile("Net TX", format!("{:.0} Mb/s", self.network_tx)))
                .child(stat_tile("Events", format!("{}", self.event_count))),
        )
        .child(
            panel(
                "Live resource meters",
                "Component progress bars showing current utilisation against configurable thresholds.",
            )
            .child(labeled_control(
                "CPU utilisation",
                if cpu_alert {
                    "Alert: usage is above your configured threshold."
                } else {
                    "Tracks active compute threads across all cores."
                },
                Progress::new("data-panel-cpu")
                    .small()
                    .value(self.cpu_usage),
            ))
            .child(labeled_control(
                "Memory pressure",
                if mem_alert {
                    "Alert: memory is above your configured threshold."
                } else {
                    "Combined physical and compressed memory usage."
                },
                Progress::new("data-panel-memory")
                    .small()
                    .value(self.memory_usage),
            ))
            .child(labeled_control(
                "Disk utilisation",
                "Occupied space as a percentage of total capacity.",
                Progress::new("data-panel-disk")
                    .small()
                    .value(self.disk_usage),
            ))
            .child(labeled_control(
                "Network throughput",
                "Current outbound bandwidth fraction against the configured cap.",
                Progress::new("data-panel-network")
                    .small()
                    .value((self.network_tx / network_limit as f32 * 100.0).min(100.0)),
            )),
        )
        .child(
            panel(
                "Alert thresholds",
                "Drag to set the percentage at which each resource is considered over-budget.",
            )
            .child(labeled_control(
                "CPU alert threshold",
                format!("Trigger alert above {cpu_threshold:.0}%"),
                Slider::new(&self.cpu_threshold).horizontal(),
            ))
            .child(labeled_control(
                "Memory alert threshold",
                format!("Trigger alert above {memory_threshold:.0}%"),
                Slider::new(&self.memory_threshold).horizontal(),
            ))
            .child(labeled_control(
                "Network cap",
                format!("Throttle above {network_limit:.0} Mb/s"),
                Slider::new(&self.network_limit).horizontal(),
            )),
        )
        .child(
            panel(
                "Feature flags",
                "Runtime toggles that change how the data pipeline behaves — all via semantic components.",
            )
            .child(labeled_control(
                "Threshold alerts",
                "Highlight meters that exceed their configured threshold.",
                Switch::new("data-panel-alerts")
                    .checked(self.alerts_enabled)
                    .label("Raise alerts when metrics cross thresholds")
                    .on_click(cx.listener(Self::toggle_alerts)),
            ))
            .child(labeled_control(
                "Auto-throttle",
                "Automatically cap network throughput when the limit is exceeded.",
                Switch::new("data-panel-throttle")
                    .checked(self.auto_throttle)
                    .label("Throttle network when cap is exceeded")
                    .on_click(cx.listener(Self::toggle_throttle)),
            ))
            .child(labeled_control(
                "Disk compression",
                "Compress idle data blocks to reduce on-disk footprint.",
                Switch::new("data-panel-compression")
                    .checked(self.disk_compression)
                    .label("Enable transparent disk compression")
                    .on_click(cx.listener(Self::toggle_compression)),
            )),
        )
        .child(
            panel(
                "Simulation controls",
                "Buttons that mutate the metric state so you can observe threshold alerts in action.",
            )
            .child(
                div()
                    .flex()
                    .gap(px(10.0))
                    .child(
                        Button::new("data-panel-spike")
                            .label("Simulate Spike")
                            .primary()
                            .on_click(cx.listener(Self::simulate_spike)),
                    )
                    .child(
                        Button::new("data-panel-reset")
                            .label("Reset Metrics")
                            .secondary()
                            .on_click(cx.listener(Self::reset_metrics)),
                    ),
            ),
        )
    }
}
