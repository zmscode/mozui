mod support;

use mozui::prelude::*;
use mozui::{ClickEvent, Context, Hsla, Window, div, hsla, px, size};
use mozui_components::{
    ActiveTheme, Disableable, Sizable, StyledExt as _,
    button::{Button, ButtonVariants},
    chart::BarChart,
    progress::{Progress, ProgressCircle},
    tag::Tag,
    theme::ThemeMode,
};
use support::{panel, run_rooted_example, shell, stat_tile};

fn main() {
    run_rooted_example(
        "Gym Workout Tracker",
        ThemeMode::Dark,
        size(px(980.0), px(820.0)),
        |window, cx| cx.new(|cx| GymTrackerExample::new(window, cx)),
    );
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Exercise {
    name: &'static str,
    muscle: &'static str,
    target_sets: usize,
    done_sets: usize,
    weight_kg: f32,
    reps: usize,
}

impl Exercise {
    fn completion(&self) -> f32 {
        if self.target_sets == 0 {
            return 100.0;
        }
        (self.done_sets as f32 / self.target_sets as f32 * 100.0).min(100.0)
    }

    fn volume(&self) -> f32 {
        self.done_sets as f32 * self.reps as f32 * self.weight_kg
    }
}

#[derive(Clone)]
struct WeekDay {
    label: &'static str,
    volume: f64,
}

struct GymTrackerExample {
    exercises: Vec<Exercise>,
}

impl GymTrackerExample {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            exercises: vec![
                Exercise {
                    name: "Bench Press",
                    muscle: "Chest",
                    target_sets: 4,
                    done_sets: 3,
                    weight_kg: 80.0,
                    reps: 8,
                },
                Exercise {
                    name: "Incline DB Press",
                    muscle: "Chest",
                    target_sets: 3,
                    done_sets: 3,
                    weight_kg: 32.0,
                    reps: 10,
                },
                Exercise {
                    name: "Pull-up",
                    muscle: "Back",
                    target_sets: 4,
                    done_sets: 2,
                    weight_kg: 82.0,
                    reps: 6,
                },
                Exercise {
                    name: "Cable Row",
                    muscle: "Back",
                    target_sets: 3,
                    done_sets: 0,
                    weight_kg: 55.0,
                    reps: 12,
                },
                Exercise {
                    name: "OHP",
                    muscle: "Shoulders",
                    target_sets: 4,
                    done_sets: 1,
                    weight_kg: 52.5,
                    reps: 8,
                },
                Exercise {
                    name: "Lateral Raise",
                    muscle: "Shoulders",
                    target_sets: 3,
                    done_sets: 2,
                    weight_kg: 12.0,
                    reps: 15,
                },
                Exercise {
                    name: "Tricep Pushdown",
                    muscle: "Arms",
                    target_sets: 3,
                    done_sets: 3,
                    weight_kg: 30.0,
                    reps: 12,
                },
                Exercise {
                    name: "Barbell Curl",
                    muscle: "Arms",
                    target_sets: 3,
                    done_sets: 0,
                    weight_kg: 35.0,
                    reps: 10,
                },
            ],
        }
    }

    fn log_set(&mut self, idx: usize, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ex) = self.exercises.get_mut(idx) {
            if ex.done_sets < ex.target_sets {
                ex.done_sets += 1;
            }
        }
        cx.notify();
    }

    fn undo_set(&mut self, idx: usize, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ex) = self.exercises.get_mut(idx) {
            ex.done_sets = ex.done_sets.saturating_sub(1);
        }
        cx.notify();
    }

    fn total_sets_done(&self) -> usize {
        self.exercises.iter().map(|e| e.done_sets).sum()
    }
    fn total_sets_target(&self) -> usize {
        self.exercises.iter().map(|e| e.target_sets).sum()
    }
    fn total_volume(&self) -> f32 {
        self.exercises.iter().map(|e| e.volume()).sum()
    }

    fn workout_pct(&self) -> f32 {
        let t = self.total_sets_target();
        if t == 0 {
            return 0.0;
        }
        (self.total_sets_done() as f32 / t as f32 * 100.0).min(100.0)
    }

    fn muscle_pct(&self, muscle: &str) -> f32 {
        let exs: Vec<_> = self
            .exercises
            .iter()
            .filter(|e| e.muscle == muscle)
            .collect();
        if exs.is_empty() {
            return 0.0;
        }
        let done: usize = exs.iter().map(|e| e.done_sets).sum();
        let target: usize = exs.iter().map(|e| e.target_sets).sum();
        if target == 0 {
            return 0.0;
        }
        (done as f32 / target as f32 * 100.0).min(100.0)
    }
}

