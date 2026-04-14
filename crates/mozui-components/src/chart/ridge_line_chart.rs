use std::rc::Rc;

use mozui::{App, Bounds, Hsla, Pixels, SharedString, Window};
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, Plot, PlotAxis, StrokeStyle,
        scale::{Scale, ScaleLinear, ScalePoint},
        shape::Area,
    },
};

use super::build_point_x_labels;

#[derive(IntoPlot)]
pub struct RidgeLineChart<T, X>
where
    T: Clone + 'static,
    X: Clone + PartialEq + Into<SharedString> + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    keys: Vec<String>,
    accessors: Vec<Rc<dyn Fn(&T) -> f64>>,
    colors: Vec<Hsla>,
    fills: Vec<Hsla>,
    /// How many extra lane heights each series can grow above its baseline (overlap factor).
    overlap: f32,
    tick_margin: usize,
    x_axis: bool,
}

impl<T, X> RidgeLineChart<T, X>
where
    T: Clone + 'static,
    X: Clone + PartialEq + Into<SharedString> + 'static,
{
    pub fn new(data: impl IntoIterator<Item = T>) -> Self {
        Self {
            data: data.into_iter().collect(),
            x: None,
            keys: vec![],
            accessors: vec![],
            colors: vec![],
            fills: vec![],
            overlap: 0.6,
            tick_margin: 1,
            x_axis: true,
        }
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn series(
        mut self,
        key: impl Into<String>,
        accessor: impl Fn(&T) -> f64 + 'static,
        stroke: impl Into<Hsla>,
        fill: impl Into<Hsla>,
    ) -> Self {
        self.keys.push(key.into());
        self.accessors.push(Rc::new(accessor));
        let s = stroke.into();
        self.colors.push(s);
        self.fills.push(fill.into());
        self
    }

    pub fn overlap(mut self, overlap: f32) -> Self {
        self.overlap = overlap.clamp(0., 3.);
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
}

impl<T, X> Plot for RidgeLineChart<T, X>
where
    T: Clone + 'static,
    X: Clone + PartialEq + Into<SharedString> + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(x_fn) = self.x.as_ref() else {
            return;
        };
        if self.keys.is_empty() || self.data.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        let n_series = self.keys.len();
        let lane_step = height / n_series as f32;

        let x = ScalePoint::new(self.data.iter().map(|v| x_fn(v)).collect(), vec![0., width]);

        // X axis
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
            let labels = build_point_x_labels(
                &self.data,
                x_fn.as_ref(),
                &x,
                self.tick_margin,
                cx.theme().muted_foreground,
            );
            axis = axis.x(height).x_label(labels);
        }
        axis.paint(&bounds, window, cx);

        let default_color = cx.theme().chart_2;

        for (i, acc) in self.accessors.iter().enumerate() {
            let stroke_color = self.colors.get(i).copied().unwrap_or(default_color);
            let fill_color = self
                .fills
                .get(i)
                .copied()
                .unwrap_or_else(|| stroke_color.opacity(0.40));

            let baseline_px = (i as f32 + 1.0) * lane_step;
            let max_val = self
                .data
                .iter()
                .map(|d| acc(d))
                .fold(0.0_f64, f64::max)
                .max(1.0);

            // Peak pixel is above baseline; clamp so it doesn't go off-chart
            let peak_px = (baseline_px - lane_step * (1.0 + self.overlap)).max(5.0);
            let y_scale = ScaleLinear::new(vec![0.0f64, max_val], vec![baseline_px, peak_px]);

            let acc_c = acc.clone();
            let x_fn_c = x_fn.clone();
            let x_c = x.clone();
            let y_c = y_scale.clone();

            Area::new()
                .data(self.data.clone())
                .x(move |d: &T| x_c.tick(&x_fn_c(d)))
                .y0(baseline_px)
                .y1(move |d: &T| y_c.tick(&acc_c(d)))
                .fill(fill_color)
                .stroke(stroke_color)
                .stroke_style(StrokeStyle::Natural)
                .paint(&bounds, window);
        }
    }
}
