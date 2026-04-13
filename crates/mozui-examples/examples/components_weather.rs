mod support;

use mozui::prelude::*;
use mozui::{ClickEvent, Context, Hsla, Window, div, hsla, px, size};
use mozui_components::{
    ActiveTheme, Sizable, StyledExt as _,
    button::{Button, ButtonVariants},
    chart::{AreaChart, LineChart},
    progress::Progress,
    tag::Tag,
    theme::ThemeMode,
};
use support::{panel, run_rooted_example, shell, stat_tile};

fn main() {
    run_rooted_example(
        "Weather",
        ThemeMode::Dark,
        size(px(980.0), px(820.0)),
        |window, cx| cx.new(|cx| WeatherExample::new(window, cx)),
    );
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum City {
    Sydney,
    London,
    Tokyo,
    NewYork,
}

impl City {
    fn label(&self) -> &'static str {
        match self {
            City::Sydney => "Sydney",
            City::London => "London",
            City::Tokyo => "Tokyo",
            City::NewYork => "New York",
        }
    }

    fn temperature(&self) -> f32 {
        match self {
            City::Sydney => 24.0,
            City::London => 11.0,
            City::Tokyo => 18.0,
            City::NewYork => 15.0,
        }
    }

    fn condition(&self) -> &'static str {
        match self {
            City::Sydney => "Partly cloudy",
            City::London => "Overcast",
            City::Tokyo => "Clear",
            City::NewYork => "Scattered showers",
        }
    }

    fn humidity(&self) -> f32 {
        match self {
            City::Sydney => 62.0,
            City::London => 84.0,
            City::Tokyo => 55.0,
            City::NewYork => 70.0,
        }
    }

    fn wind_kmh(&self) -> f32 {
        match self {
            City::Sydney => 18.0,
            City::London => 28.0,
            City::Tokyo => 12.0,
            City::NewYork => 22.0,
        }
    }

    fn uv_index(&self) -> f32 {
        match self {
            City::Sydney => 7.0,
            City::London => 2.0,
            City::Tokyo => 5.0,
            City::NewYork => 4.0,
        }
    }

    fn feels_like(&self) -> f32 {
        match self {
            City::Sydney => 22.0,
            City::London => 8.0,
            City::Tokyo => 16.0,
            City::NewYork => 12.0,
        }
    }

    fn seven_day_temps(&self) -> Vec<DayTemp> {
        match self {
            City::Sydney => vec![
                DayTemp {
                    day: "Mon",
                    high: 26.0,
                    low: 18.0,
                },
                DayTemp {
                    day: "Tue",
                    high: 24.0,
                    low: 17.0,
                },
                DayTemp {
                    day: "Wed",
                    high: 22.0,
                    low: 16.0,
                },
                DayTemp {
                    day: "Thu",
                    high: 23.0,
                    low: 16.0,
                },
                DayTemp {
                    day: "Fri",
                    high: 25.0,
                    low: 17.0,
                },
                DayTemp {
                    day: "Sat",
                    high: 28.0,
                    low: 19.0,
                },
                DayTemp {
                    day: "Sun",
                    high: 27.0,
                    low: 18.0,
                },
            ],
            City::London => vec![
                DayTemp {
                    day: "Mon",
                    high: 12.0,
                    low: 7.0,
                },
                DayTemp {
                    day: "Tue",
                    high: 11.0,
                    low: 6.0,
                },
                DayTemp {
                    day: "Wed",
                    high: 10.0,
                    low: 5.0,
                },
                DayTemp {
                    day: "Thu",
                    high: 9.0,
                    low: 4.0,
                },
                DayTemp {
                    day: "Fri",
                    high: 11.0,
                    low: 6.0,
                },
                DayTemp {
                    day: "Sat",
                    high: 13.0,
                    low: 7.0,
                },
                DayTemp {
                    day: "Sun",
                    high: 14.0,
                    low: 8.0,
                },
            ],
            City::Tokyo => vec![
                DayTemp {
                    day: "Mon",
                    high: 19.0,
                    low: 12.0,
                },
                DayTemp {
                    day: "Tue",
                    high: 20.0,
                    low: 13.0,
                },
                DayTemp {
                    day: "Wed",
                    high: 21.0,
                    low: 14.0,
                },
                DayTemp {
                    day: "Thu",
                    high: 18.0,
                    low: 11.0,
                },
                DayTemp {
                    day: "Fri",
                    high: 17.0,
                    low: 10.0,
                },
                DayTemp {
                    day: "Sat",
                    high: 20.0,
                    low: 13.0,
                },
                DayTemp {
                    day: "Sun",
                    high: 22.0,
                    low: 15.0,
                },
            ],
            City::NewYork => vec![
                DayTemp {
                    day: "Mon",
                    high: 16.0,
                    low: 9.0,
                },
                DayTemp {
                    day: "Tue",
                    high: 14.0,
                    low: 8.0,
                },
                DayTemp {
                    day: "Wed",
                    high: 13.0,
                    low: 7.0,
                },
                DayTemp {
                    day: "Thu",
                    high: 15.0,
                    low: 8.0,
                },
                DayTemp {
                    day: "Fri",
                    high: 17.0,
                    low: 10.0,
                },
                DayTemp {
                    day: "Sat",
                    high: 18.0,
                    low: 11.0,
                },
                DayTemp {
                    day: "Sun",
                    high: 16.0,
                    low: 9.0,
                },
            ],
        }
    }

    fn hourly_precip(&self) -> Vec<HourPrecip> {
        let base: &[(&str, f64)] = match self {
            City::Sydney => &[
                ("6am", 0.0),
                ("9am", 0.2),
                ("12pm", 0.5),
                ("3pm", 1.2),
                ("6pm", 0.8),
                ("9pm", 0.3),
            ],
            City::London => &[
                ("6am", 1.5),
                ("9am", 2.0),
                ("12pm", 1.8),
                ("3pm", 2.5),
                ("6pm", 2.2),
                ("9pm", 1.0),
            ],
            City::Tokyo => &[
                ("6am", 0.0),
                ("9am", 0.0),
                ("12pm", 0.1),
                ("3pm", 0.0),
                ("6pm", 0.2),
                ("9pm", 0.0),
            ],
            City::NewYork => &[
                ("6am", 0.5),
                ("9am", 1.0),
                ("12pm", 2.5),
                ("3pm", 1.8),
                ("6pm", 0.9),
                ("9pm", 0.4),
            ],
        };
        base.iter()
            .map(|(h, p)| HourPrecip { hour: h, mm: *p })
            .collect()
    }
}

