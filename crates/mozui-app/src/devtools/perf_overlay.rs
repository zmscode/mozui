use mozui_devtools::FrameTimings;
use mozui_elements::{Element, div, label};
use mozui_style::Color;

/// Number of frame-time bars to display in the graph.
const BAR_COUNT: usize = 120;
/// Bar width in pixels.
const BAR_W: f32 = 2.0;
/// Maximum bar height in pixels.
const BAR_MAX_H: f32 = 60.0;
/// Frame time (us) that maps to full bar height.
const MAX_FRAME_US: f32 = 33_333.0; // ~30fps

fn bar_color(total_us: u64) -> Color {
    if total_us < 16_667 {
        // < 16.7ms — green (60fps+)
        Color::rgba(80, 200, 120, 1.0)
    } else if total_us < 33_333 {
        // < 33.3ms — yellow (30-60fps)
        Color::rgba(240, 200, 60, 1.0)
    } else {
        // > 33.3ms — red (<30fps)
        Color::rgba(240, 80, 80, 1.0)
    }
}

/// Build the performance overlay element tree.
pub fn perf_overlay(timings: &FrameTimings) -> Box<dyn Element> {
    let bg = Color::new(0.0, 0.0, 0.0, 0.75);
    let text_color = Color::new(1.0, 1.0, 1.0, 0.9);
    let dim_color = Color::new(1.0, 1.0, 1.0, 0.5);

    let fps = timings.avg_fps();
    let latest = timings.latest().copied().unwrap_or_default();

    // FPS label color matches bar color logic
    let fps_color = if fps >= 58.0 {
        Color::rgba(80, 200, 120, 1.0)
    } else if fps >= 28.0 {
        Color::rgba(240, 200, 60, 1.0)
    } else {
        Color::rgba(240, 80, 80, 1.0)
    };

    // Build bar chart — last BAR_COUNT frames
    let all: Vec<_> = timings.iter().collect();
    let start = all.len().saturating_sub(BAR_COUNT);
    let bars = &all[start..];

    let mut bar_container = div()
        .flex_row()
        .items_end()
        .justify_end()
        .gap(1.0)
        .h(BAR_MAX_H);

    for timing in bars {
        let frac = (timing.total_us as f32 / MAX_FRAME_US).min(1.0);
        let h = (frac * BAR_MAX_H).max(1.0);
        bar_container = bar_container.child(
            div()
                .w(BAR_W)
                .h(h)
                .bg(bar_color(timing.total_us))
                .rounded(1.0),
        );
    }

    // Stats lines
    let stats_section = div()
        .flex_col()
        .gap(2.0)
        .child(stat_row("Layout", latest.layout_us, dim_color, text_color))
        .child(stat_row("Paint", latest.paint_us, dim_color, text_color))
        .child(stat_row("Render", latest.render_us, dim_color, text_color))
        .child(stat_row("Total", latest.total_us, dim_color, text_color))
        .child(
            div()
                .h(1.0)
                .w_full()
                .bg(Color::new(1.0, 1.0, 1.0, 0.15)),
        )
        .child(count_row("Draw calls", latest.draw_call_count, dim_color, text_color))
        .child(count_row("Elements", latest.element_count, dim_color, text_color))
        .child(count_row("Nodes", latest.node_count, dim_color, text_color));

    // Root container — positioned top-right via the overlay wrapper
    let panel = div()
        .w(220.0)
        .flex_col()
        .gap(8.0)
        .p(12.0)
        .bg(bg)
        .rounded(8.0)
        // FPS header
        .child(
            div()
                .flex_row()
                .items_end()
                .gap(6.0)
                .child(label(format!("{:.0}", fps)).font_size(24.0).bold().color(fps_color))
                .child(label("FPS").font_size(11.0).color(dim_color)),
        )
        // Bar chart
        .child(bar_container)
        // Stats
        .child(stats_section);

    // Full-window overlay wrapper — flex to top-right
    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_row()
            .justify_end()
            .items_start()
            .p(12.0)
            .child(panel),
    )
}

fn stat_row(name: &str, us: u64, label_color: Color, value_color: Color) -> impl Element {
    let ms = us as f64 / 1000.0;
    div()
        .flex_row()
        .justify_between()
        .gap(16.0)
        .child(label(name).font_size(11.0).color(label_color))
        .child(label(format!("{:.1}ms", ms)).font_size(11.0).color(value_color))
}

fn count_row(name: &str, count: u32, label_color: Color, value_color: Color) -> impl Element {
    div()
        .flex_row()
        .justify_between()
        .gap(16.0)
        .child(label(name).font_size(11.0).color(label_color))
        .child(label(format!("{}", count)).font_size(11.0).color(value_color))
}
