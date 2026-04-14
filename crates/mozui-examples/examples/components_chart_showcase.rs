mod support;

use mozui::prelude::*;
use mozui::{Context, Window, div, hsla, px, size};
use mozui_components::{
    ActiveTheme, StyledExt as _,
    chart::{
        AreaChart, BarChart, CandlestickChart, HeatmapChart, HistogramChart, LineChart, PieChart,
        RadarChart, RidgeLineChart, SankeyChart, ScatterChart, StackedAreaChart, StackedBarChart,
        TreemapChart, WaterfallChart, WaterfallKind,
    },
    theme::ThemeMode,
};
use support::{panel, run_rooted_example, shell};

fn main() {
    run_rooted_example(
        "Chart Gallery",
        ThemeMode::Dark,
        size(px(1060.0), px(900.0)),
        |window, cx| cx.new(|cx| ChartGallery::new(window, cx)),
    );
}

// ---------------------------------------------------------------------------
// Existing chart data types
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MonthRevenue {
    month: &'static str,
    revenue: f64,
}

#[derive(Clone)]
struct DailyVisitors {
    day: &'static str,
    visitors: f64,
}

#[derive(Clone)]
struct HourlyTraffic {
    hour: &'static str,
    web: f64,
    mobile: f64,
}

#[derive(Clone)]
struct TrafficSource {
    label: &'static str,
    pct: f32,
    color: mozui::Hsla,
}

