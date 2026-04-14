use std::rc::Rc;

use mozui::{App, Bounds, Hsla, Pixels, SharedString, TextAlign, Window, fill, hsla, px};
// Note: hsla is used in lerp_hsla helper below
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{AXIS_GAP, AxisText, Plot, PlotAxis, origin_point},
};

#[derive(IntoPlot)]
pub struct HeatmapChart<T, X, Y>
where
    T: 'static,
    X: PartialEq + Clone + Into<SharedString> + 'static,
    Y: PartialEq + Clone + Into<SharedString> + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    y: Option<Rc<dyn Fn(&T) -> Y>>,
    value: Option<Rc<dyn Fn(&T) -> f64>>,
    x_order: Vec<X>,
    y_order: Vec<Y>,
    low: Option<Hsla>,
    mid: Option<Hsla>,
    high: Option<Hsla>,
    x_axis: bool,
}

impl<T, X, Y> HeatmapChart<T, X, Y>
where
    T: 'static,
    X: PartialEq + Clone + Into<SharedString> + 'static,
    Y: PartialEq + Clone + Into<SharedString> + 'static,
{
    pub fn new(data: impl IntoIterator<Item = T>) -> Self {
        Self {
            data: data.into_iter().collect(),
            x: None,
            y: None,
            value: None,
            x_order: vec![],
            y_order: vec![],
            low: None,
            mid: None,
            high: None,
            x_axis: true,
        }
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn y(mut self, y: impl Fn(&T) -> Y + 'static) -> Self {
        self.y = Some(Rc::new(y));
        self
    }

    pub fn value(mut self, f: impl Fn(&T) -> f64 + 'static) -> Self {
        self.value = Some(Rc::new(f));
        self
    }

    pub fn x_order(mut self, order: impl IntoIterator<Item = X>) -> Self {
        self.x_order = order.into_iter().collect();
        self
    }

    pub fn y_order(mut self, order: impl IntoIterator<Item = Y>) -> Self {
        self.y_order = order.into_iter().collect();
        self
    }

    pub fn low(mut self, color: impl Into<Hsla>) -> Self {
        self.low = Some(color.into());
        self
    }

    pub fn mid(mut self, color: impl Into<Hsla>) -> Self {
        self.mid = Some(color.into());
        self
    }

    pub fn high(mut self, color: impl Into<Hsla>) -> Self {
        self.high = Some(color.into());
        self
    }

    pub fn x_axis(mut self, show: bool) -> Self {
        self.x_axis = show;
        self
    }
}

impl<T, X, Y> Plot for HeatmapChart<T, X, Y>
where
    T: 'static,
    X: PartialEq + Clone + Into<SharedString> + 'static,
    Y: PartialEq + Clone + Into<SharedString> + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (Some(x_fn), Some(y_fn), Some(val_fn)) =
            (self.x.as_ref(), self.y.as_ref(), self.value.as_ref())
        else {
            return;
        };
        if self.data.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        // Build ordered column categories
        let x_cats: Vec<X> = if !self.x_order.is_empty() {
            self.x_order.clone()
        } else {
            let mut seen: Vec<X> = vec![];
            for d in &self.data {
                let v = x_fn(d);
                if !seen.contains(&v) {
                    seen.push(v);
                }
            }
            seen
        };

        // Build ordered row categories
        let y_cats: Vec<Y> = if !self.y_order.is_empty() {
            self.y_order.clone()
        } else {
            let mut seen: Vec<Y> = vec![];
            for d in &self.data {
                let v = y_fn(d);
                if !seen.contains(&v) {
                    seen.push(v);
                }
            }
            seen
        };

        if x_cats.is_empty() || y_cats.is_empty() {
            return;
        }

        let n_cols = x_cats.len();
        let n_rows = y_cats.len();
        let cell_w = width / n_cols as f32;
        let cell_h = height / n_rows as f32;

        // Value range
        let min_val = self
            .data
            .iter()
            .map(|d| val_fn(d))
            .fold(f64::INFINITY, f64::min);
        let max_val = self
            .data
            .iter()
            .map(|d| val_fn(d))
            .fold(f64::NEG_INFINITY, f64::max);
        let val_range = (max_val - min_val).max(1e-9);

        let low = self.low.unwrap_or(cx.theme().chart_bullish);
        let mid = self.mid;
        let high = self.high.unwrap_or(cx.theme().chart_bearish);
        let origin = bounds.origin;

        // Draw cells
        for d in &self.data {
            let x_key = x_fn(d);
            let y_key = y_fn(d);
            let val = val_fn(d);

            let Some(col) = x_cats.iter().position(|v| v == &x_key) else {
                continue;
            };
            let Some(row) = y_cats.iter().position(|v| v == &y_key) else {
                continue;
            };

            let t = ((val - min_val) / val_range) as f32;
            let color = match mid {
                Some(m) if t <= 0.5 => lerp_hsla(low, m, t * 2.0),
                Some(m) => lerp_hsla(m, high, (t - 0.5) * 2.0),
                None => lerp_hsla(low, high, t),
            };

            let x_px = col as f32 * cell_w;
            let y_px = row as f32 * cell_h;
            let p1 = origin_point(px(x_px), px(y_px), origin);
            let p2 = origin_point(px(x_px + cell_w), px(y_px + cell_h), origin);
            window.paint_quad(fill(mozui::Bounds::from_corners(p1, p2), color));
        }

        // X axis labels
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
            let muted = cx.theme().muted_foreground;
            let labels: Vec<AxisText> = x_cats
                .iter()
                .enumerate()
                .map(|(i, cat)| {
                    let x_px = i as f32 * cell_w + cell_w / 2.;
                    let align = if i == 0 {
                        TextAlign::Left
                    } else if i == n_cols - 1 {
                        TextAlign::Right
                    } else {
                        TextAlign::Center
                    };
                    AxisText::new(cat.clone().into(), x_px, muted).align(align)
                })
                .collect();
            axis = axis.x(height).x_label(labels);
        }
        axis.paint(&bounds, window, cx);
    }
}

fn lerp_hsla(a: Hsla, b: Hsla, t: f32) -> Hsla {
    hsla(
        a.h + (b.h - a.h) * t,
        a.s + (b.s - a.s) * t,
        a.l + (b.l - a.l) * t,
        a.a + (b.a - a.a) * t,
    )
}
