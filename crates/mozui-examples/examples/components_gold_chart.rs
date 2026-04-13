mod support;

use mozui::prelude::*;
use mozui::{ClickEvent, Context, Window, div, hsla, px, size};
use mozui_components::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    chart::{CandlestickChart, LineChart},
    tag::Tag,
    theme::ThemeMode,
};
use support::{panel, run_rooted_example, shell, stat_tile};

fn main() {
    run_rooted_example(
        "Gold Price Tracker",
        ThemeMode::Dark,
        size(px(1020.0), px(820.0)),
        |window, cx| cx.new(|cx| GoldChartExample::new(window, cx)),
    );
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Timeframe {
    OneWeek,
    OneMonth,
    ThreeMonths,
}

impl Timeframe {
    fn label(self) -> &'static str {
        match self {
            Timeframe::OneWeek => "1W",
            Timeframe::OneMonth => "1M",
            Timeframe::ThreeMonths => "3M",
        }
    }
}

#[derive(Clone)]
struct Candle {
    date: &'static str,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

struct GoldChartExample {
    timeframe: Timeframe,
}

// 20 simulated daily XAU/USD candles (price in USD/oz)
fn candles_1m() -> Vec<Candle> {
    vec![
        Candle {
            date: "Apr 1",
            open: 2232.0,
            high: 2248.0,
            low: 2218.0,
            close: 2241.0,
        },
        Candle {
            date: "Apr 2",
            open: 2241.0,
            high: 2260.0,
            low: 2235.0,
            close: 2255.0,
        },
        Candle {
            date: "Apr 3",
            open: 2255.0,
            high: 2270.0,
            low: 2248.0,
            close: 2262.0,
        },
        Candle {
            date: "Apr 4",
            open: 2262.0,
            high: 2275.0,
            low: 2253.0,
            close: 2258.0,
        },
        Candle {
            date: "Apr 5",
            open: 2258.0,
            high: 2265.0,
            low: 2240.0,
            close: 2244.0,
        },
        Candle {
            date: "Apr 8",
            open: 2244.0,
            high: 2252.0,
            low: 2231.0,
            close: 2248.0,
        },
        Candle {
            date: "Apr 9",
            open: 2248.0,
            high: 2268.0,
            low: 2244.0,
            close: 2265.0,
        },
        Candle {
            date: "Apr 10",
            open: 2265.0,
            high: 2280.0,
            low: 2260.0,
            close: 2278.0,
        },
        Candle {
            date: "Apr 11",
            open: 2278.0,
            high: 2295.0,
            low: 2272.0,
            close: 2288.0,
        },
        Candle {
            date: "Apr 12",
            open: 2288.0,
            high: 2300.0,
            low: 2281.0,
            close: 2296.0,
        },
        Candle {
            date: "Apr 14",
            open: 2296.0,
            high: 2310.0,
            low: 2289.0,
            close: 2305.0,
        },
        Candle {
            date: "Apr 15",
            open: 2305.0,
            high: 2318.0,
            low: 2298.0,
            close: 2312.0,
        },
        Candle {
            date: "Apr 16",
            open: 2312.0,
            high: 2322.0,
            low: 2300.0,
            close: 2308.0,
        },
        Candle {
            date: "Apr 17",
            open: 2308.0,
            high: 2315.0,
            low: 2290.0,
            close: 2295.0,
        },
        Candle {
            date: "Apr 18",
            open: 2295.0,
            high: 2310.0,
            low: 2286.0,
            close: 2304.0,
        },
        Candle {
            date: "Apr 21",
            open: 2304.0,
            high: 2320.0,
            low: 2298.0,
            close: 2318.0,
        },
        Candle {
            date: "Apr 22",
            open: 2318.0,
            high: 2335.0,
            low: 2312.0,
            close: 2330.0,
        },
        Candle {
            date: "Apr 23",
            open: 2330.0,
            high: 2348.0,
            low: 2325.0,
            close: 2342.0,
        },
        Candle {
            date: "Apr 24",
            open: 2342.0,
            high: 2355.0,
            low: 2334.0,
            close: 2350.0,
        },
        Candle {
            date: "Apr 25",
            open: 2350.0,
            high: 2368.0,
            low: 2345.0,
            close: 2362.0,
        },
    ]
}

fn candles_1w() -> Vec<Candle> {
    candles_1m().into_iter().rev().take(5).rev().collect()
}

fn candles_3m() -> Vec<Candle> {
    // Extend with earlier data
    let mut earlier = vec![
        Candle {
            date: "Feb 3",
            open: 2050.0,
            high: 2065.0,
            low: 2041.0,
            close: 2058.0,
        },
        Candle {
            date: "Feb 10",
            open: 2058.0,
            high: 2075.0,
            low: 2050.0,
            close: 2070.0,
        },
        Candle {
            date: "Feb 17",
            open: 2070.0,
            high: 2090.0,
            low: 2065.0,
            close: 2085.0,
        },
        Candle {
            date: "Feb 24",
            open: 2085.0,
            high: 2100.0,
            low: 2078.0,
            close: 2095.0,
        },
        Candle {
            date: "Mar 3",
            open: 2095.0,
            high: 2115.0,
            low: 2088.0,
            close: 2108.0,
        },
        Candle {
            date: "Mar 10",
            open: 2108.0,
            high: 2125.0,
            low: 2100.0,
            close: 2118.0,
        },
        Candle {
            date: "Mar 17",
            open: 2118.0,
            high: 2135.0,
            low: 2110.0,
            close: 2128.0,
        },
        Candle {
            date: "Mar 24",
            open: 2128.0,
            high: 2148.0,
            low: 2120.0,
            close: 2140.0,
        },
        Candle {
            date: "Mar 31",
            open: 2140.0,
            high: 2158.0,
            low: 2132.0,
            close: 2150.0,
        },
    ];
    earlier.extend(candles_1m().into_iter().step_by(3));
    earlier
}

#[derive(Clone)]
struct PricePoint {
    date: &'static str,
    price: f64,
}

impl GoldChartExample {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            timeframe: Timeframe::OneMonth,
        }
    }

    fn candles(&self) -> Vec<Candle> {
        match self.timeframe {
            Timeframe::OneWeek => candles_1w(),
            Timeframe::OneMonth => candles_1m(),
            Timeframe::ThreeMonths => candles_3m(),
        }
    }

    fn set_timeframe(&mut self, tf: Timeframe, _window: &mut Window, cx: &mut Context<Self>) {
        self.timeframe = tf;
        cx.notify();
    }

    fn current_price(&self) -> f64 {
        self.candles().last().map(|c| c.close).unwrap_or(0.0)
    }

    fn price_change(&self) -> f64 {
        let candles = self.candles();
        if candles.len() < 2 {
            return 0.0;
        }
        let first = candles.first().unwrap().open;
        let last = candles.last().unwrap().close;
        last - first
    }

    fn price_change_pct(&self) -> f64 {
        let candles = self.candles();
        if candles.len() < 2 {
            return 0.0;
        }
        let first = candles.first().unwrap().open;
        let last = candles.last().unwrap().close;
        (last - first) / first * 100.0
    }

    fn period_high(&self) -> f64 {
        self.candles()
            .iter()
            .map(|c| c.high)
            .fold(0.0_f64, f64::max)
    }

    fn period_low(&self) -> f64 {
        self.candles()
            .iter()
            .map(|c| c.low)
            .fold(f64::MAX, f64::min)
    }

    fn close_line(&self) -> Vec<PricePoint> {
        self.candles()
            .into_iter()
            .map(|c| PricePoint {
                date: c.date,
                price: c.close,
            })
            .collect()
    }
}

