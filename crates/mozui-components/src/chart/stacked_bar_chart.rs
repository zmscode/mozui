use std::rc::Rc;

use mozui::{App, Bounds, Hsla, Pixels, SharedString, Window, px};
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, Grid, Plot, PlotAxis,
        scale::{Scale, ScaleBand, ScaleLinear},
        shape::{Bar, Stack, StackPoint},
    },
};

use super::build_band_x_labels;

#[derive(IntoPlot)]
pub struct StackedBarChart<T, X>
where
    T: Clone + 'static,
    X: Clone + PartialEq + Into<SharedString> + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    keys: Vec<String>,
    accessors: Vec<Rc<dyn Fn(&T) -> f32>>,
    colors: Vec<Hsla>,
    tick_margin: usize,
    x_axis: bool,
    grid: bool,
}

impl<T, X> StackedBarChart<T, X>
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
            tick_margin: 1,
            x_axis: true,
            grid: true,
        }
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn series(
        mut self,
        key: impl Into<String>,
        accessor: impl Fn(&T) -> f32 + 'static,
        color: impl Into<Hsla>,
    ) -> Self {
        self.keys.push(key.into());
        self.accessors.push(Rc::new(accessor));
        self.colors.push(color.into());
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

impl<T, X> Plot for StackedBarChart<T, X>
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

        // X scale
        let x = ScaleBand::new(self.data.iter().map(|v| x_fn(v)).collect(), vec![0., width])
            .padding_inner(0.4)
            .padding_outer(0.2);
        let band_width = x.band_width();

        // Compute max total (highest stack)
        let max_total = self
            .data
            .iter()
            .map(|d| self.accessors.iter().map(|acc| acc(d) as f64).sum::<f64>())
            .fold(0.0_f64, f64::max);

        let y = ScaleLinear::new(vec![0.0f64, max_total.max(1.0)], vec![height, 10.]);

        // X axis
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
            let labels = build_band_x_labels(
                &self.data,
                x_fn.as_ref(),
                &x,
                band_width,
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
        for (series_idx, stack_series) in stacked.into_iter().enumerate() {
            let color = self
                .colors
                .get(series_idx)
                .copied()
                .unwrap_or(default_color);
            let x_fn = x_fn.clone();
            let x_scale = x.clone();
            let y_scale_y0 = y.clone();
            let y_scale_y1 = y.clone();

            Bar::new()
                .data(stack_series.points)
                .band_width(band_width)
                .x(move |sp: &StackPoint<T>| x_scale.tick(&x_fn(&sp.data)))
                .y0(move |sp: &StackPoint<T>| y_scale_y0.tick(&(sp.y0 as f64)).unwrap_or(height))
                .y1(move |sp: &StackPoint<T>| y_scale_y1.tick(&(sp.y1 as f64)))
                .fill(move |_| color)
                .paint(&bounds, window, cx);
        }
    }
}