#[derive(Clone)]
struct AssetCandle {
    date: &'static str,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

// ---------------------------------------------------------------------------
// Phase 1 chart data types
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct RequestSample {
    ms: f64,
}

#[derive(Clone)]
struct RevenueItem {
    label: &'static str,
    delta: f64,
    kind: WaterfallKind,
}

#[derive(Clone)]
struct MonthlyTraffic {
    month: &'static str,
    organic: f32,
    direct: f32,
    social: f32,
}

#[derive(Clone)]
struct HourlyStack {
    hour: &'static str,
    cpu: f32,
    memory: f32,
    network: f32,
}

// ---------------------------------------------------------------------------
// Phase 2 chart data types
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ClusteredPoint {
    x: f64,
    y: f64,
    cluster: u8,
}

#[derive(Clone)]
struct ServerLoad {
    time: &'static str,
    app: f64,
    db: f64,
    cache: f64,
    queue: f64,
    workers: f64,
    network: f64,
}

#[derive(Clone)]
struct HeatCell {
    day: &'static str,
    hour: &'static str,
    count: f64,
}

// ---------------------------------------------------------------------------
// Phase 3 chart data types
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct BudgetSegment {
    name: &'static str,
    amount: f64,
    pct: f64,
}

// ---------------------------------------------------------------------------
// Sample data
// ---------------------------------------------------------------------------

fn monthly_revenue() -> Vec<MonthRevenue> {
    vec![
        MonthRevenue {
            month: "Jan",
            revenue: 48_200.0,
        },
        MonthRevenue {
            month: "Feb",
            revenue: 52_100.0,
        },
        MonthRevenue {
            month: "Mar",
            revenue: 61_800.0,
        },
        MonthRevenue {
            month: "Apr",
            revenue: 57_400.0,
        },
        MonthRevenue {
            month: "May",
            revenue: 66_900.0,
        },
        MonthRevenue {
            month: "Jun",
            revenue: 72_300.0,
        },
        MonthRevenue {
            month: "Jul",
            revenue: 68_500.0,
        },
        MonthRevenue {
            month: "Aug",
            revenue: 74_100.0,
        },
        MonthRevenue {
            month: "Sep",
            revenue: 81_700.0,
        },
        MonthRevenue {
            month: "Oct",
            revenue: 79_200.0,
        },
        MonthRevenue {
            month: "Nov",
            revenue: 88_600.0,
        },
        MonthRevenue {
            month: "Dec",
            revenue: 94_800.0,
        },
    ]
}

fn daily_visitors() -> Vec<DailyVisitors> {
    vec![
        DailyVisitors {
            day: "Mon",
            visitors: 12_400.0,
        },
        DailyVisitors {
            day: "Tue",
            visitors: 14_100.0,
        },
        DailyVisitors {
            day: "Wed",
            visitors: 15_800.0,
        },
        DailyVisitors {
            day: "Thu",
            visitors: 13_500.0,
        },
        DailyVisitors {
            day: "Fri",
            visitors: 16_200.0,
        },
        DailyVisitors {
            day: "Sat",
            visitors: 9_800.0,
        },
        DailyVisitors {
            day: "Sun",
            visitors: 8_300.0,
        },
    ]
}

fn hourly_traffic() -> Vec<HourlyTraffic> {
    vec![
        HourlyTraffic {
            hour: "00",
            web: 820.0,
            mobile: 1_240.0,
        },
        HourlyTraffic {
            hour: "02",
            web: 410.0,
            mobile: 980.0,
        },
        HourlyTraffic {
            hour: "04",
            web: 280.0,
            mobile: 640.0,
        },
        HourlyTraffic {
            hour: "06",
            web: 1_100.0,
            mobile: 1_820.0,
        },
        HourlyTraffic {
            hour: "08",
            web: 4_200.0,
            mobile: 3_600.0,
        },
        HourlyTraffic {
            hour: "10",
            web: 6_800.0,
            mobile: 5_100.0,
        },
        HourlyTraffic {
            hour: "12",
            web: 7_400.0,
            mobile: 5_800.0,
        },
        HourlyTraffic {
            hour: "14",
            web: 6_900.0,
            mobile: 5_400.0,
        },
        HourlyTraffic {
            hour: "16",
            web: 7_100.0,
            mobile: 6_200.0,
        },
        HourlyTraffic {
            hour: "18",
            web: 5_600.0,
            mobile: 7_800.0,
        },
        HourlyTraffic {
            hour: "20",
            web: 3_800.0,
            mobile: 6_400.0,
        },
        HourlyTraffic {
            hour: "22",
            web: 2_100.0,
            mobile: 4_100.0,
        },
    ]
}

fn traffic_sources() -> Vec<TrafficSource> {
    vec![
        TrafficSource {
            label: "Organic",
            pct: 38.0,
            color: hsla(0.60, 0.55, 0.58, 1.0),
        },
        TrafficSource {
            label: "Direct",
            pct: 24.0,
            color: hsla(0.60, 0.45, 0.48, 1.0),
        },
        TrafficSource {
            label: "Referral",
            pct: 18.0,
            color: hsla(0.60, 0.38, 0.38, 1.0),
        },
        TrafficSource {
            label: "Social",
            pct: 13.0,
            color: hsla(0.60, 0.30, 0.30, 1.0),
        },
        TrafficSource {
            label: "Email",
            pct: 7.0,
            color: hsla(0.60, 0.22, 0.22, 1.0),
        },
    ]
}

fn asset_candles() -> Vec<AssetCandle> {
    vec![
        AssetCandle {
            date: "Apr 1",
            open: 182.40,
            high: 185.20,
            low: 180.10,
            close: 184.60,
        },
        AssetCandle {
            date: "Apr 2",
            open: 184.60,
            high: 188.80,
            low: 183.20,
            close: 187.90,
        },
        AssetCandle {
            date: "Apr 3",
            open: 187.90,
            high: 190.40,
            low: 185.60,
            close: 186.30,
        },
        AssetCandle {
            date: "Apr 4",
            open: 186.30,
            high: 187.10,
            low: 181.40,
            close: 182.70,
        },
        AssetCandle {
            date: "Apr 7",
            open: 182.70,
            high: 184.50,
            low: 178.90,
            close: 183.40,
        },
        AssetCandle {
            date: "Apr 8",
            open: 183.40,
            high: 189.20,
            low: 182.60,
            close: 188.10,
        },
        AssetCandle {
            date: "Apr 9",
            open: 188.10,
            high: 192.70,
            low: 187.30,
            close: 191.50,
        },
        AssetCandle {
            date: "Apr 10",
            open: 191.50,
            high: 194.80,
            low: 189.60,
            close: 193.20,
        },
        AssetCandle {
            date: "Apr 11",
            open: 193.20,
            high: 195.40,
            low: 190.10,
            close: 191.80,
        },
        AssetCandle {
            date: "Apr 14",
            open: 191.80,
            high: 196.90,
            low: 190.80,
            close: 195.60,
        },
        AssetCandle {
            date: "Apr 15",
            open: 195.60,
            high: 199.20,
            low: 194.40,
            close: 198.10,
        },
        AssetCandle {
            date: "Apr 16",
            open: 198.10,
            high: 201.50,
            low: 196.80,
            close: 200.40,
        },
    ]
}

fn request_samples() -> Vec<RequestSample> {
    // Normal-ish distribution centered at 200ms, σ≈40ms — 160 values
    // Arranged into symmetric count groups so each 20ms bin approximates a bell curve:
    //   bins: [80,100)×2  [100,120)×4  [120,140)×8  [140,160)×15  [160,180)×23
    //         [180,200)×28  [200,220)×28  [220,240)×23  [240,260)×15
    //         [260,280)×8  [280,300)×4  [300,320)×2
    #[rustfmt::skip]
    let values: &[f64] = &[
        // tails left
        85., 95.,
        // left outer
        104., 110., 116., 119.,
        // left mid
        121., 124., 127., 130., 132., 135., 137., 139.,
        // left inner
        141., 143., 144., 146., 147., 149., 150., 151., 153., 154., 155., 156., 157., 158., 159.,
        // core left
        160., 161., 163., 164., 165., 166., 167., 168., 169., 170.,
        171., 172., 173., 174., 175., 176., 177., 177., 178., 178., 179., 179., 179.,
        // peak left
        180., 181., 182., 183., 184., 185., 186., 187., 188., 189., 190.,
        191., 192., 193., 194., 195., 196., 197., 198., 199.,
        185., 188., 191., 194., 197., 195., 192., 189.,
        // peak right
        200., 201., 202., 203., 204., 205., 206., 207., 208., 209., 210.,
        211., 212., 213., 214., 215., 216., 217., 218., 219.,
        205., 208., 211., 214., 217., 203., 206., 209.,
        // core right
        220., 221., 222., 223., 224., 225., 226., 227., 228., 229., 230.,
        231., 232., 233., 234., 235., 236., 237., 237., 238., 238., 239., 239.,
        // right inner
        241., 243., 245., 247., 249., 251., 252., 253., 254., 255., 256., 257., 258., 259., 259.,
        // right mid
        261., 264., 267., 270., 272., 275., 277., 279.,
        // right outer
        282., 288., 293., 298.,
        // tails right
        305., 315.,
    ];
    values.iter().map(|&ms| RequestSample { ms }).collect()
}

fn revenue_bridge() -> Vec<RevenueItem> {
    vec![
        RevenueItem {
            label: "Start",
            delta: 120_000.,
            kind: WaterfallKind::Total,
        },
        RevenueItem {
            label: "Product",
            delta: 58_000.,
            kind: WaterfallKind::Increase,
        },
        RevenueItem {
            label: "Services",
            delta: 31_000.,
            kind: WaterfallKind::Increase,
        },
        RevenueItem {
            label: "COGS",
            delta: -42_000.,
            kind: WaterfallKind::Decrease,
        },
        RevenueItem {
            label: "Overhead",
            delta: -19_500.,
            kind: WaterfallKind::Decrease,
        },
        RevenueItem {
            label: "Net",
            delta: 0.,
            kind: WaterfallKind::Total,
        },
    ]
}

fn monthly_traffic() -> Vec<MonthlyTraffic> {
    vec![
        MonthlyTraffic {
            month: "Jan",
            organic: 3_800.,
            direct: 4_200.,
            social: 900.,
        },
        MonthlyTraffic {
            month: "Feb",
            organic: 4_100.,
            direct: 3_800.,
            social: 1_100.,
        },
        MonthlyTraffic {
            month: "Mar",
            organic: 4_800.,
            direct: 3_500.,
            social: 1_400.,
        },
        MonthlyTraffic {
            month: "Apr",
            organic: 5_200.,
            direct: 2_900.,
            social: 1_700.,
        },
        MonthlyTraffic {
            month: "May",
            organic: 6_100.,
            direct: 2_600.,
            social: 2_000.,
        },
        MonthlyTraffic {
            month: "Jun",
            organic: 6_800.,
            direct: 2_400.,
            social: 2_400.,
        },
        MonthlyTraffic {
            month: "Jul",
            organic: 7_400.,
            direct: 2_200.,
            social: 2_700.,
        },
        MonthlyTraffic {
            month: "Aug",
            organic: 8_100.,
            direct: 2_000.,
            social: 3_100.,
        },
    ]
}

fn hourly_stack() -> Vec<HourlyStack> {
    vec![
        HourlyStack {
            hour: "00",
            cpu: 12.,
            memory: 38.,
            network: 3.,
        },
        HourlyStack {
            hour: "02",
            cpu: 8.,
            memory: 36.,
            network: 2.,
        },
        HourlyStack {
            hour: "04",
            cpu: 6.,
            memory: 35.,
            network: 1.,
        },
        HourlyStack {
            hour: "06",
            cpu: 18.,
            memory: 37.,
            network: 5.,
        },
        HourlyStack {
            hour: "08",
            cpu: 48.,
            memory: 52.,
            network: 18.,
        },
        HourlyStack {
            hour: "10",
            cpu: 71.,
            memory: 64.,
            network: 32.,
        },
        HourlyStack {
            hour: "12",
            cpu: 78.,
            memory: 70.,
            network: 38.,
        },
        HourlyStack {
            hour: "14",
            cpu: 74.,
            memory: 68.,
            network: 35.,
        },
        HourlyStack {
            hour: "16",
            cpu: 80.,
            memory: 72.,
            network: 41.,
        },
        HourlyStack {
            hour: "18",
            cpu: 61.,
            memory: 65.,
            network: 28.,
        },
        HourlyStack {
            hour: "20",
            cpu: 38.,
            memory: 56.,
            network: 14.,
        },
        HourlyStack {
            hour: "22",
            cpu: 21.,
            memory: 44.,
            network: 7.,
        },
    ]
}

fn clustered_scatter() -> Vec<ClusteredPoint> {
    // Triangle arrangement with intentional boundary overlap.
    //   0 (blue)  — bottom-left,  center ~(26, 26), bulk [10, 42]
    //   1 (green) — top-right,    center ~(70, 70), bulk [56, 84]
    //   2 (red)   — center sweep, center ~(48, 48), bulk [32, 64]
    // Clusters 0↔2 share territory around (34-42, 34-42).
    // Clusters 1↔2 share territory around (56-64, 56-64).
    #[rustfmt::skip]
    let raw: &[(f64, f64, u8)] = &[
        // Cluster 0 — bottom-left core
        (12.,14.,0), (14.,18.,0), (16.,12.,0), (18.,22.,0), (20.,16.,0), (22.,20.,0), (24.,14.,0),
        (24.,26.,0), (26.,18.,0), (26.,28.,0), (28.,22.,0), (28.,30.,0), (30.,16.,0), (30.,26.,0),
        (32.,20.,0), (32.,30.,0), (34.,24.,0), (20.,32.,0), (22.,36.,0), (16.,28.,0), (18.,34.,0),
        (14.,30.,0), (12.,24.,0), (26.,36.,0), (28.,38.,0), (30.,34.,0), (34.,38.,0), (36.,34.,0),
        (38.,28.,0), (40.,32.,0), (20.,28.,0), (24.,34.,0), (18.,26.,0), (36.,26.,0), (38.,22.,0),
        (40.,18.,0), (32.,14.,0), (28.,14.,0), (22.,12.,0), (16.,20.,0),
        // Cluster 1 — top-right core
        (58.,58.,1), (60.,62.,1), (62.,58.,1), (62.,66.,1), (64.,60.,1), (64.,68.,1), (66.,56.,1),
        (66.,64.,1), (68.,60.,1), (68.,70.,1), (70.,64.,1), (70.,74.,1), (72.,58.,1), (72.,68.,1),
        (74.,62.,1), (74.,72.,1), (76.,66.,1), (76.,76.,1), (78.,62.,1), (78.,72.,1), (80.,68.,1),
        (80.,78.,1), (82.,64.,1), (82.,74.,1), (56.,64.,1), (58.,70.,1), (60.,76.,1), (62.,80.,1),
        (64.,74.,1), (66.,80.,1), (68.,76.,1), (70.,80.,1), (72.,76.,1), (74.,82.,1), (56.,72.,1),
        (60.,68.,1), (64.,78.,1), (58.,78.,1), (70.,82.,1), (76.,70.,1),
        // Cluster 2 — central sweep, overlapping both edges
        (34.,34.,2), (36.,38.,2), (38.,36.,2), (38.,42.,2), (40.,40.,2), (40.,46.,2), (42.,38.,2),
        (42.,48.,2), (44.,42.,2), (44.,50.,2), (46.,44.,2), (46.,52.,2), (48.,46.,2), (48.,54.,2),
        (50.,48.,2), (50.,56.,2), (52.,50.,2), (52.,58.,2), (54.,52.,2), (54.,60.,2), (56.,54.,2),
        (56.,62.,2), (58.,56.,2), (60.,58.,2), (62.,54.,2), (62.,60.,2), (36.,44.,2), (40.,50.,2),
        (44.,56.,2), (48.,60.,2), (34.,40.,2), (42.,44.,2), (46.,48.,2), (50.,52.,2), (54.,56.,2),
        (58.,60.,2), (38.,50.,2), (44.,46.,2), (50.,44.,2), (56.,56.,2),
    ];
    raw.iter()
        .map(|&(x, y, c)| ClusteredPoint { x, y, cluster: c })
        .collect()
}

fn server_load() -> Vec<ServerLoad> {
    vec![
        ServerLoad {
            time: "00",
            app: 12.,
            db: 8.,
            cache: 24.,
            queue: 3.,
            workers: 6.,
            network: 18.,
        },
        ServerLoad {
            time: "02",
            app: 8.,
            db: 5.,
            cache: 20.,
            queue: 2.,
            workers: 4.,
            network: 12.,
        },
        ServerLoad {
            time: "04",
            app: 6.,
            db: 4.,
            cache: 18.,
            queue: 1.,
            workers: 3.,
            network: 9.,
        },
        ServerLoad {
            time: "06",
            app: 14.,
            db: 10.,
            cache: 26.,
            queue: 5.,
            workers: 10.,
            network: 22.,
        },
        ServerLoad {
            time: "08",
            app: 48.,
            db: 38.,
            cache: 42.,
            queue: 22.,
            workers: 35.,
            network: 55.,
        },
        ServerLoad {
            time: "10",
            app: 72.,
            db: 58.,
            cache: 55.,
            queue: 40.,
            workers: 62.,
            network: 80.,
        },
        ServerLoad {
            time: "12",
            app: 80.,
            db: 65.,
            cache: 60.,
            queue: 48.,
            workers: 70.,
            network: 88.,
        },
        ServerLoad {
            time: "14",
            app: 76.,
            db: 61.,
            cache: 57.,
            queue: 44.,
            workers: 65.,
            network: 82.,
        },
        ServerLoad {
            time: "16",
            app: 82.,
            db: 68.,
            cache: 62.,
            queue: 50.,
            workers: 72.,
            network: 90.,
        },
        ServerLoad {
            time: "18",
            app: 60.,
            db: 50.,
            cache: 52.,
            queue: 32.,
            workers: 54.,
            network: 70.,
        },
        ServerLoad {
            time: "20",
            app: 38.,
            db: 28.,
            cache: 40.,
            queue: 18.,
            workers: 32.,
            network: 46.,
        },
        ServerLoad {
            time: "22",
            app: 22.,
            db: 14.,
            cache: 30.,
            queue: 8.,
            workers: 18.,
            network: 28.,
        },
    ]
}

fn heat_cells() -> Vec<HeatCell> {
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let hours = ["06-09", "09-12", "12-15", "15-18", "18-21", "21-24"];
    // Wide range: overnight = 2-6 (green), mid-day = 30-55 (yellow), peak = 80-100 (red).
    // Weekdays peak at lunch/afternoon; weekends peak in the evening.
    #[rustfmt::skip]
    let counts: [[f64; 6]; 7] = [
        //  06-09  09-12  12-15  15-18  18-21  21-24
        [    6.,   48.,   82.,   90.,   35.,    4.], // Mon
        [    4.,   52.,   88.,   95.,   38.,    5.], // Tue
        [    5.,   55.,   92.,  100.,   42.,    6.], // Wed
        [    4.,   50.,   85.,   88.,   36.,    4.], // Thu
        [    8.,   46.,   75.,   78.,   62.,   18.], // Fri
        [    3.,   12.,   28.,   40.,   85.,   55.], // Sat
        [    2.,    8.,   18.,   30.,   72.,   42.], // Sun
    ];
    let mut cells = vec![];
    for (di, &day) in days.iter().enumerate() {
        for (hi, &hour) in hours.iter().enumerate() {
            cells.push(HeatCell {
                day,
                hour,
                count: counts[di][hi],
            });
        }
    }
    cells
}

fn budget_segments() -> Vec<BudgetSegment> {
    vec![
        BudgetSegment { name: "Engineering",  amount: 4_200_000., pct: 35.0 },
        BudgetSegment { name: "Sales",        amount: 2_400_000., pct: 20.0 },
        BudgetSegment { name: "Marketing",    amount: 1_800_000., pct: 15.0 },
        BudgetSegment { name: "Operations",   amount: 1_440_000., pct: 12.0 },
        BudgetSegment { name: "Support",      amount:   960_000., pct:  8.0 },
        BudgetSegment { name: "HR",           amount:   600_000., pct:  5.0 },
        BudgetSegment { name: "Legal",        amount:   360_000., pct:  3.0 },
        BudgetSegment { name: "Finance",      amount:   240_000., pct:  2.0 },
    ]
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

struct ChartGallery;

impl ChartGallery {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for ChartGallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chart_1 = cx.theme().chart_1;
        let chart_2 = cx.theme().chart_2;
        let chart_3 = cx.theme().chart_3;
        let chart_4 = cx.theme().chart_4;
        let bullish = cx.theme().chart_bullish;
        let bearish = cx.theme().chart_bearish;
        let warning = cx.theme().warning;

        let revenue = monthly_revenue();
        let visitors = daily_visitors();
        let traffic = hourly_traffic();
        let sources = traffic_sources();
        let candles = asset_candles();
        let requests = request_samples();
        let bridge = revenue_bridge();
        let m_traffic = monthly_traffic();
        let h_stack = hourly_stack();
        let scatter_pts = clustered_scatter();
        let srv_load = server_load();
        let heat = heat_cells();
        let budget = budget_segments();

        // Distinct hues for stacked series — from theme tokens
        let s_blue = cx.theme().chart_2;
        let s_green = cx.theme().chart_bullish;
        let s_amber = cx.theme().chart_bearish;

        shell(
            "Chart Gallery",
            "mozui chart components",
        )
        .id("chart-gallery-scroll")
        .overflow_y_scroll()
        // ── Section: Existing Charts ───────────────────────────────────────
        .child(section_label("Existing Charts"))
        .child(
            panel(
                "Bar Chart — Monthly Revenue",
                "Total revenue per month. ScaleBand X, automatic bar width capped at 30px.",
            )
            .child(
                div().w_full().h(px(200.0)).child(
                    BarChart::new(revenue)
                        .x(|d: &MonthRevenue| d.month.to_string())
                        .y(|d: &MonthRevenue| d.revenue)
                        .fill(move |_| chart_2),
                ),
            ),
        )
        .child(
            panel(
                "Line Chart — Daily Active Visitors",
                "Catmull-Rom spline with data-point dots. ScalePoint X.",
            )
            .child(
                div().w_full().h(px(180.0)).child(
                    LineChart::new(visitors)
                        .x(|d: &DailyVisitors| d.day.to_string())
                        .y(|d: &DailyVisitors| d.visitors)
                        .stroke(chart_2)
                        .natural()
                        .dot(),
                ),
            ),
        )
        .child(
            panel(
                "Area Chart — Hourly Traffic by Platform",
                "Two overlapping area series. Stroke + semi-transparent fill per series.",
            )
            .child(
                div().w_full().h(px(180.0)).child(
                    AreaChart::new(traffic.clone())
                        .x(|d: &HourlyTraffic| d.hour.to_string())
                        .y(|d: &HourlyTraffic| d.web)
                        .stroke(chart_3)
                        .fill(chart_3.opacity(0.35))
                        .natural()
                        .y(|d: &HourlyTraffic| d.mobile)
                        .stroke(chart_1)
                        .fill(chart_1.opacity(0.25))
                        .natural()
                        .tick_margin(2),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(legend_dot(chart_3, "Web"))
                    .child(legend_dot(chart_1, "Mobile")),
            ),
        )
        .child(
            panel(
                "Pie Chart (Donut) — Traffic Sources",
                "inner_radius > 0 renders donut. center_value / center_label drawn in the hole.",
            )
            .child(
                div().w_full().h(px(220.0)).child(
                    PieChart::new(sources.clone())
                        .value(|d: &TrafficSource| d.pct)
                        .color(|d: &TrafficSource| d.color)
                        .inner_radius(52.0)
                        .outer_radius(80.0)
                        .pad_angle(0.025)
                        .center_value("100%")
                        .center_label("Coverage"),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .children(sources.iter().map(|s| legend_dot(s.color, s.label))),
            ),
        )
        .child(
            panel(
                "Candlestick Chart — Asset OHLC",
                "Green body = close above open. 1px wick, filled body. ScaleBand X.",
            )
            .child(
                div().w_full().h(px(220.0)).child(
                    CandlestickChart::new(candles)
                        .x(|c: &AssetCandle| c.date.to_string())
                        .open(|c: &AssetCandle| c.open)
                        .high(|c: &AssetCandle| c.high)
                        .low(|c: &AssetCandle| c.low)
                        .close(|c: &AssetCandle| c.close)
                        .body_width_ratio(0.7)
                        .tick_margin(2),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(legend_dot(bullish, "Bullish"))
                    .child(legend_dot(bearish, "Bearish")),
            ),
        )
        // ── Section: Phase 1 Charts ────────────────────────────────────────
        .child(section_label("Phase 1 Charts"))
        .child(
            panel(
                "Histogram — HTTP Request Latency",
                "Bimodal distribution: fast API (~80ms) and slow render (~320ms) clusters. \
                 Contiguous bins, no gap. ScaleLinear X auto-computed from value range.",
            )
            .child(
                div().w_full().h(px(200.0)).child(
                    HistogramChart::new(requests)
                        .value(|s: &RequestSample| s.ms)
                        .bins(16)
                        .fill(chart_2)
                        .tick_margin(4),
                ),
            ),
        )
        .child(
            panel(
                "Waterfall Chart — Revenue Bridge",
                "Total bars span from zero. Increase = bullish, Decrease = bearish. \
                 Dashed connectors at new running sum.",
            )
            .child(
                div().w_full().h(px(200.0)).child(
                    WaterfallChart::new(bridge)
                        .x(|d: &RevenueItem| d.label.to_string())
                        .y(|d: &RevenueItem| d.delta)
                        .kind(|d: &RevenueItem| d.kind),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(legend_dot(bullish, "Increase"))
                    .child(legend_dot(bearish, "Decrease"))
                    .child(legend_dot(chart_2, "Total")),
            ),
        )
        .child(
            panel(
                "Stacked Bar Chart — Monthly Traffic by Source",
                "Uses Stack primitive to compute y0/y1 per segment. Bar drawn bottom-to-top. \
                 ScaleBand X.",
            )
            .child(
                div().w_full().h(px(200.0)).child(
                    StackedBarChart::new(m_traffic.clone())
                        .x(|d: &MonthlyTraffic| d.month.to_string())
                        .series("Organic", |d: &MonthlyTraffic| d.organic, s_blue)
                        .series("Direct", |d: &MonthlyTraffic| d.direct, s_amber)
                        .series("Social", |d: &MonthlyTraffic| d.social, s_green),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(legend_dot(s_blue, "Organic"))
                    .child(legend_dot(s_amber, "Direct"))
                    .child(legend_dot(s_green, "Social")),
            ),
        )
        .child(
            panel(
                "Stacked Area Chart — Hourly Resource Usage",
                "Manual PathBuilder fill + stroke. Variable y0 per layer (Stack primitive). \
                 ScalePoint X.",
            )
            .child(
                div().w_full().h(px(200.0)).child(
                    StackedAreaChart::new(h_stack)
                        .x(|d: &HourlyStack| d.hour.to_string())
                        .series("CPU", |d: &HourlyStack| d.cpu, s_blue, s_blue.opacity(0.45))
                        .series(
                            "Memory",
                            |d: &HourlyStack| d.memory,
                            s_amber,
                            s_amber.opacity(0.40),
                        )
                        .series(
                            "Network",
                            |d: &HourlyStack| d.network,
                            s_green,
                            s_green.opacity(0.35),
                        )
                        .tick_margin(2),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(legend_dot(s_blue, "CPU"))
                    .child(legend_dot(s_amber, "Memory"))
                    .child(legend_dot(s_green, "Network")),
            ),
        )
        // ── Section: Phase 2 Charts ────────────────────────────────────────
        .child(section_label("Phase 2 Charts"))
        .child(
            panel(
                "Scatter Chart — Three Cluster Groups",
                "120 points across three clusters. Per-point color accessor maps cluster → theme color.",
            )
            .child(
                div().w_full().h(px(220.0)).child(
                    ScatterChart::new(scatter_pts)
                        .x(|d: &ClusteredPoint| d.x)
                        .y(|d: &ClusteredPoint| d.y)
                        .color(move |d: &ClusteredPoint| match d.cluster {
                            0 => chart_2,
                            1 => bullish,
                            _ => bearish,
                        })
                        .dot_size(5.0),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(legend_dot(chart_2, "Cluster A"))
                    .child(legend_dot(bullish, "Cluster B"))
                    .child(legend_dot(bearish, "Cluster C")),
            ),
        )
        .child(
            panel(
                "Radar Charts — Performance Profile & Product Comparison",
                "Left: 5 axes, 2 series. Right: 7 axes, 4 series, 5 grid levels.",
            )
            .child(
                div()
                    .flex()
                    .gap(px(16.0))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(div().w_full().h(px(260.0)).child(
                                RadarChart::new()
                                    .axes(["Speed", "Power", "Endurance", "Agility", "Technique"])
                                    .series("Athlete A", vec![82., 65., 74., 88., 77.], chart_2)
                                    .series("Athlete B", vec![70., 88., 66., 74., 84.], bullish)
                                    .fill_opacity(0.18),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(12.0))
                                    .child(legend_dot(chart_2, "Athlete A"))
                                    .child(legend_dot(bullish, "Athlete B")),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(div().w_full().h(px(260.0)).child(
                                RadarChart::new()
                                    .axes(["Speed", "Quality", "Reliability", "Support", "Price", "UX", "Features"])
                                    .series("Pro",        vec![88., 82., 90., 78., 55., 85., 92.], chart_2)
                                    .series("Basic",      vec![60., 68., 74., 62., 88., 70., 58.], bullish)
                                    .series("Enterprise", vec![92., 90., 94., 95., 40., 88., 96.], bearish)
                                    .series("Trial",      vec![50., 55., 62., 48., 95., 60., 45.], warning)
                                    .fill_opacity(0.12)
                                    .grid_levels(5),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(12.0))
                                    .child(legend_dot(chart_2, "Pro"))
                                    .child(legend_dot(bullish, "Basic"))
                                    .child(legend_dot(bearish, "Enterprise"))
                                    .child(legend_dot(warning, "Trial")),
                            ),
                    ),
            ),
        )
        .child(
            panel(
                "Ridge Line Chart — Server Component Load",
                "6 series, each in its own lane. Catmull-Rom fill + stroke. Overlap = 0.6.",
            )
            .child(
                div().w_full().h(px(320.0)).child(
                    RidgeLineChart::new(srv_load)
                        .x(|d: &ServerLoad| d.time.to_string())
                        .series("App",     |d: &ServerLoad| d.app,     chart_2, chart_2.opacity(0.40))
                        .series("DB",      |d: &ServerLoad| d.db,      bullish, bullish.opacity(0.40))
                        .series("Cache",   |d: &ServerLoad| d.cache,   bearish, bearish.opacity(0.35))
                        .series("Queue",   |d: &ServerLoad| d.queue,   chart_3, chart_3.opacity(0.35))
                        .series("Workers", |d: &ServerLoad| d.workers, warning, warning.opacity(0.35))
                        .series("Network", |d: &ServerLoad| d.network, chart_4, chart_4.opacity(0.35))
                        .overlap(0.6)
                        .tick_margin(2),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(legend_dot(chart_2, "App"))
                    .child(legend_dot(bullish, "DB"))
                    .child(legend_dot(bearish, "Cache"))
                    .child(legend_dot(chart_3, "Queue"))
                    .child(legend_dot(warning, "Workers"))
                    .child(legend_dot(chart_4, "Network")),
            ),
        )
        .child(
            panel(
                "Heatmap — Activity by Day & Hour",
                "3-stop Hsla lerp: green (low) → warning (mid) → red (high). No Y-axis labels.",
            )
            .child(
                div().w_full().h(px(180.0)).child(
                    HeatmapChart::new(heat)
                        .x(|d: &HeatCell| d.day.to_string())
                        .y(|d: &HeatCell| d.hour.to_string())
                        .value(|d: &HeatCell| d.count)
                        .mid(warning)
                        .x_order(
                            ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map(str::to_string),
                        )
                        .y_order(
                            ["06-09", "09-12", "12-15", "15-18", "18-21", "21-24"]
                                .map(str::to_string),
                        ),
                ),
            ),
        )
        // ── Section: Phase 3 Charts ────────────────────────────────────────
        .child(section_label("Phase 3 Charts"))
        .child(
            panel(
                "Treemap — Budget by Department",
                "Squarify recursive layout. Tiles sorted descending, color cycles chart_1–chart_5. \
                 Labels hidden when tile too small.",
            )
            .child(
                div().w_full().h(px(260.0)).child(
                    TreemapChart::new(budget)
                        .value(|d: &BudgetSegment| d.amount)
                        .label(|d: &BudgetSegment| d.name.into())
                        .sublabel(|d: &BudgetSegment| format!("{:.0}%", d.pct).into())
                        .cell_gap(2.),
                ),
            ),
        )
        .child(
            panel(
                "Sankey Diagram — User Acquisition Flow",
                "BFS column assignment, proportional node heights, relaxation passes. \
                 Cubic Bezier fill-width ribbons colored by source node.",
            )
            .child({
                let c2      = cx.theme().chart_2;
                let c3      = cx.theme().chart_3;
                let bullish = cx.theme().chart_bullish;
                let bearish = cx.theme().chart_bearish;
                let warn    = cx.theme().warning;
                div().w_full().h(px(300.0)).child(
                    SankeyChart::new()
                        .node("visits",   "Visits")
                        .node("organic",  "Organic")
                        .node("paid",     "Paid")
                        .node("referral", "Referral")
                        .node("signup",   "Sign Up")
                        .node("trial",    "Free Trial")
                        .node("purchase", "Purchase")
                        .node("churn",    "Churned")
                        .link("visits",   "organic",  3_800.)
                        .link("visits",   "paid",     2_400.)
                        .link("visits",   "referral", 1_200.)
                        .link("organic",  "signup",   2_100.)
                        .link("paid",     "signup",   1_600.)
                        .link("referral", "signup",     800.)
                        .link("signup",   "trial",    3_800.)
                        .link("trial",    "purchase", 1_400.)
                        .link("trial",    "churn",    2_400.)
                        .color(move |n| match n.id.as_ref() {
                            "visits"   => c2,
                            "organic"  => bullish,
                            "paid"     => warn,
                            "referral" => c3,
                            "signup"   => c2,
                            "trial"    => bullish,
                            "purchase" => bullish,
                            "churn"    => bearish,
                            _          => c2,
                        })
                        .node_gap(10.)
                        .link_opacity(0.45),
                )
            }),
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn section_label(label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(
            div()
                .text_xs()
                .font_semibold()
                .text_color(hsla(0.0, 0.0, 1.0, 0.50))
                .child(label),
        )
        .child(div().flex_1().h(px(1.0)).bg(hsla(0.0, 0.0, 1.0, 0.08)))
}

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