impl Render for GymTrackerExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pct = self.workout_pct();
        let volume = self.total_volume();
        let done = self.total_sets_done();
        let target = self.total_sets_target();
        let chart_2 = cx.theme().chart_2;

        let weekly: Vec<WeekDay> = vec![
            WeekDay {
                label: "Mon",
                volume: 4200.0,
            },
            WeekDay {
                label: "Tue",
                volume: 0.0,
            },
            WeekDay {
                label: "Wed",
                volume: 5100.0,
            },
            WeekDay {
                label: "Thu",
                volume: 3800.0,
            },
            WeekDay {
                label: "Fri",
                volume: volume as f64,
            },
            WeekDay {
                label: "Sat",
                volume: 0.0,
            },
            WeekDay {
                label: "Sun",
                volume: 0.0,
            },
        ];

        shell(
            "Gym Workout Tracker",
            "Pure mozui-components: log sets, track muscle group completion, and view weekly volume.",
        )
        .id("gym-scroll")
        .overflow_y_scroll()
        // Stats + completion ring
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .items_center()
                .child(stat_tile("Sets Done", format!("{}/{}", done, target)))
                .child(stat_tile("Volume", format!("{:.0} kg", volume)))
                .child(stat_tile("Exercises", format!("{}", self.exercises.len())))
                .child(stat_tile("Completion", format!("{:.0}%", pct)))
                .child(
                    ProgressCircle::new("gym-ring")
                        .value(pct)
                        .color(chart_2)
                        .w(px(80.0))
                        .h(px(80.0))
                        .child(
                            div()
                                .font_semibold()
                                .text_xs()
                                .child(format!("{:.0}%", pct)),
                        ),
                ),
        )
        // Exercise list
        .child(
            panel(
                "Today's session",
                "Log sets as you complete them. Press – to undo the last logged set.",
            )
            .children(
                self.exercises
                    .iter()
                    .enumerate()
                    .map(|(idx, ex)| {
                        let done = ex.done_sets;
                        let target_sets = ex.target_sets;
                        let pct = ex.completion();
                        let is_done = done >= target_sets;
                        let name = ex.name;
                        let muscle = ex.muscle;
                        let weight = ex.weight_kg;
                        let reps = ex.reps;

                        div()
                            .w_full()
                            .rounded(px(12.0))
                            .border_1()
                            .border_color(hsla(0.0, 0.0, 1.0, if is_done { 0.14 } else { 0.07 }))
                            .bg(hsla(0.0, 0.0, 1.0, if is_done { 0.05 } else { 0.02 }))
                            .p(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(
                                                div()
                                                    .font_semibold()
                                                    .text_xs()
                                                    .text_color(hsla(0.0, 0.0, 1.0, 0.88))
                                                    .child(name),
                                            )
                                            .child(Tag::secondary().small().child(muscle)),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.50))
                                            .child(format!("{}kg × {} reps", weight, reps)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.0))
                                    .child(
                                        Progress::new(format!("gym-ex-{idx}"))
                                            .value(pct)
                                            .small()
                                            .flex_1(),
                                    )
                                    .child(
                                        div()
                                            .w(px(30.0))
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.60))
                                            .child(format!("{}/{}", done, target_sets)),
                                    )
                                    .child(
                                        Button::new(format!("gym-undo-{idx}"))
                                            .label("–")
                                            .secondary()
                                            .small()
                                            .disabled(done == 0)
                                            .on_click(cx.listener(
                                                move |this, _: &ClickEvent, window, cx| {
                                                    this.undo_set(idx, window, cx);
                                                },
                                            )),
                                    )
                                    .child(
                                        Button::new(format!("gym-log-{idx}"))
                                            .label("+ Set")
                                            .primary()
                                            .small()
                                            .disabled(is_done)
                                            .on_click(cx.listener(
                                                move |this, _: &ClickEvent, window, cx| {
                                                    this.log_set(idx, window, cx);
                                                },
                                            )),
                                    ),
                            )
                    })
                    .collect::<Vec<_>>(),
            ),
        )
        // Muscle group focus
        .child(
            panel(
                "Muscle group focus",
                "Percentage of planned sets completed per muscle group today.",
            )
            .child(muscle_row("Chest",     self.muscle_pct("Chest"),     hsla(0.02, 0.70, 0.55, 1.0)))
            .child(muscle_row("Back",      self.muscle_pct("Back"),      hsla(0.56, 0.60, 0.55, 1.0)))
            .child(muscle_row("Shoulders", self.muscle_pct("Shoulders"), hsla(0.13, 0.75, 0.55, 1.0)))
            .child(muscle_row("Arms",      self.muscle_pct("Arms"),      hsla(0.72, 0.65, 0.55, 1.0))),
        )
        // Weekly volume bar chart — wrap in sized div since BarChart doesn't impl Styled
        .child(
            panel(
                "Weekly volume",
                "Total kg lifted each day this week (sets × reps × weight).",
            )
            .child(
                div()
                    .w_full()
                    .h(px(180.0))
                    .child(
                        BarChart::new(weekly)
                            .x(|d: &WeekDay| d.label.to_string())
                            .y(|d: &WeekDay| d.volume)
                            .fill(move |_| chart_2),
                    ),
            ),
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn muscle_row(label: &'static str, pct: f32, color: impl Into<Hsla>) -> impl IntoElement {
    let color = color.into();
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .w(px(80.0))
                .text_xs()
                .font_semibold()
                .text_color(hsla(0.0, 0.0, 1.0, 0.82))
                .child(label),
        )
        .child(
            Progress::new(format!("gym-muscle-{label}"))
                .value(pct)
                .color(color)
                .small()
                .flex_1(),
        )
        .child(
            div()
                .w(px(36.0))
                .text_xs()
                .text_color(hsla(0.0, 0.0, 1.0, 0.60))
                .child(format!("{:.0}%", pct)),
        )
}
