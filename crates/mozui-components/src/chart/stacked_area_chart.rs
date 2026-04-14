use std::rc::Rc;

use mozui::{App, Bounds, Hsla, PathBuilder, Pixels, SharedString, Window, px};
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, Grid, Plot, PlotAxis, origin_point,
        scale::{Scale, ScaleLinear, ScalePoint},
        shape::Stack,
    },
};

use super::build_point_x_labels;

#[derive(IntoPlot)]
pub struct StackedAreaChart<T, X>
where
    T: Clone + 'static,
    X: Clone + PartialEq + Into<SharedString> + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    keys: Vec<String>,
    accessors: Vec<Rc<dyn Fn(&T) -> f32>>,
    colors: Vec<Hsla>,
    fills: Vec<Hsla>,
    tick_margin: usize,
    x_axis: bool,
    grid: bool,
}

impl<T, X> StackedAreaChart<T, X>
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
            tick_margin: 1,
            x_axis: true,
            grid: true,
        }
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    /// Add a series. `stroke` is the top-edge line color; `fill` is the area fill.
    /// If `fill` is omitted via the second method, defaults to `stroke.opacity(0.35)`.
    pub fn series(
        mut self,
        key: impl Into<String>,
        accessor: impl Fn(&T) -> f32 + 'static,
        stroke: impl Into<Hsla>,
        fill: impl Into<Hsla>,
    ) -> Self {
        self.keys.push(key.into());
        self.accessors.push(Rc::new(accessor));
        let stroke = stroke.into();
        self.colors.push(stroke);
        self.fills.push(fill.into());
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

impl<T, X> Plot for StackedAreaChart<T, X>
where
    T: Clone + 'static,
    X: Clone + PartialEq + Into<SharedString> + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(x_fn) = self.x.as_ref() else {
            return;
        };
        if self.keys.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        // X scale (ScalePoint for area-style charts)
        let x = ScalePoint::new(self.data.iter().map(|v| x_fn(v)).collect(), vec![0., width]);

        // Compute max total for Y scale
        let max_total = self
            .data
            .iter()
            .map(|d| self.accessors.iter().map(|acc| acc(d) as f64).sum::<f64>())
            .fold(0.0_f64, f64::max);

        let y = ScaleLinear::new(vec![0.0f64, max_total.max(1.0)], vec![height, 10.]);

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

        // Grid
        if self.grid {
            Grid::new()
                .y((0..=3).map(|i| height * i as f32 / 4.0).collect())
                .stroke(cx.theme().border)
                .dash_array(&[px(4.), px(2.)])
                .paint(&bounds, window);
        }

        // Build stack
        let keys = self.keys.clone();
        let accessors = self.accessors.clone();
        let stacked = Stack::new()
            .data(self.data.clone())
            .keys(keys.clone())
            .value(move |d, key| keys.iter().position(|k| k == key).map(|i| accessors[i](d)))
            .series();

        // Draw each series bottom-to-top
        let default_color = cx.theme().chart_2;
        let origin = bounds.origin;

        for (series_idx, stack_series) in stacked.iter().enumerate() {
            let stroke_color = self
                .colors
                .get(series_idx)
                .copied()
                .unwrap_or(default_color);
            let fill_color = self
                .fills
                .get(series_idx)
                .copied()
                .unwrap_or_else(|| stroke_color.opacity(0.35));

            // Collect pixel positions for each data point
            let points: Vec<(f32, f32, f32)> = stack_series
                .points
                .iter()
                .filter_map(|sp| {
                    let x_px = x.tick(&x_fn(&sp.data))?;
                    let y0_px = y.tick(&(sp.y0 as f64))?;
                    let y1_px = y.tick(&(sp.y1 as f64))?;
                    Some((x_px, y0_px, y1_px))
                })
                .collect();

            if points.len() < 2 {
                continue;
            }

            // Fill path: top edge forward, bottom edge backward, close
            let mut fill_builder = PathBuilder::fill();
            fill_builder.move_to(origin_point(px(points[0].0), px(points[0].2), origin));
            for &(x_px, _, y1_px) in &points[1..] {
                fill_builder.line_to(origin_point(px(x_px), px(y1_px), origin));
            }
            for &(x_px, y0_px, _) in points.iter().rev() {
                fill_builder.line_to(origin_point(px(x_px), px(y0_px), origin));
            }
            fill_builder.close();
            if let Ok(path) = fill_builder.build() {
                window.paint_path(path, fill_color);
            }

            // Stroke path: top edge only (2px to match line chart weight)
            let mut stroke_builder = PathBuilder::stroke(px(2.));
            stroke_builder.move_to(origin_point(px(points[0].0), px(points[0].2), origin));
            for &(x_px, _, y1_px) in &points[1..] {
                stroke_builder.line_to(origin_point(px(x_px), px(y1_px), origin));
            }
            if let Ok(path) = stroke_builder.build() {
                window.paint_path(path, stroke_color);
            }
        }
    }
}
