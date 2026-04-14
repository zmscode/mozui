use std::rc::Rc;

use mozui::{App, BorderStyle, Bounds, Hsla, Pixels, TextAlign, Window, px, quad, size};
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, AxisText, Grid, Plot, PlotAxis, origin_point,
        scale::{Scale, ScaleLinear},
    },
};

#[derive(IntoPlot)]
pub struct ScatterChart<T: 'static> {
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> f64>>,
    y: Option<Rc<dyn Fn(&T) -> f64>>,
    color: Option<Rc<dyn Fn(&T) -> Hsla>>,
    dot_size: f32,
    x_ticks: usize,
    x_axis: bool,
    grid: bool,
}

impl<T> ScatterChart<T> {
    pub fn new(data: impl IntoIterator<Item = T>) -> Self {
        Self {
            data: data.into_iter().collect(),
            x: None,
            y: None,
            color: None,
            dot_size: 5.0,
            x_ticks: 5,
            x_axis: true,
            grid: true,
        }
    }

    pub fn x(mut self, x: impl Fn(&T) -> f64 + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn y(mut self, y: impl Fn(&T) -> f64 + 'static) -> Self {
        self.y = Some(Rc::new(y));
        self
    }

    /// Set a constant fill color for all dots.
    pub fn fill(mut self, color: impl Into<Hsla> + 'static + Copy) -> Self {
        self.color = Some(Rc::new(move |_| color.into()));
        self
    }

    /// Set a per-point color accessor (e.g. for cluster coloring).
    pub fn color(mut self, f: impl Fn(&T) -> Hsla + 'static) -> Self {
        self.color = Some(Rc::new(f));
        self
    }

    pub fn dot_size(mut self, s: f32) -> Self {
        self.dot_size = s.max(1.0);
        self
    }

    pub fn x_ticks(mut self, n: usize) -> Self {
        self.x_ticks = n.max(2);
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

impl<T> Plot for ScatterChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (Some(x_fn), Some(y_fn)) = (self.x.as_ref(), self.y.as_ref()) else {
            return;
        };
        if self.data.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        let xs: Vec<f64> = self.data.iter().map(|d| x_fn(d)).collect();
        let ys: Vec<f64> = self.data.iter().map(|d| y_fn(d)).collect();
        let x_min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let x_max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_min = ys.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        if x_min >= x_max || y_min >= y_max {
            return;
        }

        // Pad domain so edge dots aren't clipped
        let x_pad = (x_max - x_min) * 0.05;
        let y_pad = (y_max - y_min) * 0.05;
        let x_scale = ScaleLinear::new(vec![x_min - x_pad, x_max + x_pad], vec![0., width]);
        let y_scale = ScaleLinear::new(vec![y_min - y_pad, y_max + y_pad], vec![height, 10.]);

        // Grid
        if self.grid {
            Grid::new()
                .y((0..=3).map(|i| height * i as f32 / 4.0).collect())
                .stroke(cx.theme().border)
                .dash_array(&[px(4.), px(2.)])
                .paint(&bounds, window);
        }

        // X axis
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
            let muted = cx.theme().muted_foreground;
            let n = self.x_ticks;
            let step = (x_max - x_min) / (n - 1) as f64;
            let labels: Vec<AxisText> = (0..n)
                .filter_map(|i| {
                    let v = x_min + i as f64 * step;
                    let x_px = x_scale.tick(&v)?;
                    let align = if i == 0 {
                        TextAlign::Left
                    } else if i == n - 1 {
                        TextAlign::Right
                    } else {
                        TextAlign::Center
                    };
                    Some(AxisText::new(format!("{:.0}", v), x_px, muted).align(align))
                })
                .collect();
            axis = axis.x(height).x_label(labels);
        }
        axis.paint(&bounds, window, cx);

        // Dots
        let default_color = cx.theme().chart_2;
        let color_fn = self.color.clone();
        let dot_s = self.dot_size;

        for d in &self.data {
            let x_px = x_scale.tick(&x_fn(d)).unwrap_or(0.);
            let y_px = y_scale.tick(&y_fn(d)).unwrap_or(0.);
            let color = color_fn.as_ref().map(|f| f(d)).unwrap_or(default_color);
            let top_left =
                origin_point(px(x_px - dot_s / 2.), px(y_px - dot_s / 2.), bounds.origin);
            let dot_px = px(dot_s);
            window.paint_quad(quad(
                mozui::bounds(top_left, size(dot_px, dot_px)),
                dot_px / 2.,
                color,
                px(0.),
                color,
                BorderStyle::default(),
            ));
        }
    }
}