impl Render for GoldChartExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let price = self.current_price();
        let change = self.price_change();
        let change_pct = self.price_change_pct();
        let high = self.period_high();
        let low = self.period_low();
        let candles = self.candles();
        let line_data = self.close_line();
        let tf = self.timeframe;
        let is_up = change >= 0.0;
        let chart_2 = cx.theme().chart_2;
        let bullish = cx.theme().chart_bullish;
        let bearish = cx.theme().chart_bearish;

        let change_tag = if is_up {
            Tag::success()
                .small()
                .child(format!("+{:.0} (+{:.2}%)", change, change_pct))
        } else {
            Tag::danger()
                .small()
                .child(format!("{:.0} ({:.2}%)", change, change_pct))
        };

        shell(
            "Gold Price Tracker",
            "Pure mozui-components: XAU/USD candlestick chart and closing price line — simulated data.",
        )
        .id("gold-scroll")
        .overflow_y_scroll()
        // Timeframe selector
        .child(
            div()
                .flex()
                .gap(px(8.0))
                .children([Timeframe::OneWeek, Timeframe::OneMonth, Timeframe::ThreeMonths].map(|t| {
                    let label = t.label();
                    let is_active = t == tf;
                    Button::new(format!("gold-tf-{}", label))
                        .label(label)
                        .when(is_active, |b| b.primary())
                        .when(!is_active, |b| b.secondary())
                        .small()
                        .on_click(cx.listener(move |this: &mut GoldChartExample, _: &ClickEvent, w, cx| {
                            this.set_timeframe(t, w, cx);
                        }))
                })),
        )
        // Stats row
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .items_center()
                .child(stat_tile("XAU/USD", format!("${:.2}", price)))
                .child(stat_tile("Period High", format!("${:.2}", high)))
                .child(stat_tile("Period Low",  format!("${:.2}", low)))
                .child(stat_tile("Candles",     format!("{}", candles.len())))
                .child(
                    div().flex().items_center().child(change_tag),
                ),
        )
        // Candlestick chart
        .child(
            panel(
                "XAU/USD candlestick",
                "Open, high, low, close for each period. Green = bullish, red = bearish.",
            )
            .child(
                div()
                    .w_full()
                    .h(px(240.0))
                    .child(
                        CandlestickChart::new(candles)
                            .x(|c: &Candle| c.date.to_string())
                            .open(|c: &Candle| c.open)
                            .high(|c: &Candle| c.high)
                            .low(|c: &Candle| c.low)
                            .close(|c: &Candle| c.close)
                            .body_width_ratio(0.7)
                            .tick_margin(2),
                    ),
            ),
        )
        // Close price line chart
        .child(
            panel(
                "Closing price",
                "End-of-period close price plotted as a continuous line.",
            )
            .child(
                div()
                    .w_full()
                    .h(px(180.0))
                    .child(
                        LineChart::new(line_data)
                            .x(|p: &PricePoint| p.date.to_string())
                            .y(|p: &PricePoint| p.price)
                            .stroke(if is_up { bullish } else { bearish })
                            .natural()
                            .tick_margin(2),
                    ),
            ),
        )
        // Legend
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .items_center()
                .child(legend_dot(bullish, "Bullish"))
                .child(legend_dot(bearish, "Bearish"))
                .child(legend_dot(chart_2, "Close line")),
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn legend_dot(color: mozui::Hsla, label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(div().w(px(10.0)).h(px(10.0)).rounded_full().bg(color))
        .child(
            div()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 1.0, 0.62))
                .child(label),
        )
}
