mod area_chart;
mod bar_chart;
mod candlestick_chart;
mod heatmap_chart;
mod histogram_chart;
mod line_chart;
mod pie_chart;
mod radar_chart;
mod ridge_line_chart;
mod sankey_chart;
mod scatter_chart;
mod stacked_area_chart;
mod stacked_bar_chart;
mod treemap_chart;
mod waterfall_chart;

pub use area_chart::AreaChart;
pub use bar_chart::BarChart;
pub use candlestick_chart::CandlestickChart;
pub use heatmap_chart::HeatmapChart;
pub use histogram_chart::{HistogramBins, HistogramChart};
pub use line_chart::LineChart;
pub use pie_chart::PieChart;
pub use radar_chart::{RadarChart, RadarSeries};
pub use ridge_line_chart::RidgeLineChart;
pub use sankey_chart::{SankeyChart, SankeyLink, SankeyNode};
pub use scatter_chart::ScatterChart;
pub use stacked_area_chart::StackedAreaChart;
pub use stacked_bar_chart::StackedBarChart;
pub use treemap_chart::TreemapChart;
pub use waterfall_chart::{WaterfallChart, WaterfallKind};

use mozui::{Hsla, SharedString, TextAlign};

use crate::plot::{
    AxisText,
    scale::{Scale, ScaleBand, ScalePoint},
};

/// Build x-axis labels for point-based scales (`LineChart`, `AreaChart`).
///
/// Point scales place items at evenly spaced positions. The first label is
/// left-aligned, the last is right-aligned, and the rest are centered.
pub(crate) fn build_point_x_labels<T, X>(
    data: &[T],
    x_fn: &dyn Fn(&T) -> X,
    x_scale: &ScalePoint<X>,
    tick_margin: usize,
    color: Hsla,
) -> Vec<AxisText>
where
    X: PartialEq + Into<SharedString>,
{
    let data_len = data.len();
    data.iter()
        .enumerate()
        .filter_map(|(i, d)| {
            if (i + 1) % tick_margin != 0 {
                return None;
            }
            x_scale.tick(&x_fn(d)).map(|x_tick| {
                let align = match i {
                    0 if data_len == 1 => TextAlign::Center,
                    0 => TextAlign::Left,
                    i if i == data_len - 1 => TextAlign::Right,
                    _ => TextAlign::Center,
                };
                // Call x_fn again to get an owned value for the label text.
                AxisText::new(x_fn(d).into(), x_tick, color).align(align)
            })
        })
        .collect()
}

/// Build x-axis labels for band-based scales (`BarChart`, `CandlestickChart`).
///
/// Band scales place items in evenly sized bands. Labels are always
/// center-aligned within their band.
pub(crate) fn build_band_x_labels<T, X>(
    data: &[T],
    x_fn: &dyn Fn(&T) -> X,
    x_scale: &ScaleBand<X>,
    band_width: f32,
    tick_margin: usize,
    color: Hsla,
) -> Vec<AxisText>
where
    X: PartialEq + Into<SharedString>,
{
    data.iter()
        .enumerate()
        .filter_map(|(i, d)| {
            if (i + 1) % tick_margin != 0 {
                return None;
            }
            x_scale.tick(&x_fn(d)).map(|x_tick| {
                // Call x_fn again to get an owned value for the label text.
                AxisText::new(x_fn(d).into(), x_tick + band_width / 2., color)
                    .align(TextAlign::Center)
            })
        })
        .collect()
}
