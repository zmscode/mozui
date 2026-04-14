use std::rc::Rc;

use mozui::{App, Bounds, Hsla, Pixels, TextAlign, Window, fill, px};
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, AxisText, Grid, Plot, PlotAxis, origin_point,
        scale::{Scale, ScaleLinear},
    },
};

pub enum HistogramBins {
    Count(usize),
    Thresholds(Vec<f64>),
}

#[derive(IntoPlot)]
pub struct HistogramChart<T: 'static> {
    data: Vec<T>,
    value: Option<Rc<dyn Fn(&T) -> f64>>,
    bins: HistogramBins,
    fill: Option<Hsla>,
    tick_margin: usize,
    x_axis: bool,
    grid: bool,
}

impl<T> HistogramChart<T> {
    pub fn new(data: impl IntoIterator<Item = T>) -> Self {
        Self {
            data: data.into_iter().collect(),
            value: None,
            bins: HistogramBins::Count(20),
            fill: None,
            tick_margin: 5,
            x_axis: true,
            grid: true,
        }
    }

    pub fn value(mut self, f: impl Fn(&T) -> f64 + 'static) -> Self {
        self.value = Some(Rc::new(f));
        self
    }

    pub fn bins(mut self, count: usize) -> Self {
        self.bins = HistogramBins::Count(count.max(1));
        self
    }

    pub fn thresholds(mut self, thresholds: Vec<f64>) -> Self {
        self.bins = HistogramBins::Thresholds(thresholds);
        self
    }

    pub fn fill(mut self, color: impl Into<Hsla>) -> Self {
        self.fill = Some(color.into());
        self
    }

    pub fn tick_margin(mut self, n: usize) -> Self {
        self.tick_margin = n.max(1);
        self
    }

    pub fn x_axis(mut self, show: bool) -> Self {
        self.x_axis = show;
        self
    }

    pub fn grid(mut self, show: bool) -> Self {
        self.grid = show;
        self
    }
}

impl<T> Plot for HistogramChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(value_fn) = self.value.as_ref() else {
            return;
        };

        let values: Vec<f64> = self.data.iter().map(|d| value_fn(d)).collect();
        if values.is_empty() {
            return;
        }

        let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if min_val >= max_val {
            return;
        }

        // Compute bin break points
        let breaks: Vec<f64> = match &self.bins {
            HistogramBins::Count(n) => {
                let n = *n;
                let step = (max_val - min_val) / n as f64;
                (0..=n).map(|i| min_val + i as f64 * step).collect()
            }
            HistogramBins::Thresholds(thresholds) => {
                let mut t = thresholds.clone();
                t.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                if t.first().map_or(true, |&v| v > min_val) {
                    t.insert(0, min_val);
                }
                if t.last().map_or(true, |&v| v < max_val) {
                    t.push(max_val);
                }
                t
            }
        };

        let n_bins = breaks.len().saturating_sub(1);
        if n_bins == 0 {
            return;
        }

        // Count values per bin using upper-bound search
        // Bin i = [breaks[i], breaks[i+1]), except last = [breaks[n-1], breaks[n]]
        let mut counts = vec![0usize; n_bins];
        for &v in &values {
            let idx = breaks
                .partition_point(|&b| b <= v)
                .saturating_sub(1)
                .min(n_bins - 1);
            counts[idx] += 1;
        }

        let max_count = counts.iter().cloned().max().unwrap_or(1).max(1);

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        let x_scale = ScaleLinear::new(vec![min_val, max_val], vec![0., width]);
        let y_scale = ScaleLinear::new(vec![0.0f64, max_count as f64], vec![height, 10.]);

        // Grid
        if self.grid {
            Grid::new()
                .y((0..=3).map(|i| height * i as f32 / 4.0).collect())
                .stroke(cx.theme().border)
                .dash_array(&[px(4.), px(2.)])
                .paint(&bounds, window);
        }

        // Bars — contiguous, no gap between bins
        let bar_color = self.fill.unwrap_or(cx.theme().chart_2);
        let origin = bounds.origin;

        for (i, &count) in counts.iter().enumerate() {
            let lo = breaks[i];
            let hi = breaks[i + 1];

            let x_lo = x_scale.tick(&lo).unwrap_or(0.);
            let x_hi = x_scale.tick(&hi).unwrap_or(0.);
            let y_top = y_scale.tick(&(count as f64)).unwrap_or(height);

            let p1 = origin_point(px(x_lo), px(y_top), origin);
            let p2 = origin_point(px(x_hi), px(height), origin);
            window.paint_quad(fill(mozui::Bounds::from_corners(p1, p2), bar_color));
        }

        // X axis
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
            let n_boundaries = n_bins + 1;
            let muted = cx.theme().muted_foreground;
            let tick_margin = self.tick_margin;

            let labels: Vec<AxisText> = breaks
                .iter()
                .enumerate()
                .filter_map(|(i, &boundary)| {
                    if i % tick_margin != 0 && i != n_boundaries - 1 {
                        return None;
                    }
                    let x_px = x_scale.tick(&boundary)?;
                    let align = if i == 0 {
                        TextAlign::Left
                    } else if i == n_boundaries - 1 {
                        TextAlign::Right
                    } else {
                        TextAlign::Center
                    };
                    Some(AxisText::new(format!("{:.0}", boundary), x_px, muted).align(align))
                })
                .collect();

            axis = axis.x(height).x_label(labels);
        }
        axis.paint(&bounds, window, cx);
    }
}