#[derive(Clone)]
struct DayTemp {
    day: &'static str,
    high: f64,
    low: f64,
}

#[derive(Clone)]
struct HourPrecip {
    hour: &'static str,
    mm: f64,
}

struct WeatherExample {
    city: City,
}

impl WeatherExample {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { city: City::Sydney }
    }

    fn set_city(&mut self, city: City, _window: &mut Window, cx: &mut Context<Self>) {
        self.city = city;
        cx.notify();
    }
}

impl Render for WeatherExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let city = &self.city;
        let temp = city.temperature();
        let feels = city.feels_like();
        let condition = city.condition();
        let humidity = city.humidity();
        let wind = city.wind_kmh();
        let uv = city.uv_index();
        let daily = city.seven_day_temps();
        let hourly = city.hourly_precip();
        let chart_1 = cx.theme().chart_1;
        let chart_2 = cx.theme().chart_2;
        let name = city.label();

        // Tag variant by condition
        let condition_tag = if condition.contains("Clear") {
            Tag::info().small().child(condition)
        } else if condition.contains("shower") || condition.contains("rain") {
            Tag::primary().small().child(condition)
        } else if condition.contains("cloud") || condition.contains("cast") {
            Tag::secondary().small().child(condition)
        } else {
            Tag::warning().small().child(condition)
        };

        shell(
            "Weather",
            "Pure mozui-components: 7-day temperature line, hourly precipitation area chart, and live condition indicators.",
        )
        .id("weather-scroll")
        .overflow_y_scroll()
        // City selector
        .child(
            div()
                .flex()
                .gap(px(8.0))
                .child(city_btn("weather-sydney",   "Sydney",   cx.listener(|t: &mut WeatherExample, _: &ClickEvent, w, cx| t.set_city(City::Sydney,  w, cx))))
                .child(city_btn("weather-london",   "London",   cx.listener(|t: &mut WeatherExample, _: &ClickEvent, w, cx| t.set_city(City::London,  w, cx))))
                .child(city_btn("weather-tokyo",    "Tokyo",    cx.listener(|t: &mut WeatherExample, _: &ClickEvent, w, cx| t.set_city(City::Tokyo,   w, cx))))
                .child(city_btn("weather-newyork",  "New York", cx.listener(|t: &mut WeatherExample, _: &ClickEvent, w, cx| t.set_city(City::NewYork, w, cx)))),
        )
        // Current conditions
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .items_center()
                .child(stat_tile("Location", name))
                .child(stat_tile("Temp", format!("{}°C", temp)))
                .child(stat_tile("Feels like", format!("{}°C", feels)))
                .child(stat_tile("Wind", format!("{} km/h", wind)))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(condition_tag),
                ),
        )
        // Condition meters
        .child(
            panel("Conditions", "Live humidity, UV index and wind readings.")
                .child(meter_row("Humidity",  humidity,        100.0, hsla(0.56, 0.60, 0.55, 1.0), "%"))
                .child(meter_row("UV Index",  uv,              11.0,  hsla(0.13, 0.75, 0.55, 1.0), " / 11"))
                .child(meter_row("Wind",      wind,            80.0,  hsla(0.62, 0.55, 0.55, 1.0), " km/h")),
        )
        // 7-day high temperature line chart
        .child(
            panel(
                "7-day forecast — high temperature",
                "Daily high temperature for the coming week.",
            )
            .child(
                div()
                    .w_full()
                    .h(px(180.0))
                    .child(
                        LineChart::new(daily.clone())
                            .x(|d: &DayTemp| d.day.to_string())
                            .y(|d: &DayTemp| d.high)
                            .stroke(chart_2)
                            .natural()
                            .dot(),
                    ),
            ),
        )
        // Hourly precipitation area chart
        .child(
            panel(
                "Hourly precipitation",
                "Estimated rainfall in mm for key times throughout today.",
            )
            .child(
                div()
                    .w_full()
                    .h(px(160.0))
                    .child(
                        AreaChart::new(hourly)
                            .x(|d: &HourPrecip| d.hour.to_string())
                            .y(|d: &HourPrecip| d.mm)
                            .stroke(chart_1)
                            .natural(),
                    ),
            ),
        )
        // 7-day low temperature line chart
        .child(
            panel(
                "7-day forecast — low temperature",
                "Daily low temperature for the coming week.",
            )
            .child(
                div()
                    .w_full()
                    .h(px(160.0))
                    .child(
                        LineChart::new(daily)
                            .x(|d: &DayTemp| d.day.to_string())
                            .y(|d: &DayTemp| d.low)
                            .stroke(hsla(0.56, 0.60, 0.55, 1.0))
                            .natural()
                            .dot(),
                    ),
            ),
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn city_btn(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut mozui::App) + 'static,
) -> impl IntoElement {
    Button::new(id)
        .label(label)
        .secondary()
        .small()
        .on_click(on_click)
}

fn meter_row(
    label: &'static str,
    value: f32,
    max: f32,
    color: Hsla,
    suffix: &'static str,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .w(px(72.0))
                .text_xs()
                .font_semibold()
                .text_color(hsla(0.0, 0.0, 1.0, 0.82))
                .child(label),
        )
        .child(
            Progress::new(format!("weather-{label}"))
                .value(value / max * 100.0)
                .color(color)
                .small()
                .flex_1(),
        )
        .child(
            div()
                .w(px(60.0))
                .text_xs()
                .text_color(hsla(0.0, 0.0, 1.0, 0.60))
                .child(format!("{:.0}{}", value, suffix)),
        )
}
