use std::rc::Rc;

use mozui::{App, Bounds, Hsla, PathBuilder, Pixels, SharedString, TextAlign, Window, fill, px};
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        AXIS_GAP, AxisText, Grid, Plot, PlotAxis, origin_point,
        scale::{Scale, ScaleBand, ScaleLinear},
    },
};

#[derive(Clone, Copy, PartialEq)]
pub enum WaterfallKind {
    Increase,
    Decrease,
    Total,
}

#[derive(IntoPlot)]
pub struct WaterfallChart<T, X>
where
    T: 'static,
    X: PartialEq + Into<SharedString> + Clone + 'static,
{
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> X>>,
    y: Option<Rc<dyn Fn(&T) -> f64>>,
    kind: Option<Rc<dyn Fn(&T) -> WaterfallKind>>,
    connector: bool,
    tick_margin: usize,
    x_axis: bool,
    grid: bool,
}

impl<T, X> WaterfallChart<T, X>
where
    X: PartialEq + Into<SharedString> + Clone + 'static,
{
    pub fn new(data: impl IntoIterator<Item = T>) -> Self {
        Self {
            data: data.into_iter().collect(),
            x: None,
            y: None,
            kind: None,
            connector: true,
            tick_margin: 1,
            x_axis: true,
            grid: true,
        }
    }

    pub fn x(mut self, x: impl Fn(&T) -> X + 'static) -> Self {
        self.x = Some(Rc::new(x));
        self
    }

    pub fn y(mut self, y: impl Fn(&T) -> f64 + 'static) -> Self {
        self.y = Some(Rc::new(y));
        self
    }

    pub fn kind(mut self, kind: impl Fn(&T) -> WaterfallKind + 'static) -> Self {
        self.kind = Some(Rc::new(kind));
        self
    }

    pub fn connector(mut self, show: bool) -> Self {
        self.connector = show;
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

// Pre-computed bar layout
struct WaterfallBar {
    x_key: String,
    y0: f64,
    y1: f64,
    kind: WaterfallKind,
    /// Running sum after this bar — used to position the connector to the next bar.
    connector_y: f64,
}

impl<T, X> Plot for WaterfallChart<T, X>
where
    X: PartialEq + Into<SharedString> + Clone + 'static,
{
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let (Some(x_fn), Some(y_fn)) = (self.x.as_ref(), self.y.as_ref()) else {
            return;
        };

        // Compute running totals and bar extents
        let mut running = 0.0f64;
        let bars: Vec<WaterfallBar> = self
            .data
            .iter()
            .map(|d| {
                let delta = y_fn(d);
                let kind = self.kind.as_ref().map(|f| f(d)).unwrap_or(if delta >= 0.0 {
                    WaterfallKind::Increase
                } else {
                    WaterfallKind::Decrease
                });

                let old_running = running;
                let (y0, y1) = match kind {
                    WaterfallKind::Increase | WaterfallKind::Decrease => {
                        running += delta;
                        (old_running, running)
                    }
                    WaterfallKind::Total => (0.0, running),
                };

                WaterfallBar {
                    x_key: x_fn(d).clone().into().to_string(),
                    y0,
                    y1,
                    kind,
                    connector_y: running,
                }
            })
            .collect();

        if bars.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let axis_gap = if self.x_axis { AXIS_GAP } else { 0. };
        let height = bounds.size.height.as_f32() - axis_gap;

        // X scale
        let x_keys: Vec<X> = self.data.iter().map(|d| x_fn(d)).collect();
        let x = ScaleBand::new(x_keys, vec![0., width])
            .padding_inner(0.4)
            .padding_outer(0.2);
        let band_width = x.band_width();

        // Y scale — domain spans all y0/y1 values and always includes 0
        let y_domain: Vec<f64> = bars
            .iter()
            .flat_map(|b| [b.y0, b.y1])
            .chain(std::iter::once(0.0))
            .collect();
        let y = ScaleLinear::new(y_domain, vec![height, 10.]);

        // Zero baseline pixel position
        let zero_y = y.tick(&0.0f64).unwrap_or(height);

        // Draw X axis
        let mut axis = PlotAxis::new().stroke(cx.theme().border);
        if self.x_axis {
            let n = bars.len();
            let muted = cx.theme().muted_foreground;
            let tick_margin = self.tick_margin;
            let band_width_half = band_width / 2.;

            let labels: Vec<AxisText> = bars
                .iter()
                .enumerate()
                .filter_map(|(i, b)| {
                    if (i + 1) % tick_margin != 0 {
                        return None;
                    }
                    let x_px = x.tick(&self.data.iter().nth(i).map(|d| x_fn(d))?)?;
                    let align = if i == 0 && n > 1 {
                        TextAlign::Left
                    } else if i == n - 1 {
                        TextAlign::Right
                    } else {
                        TextAlign::Center
                    };
                    Some(AxisText::new(b.x_key.clone(), x_px + band_width_half, muted).align(align))
                })
                .collect();

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

        // Zero baseline (stronger than grid — drawn after grid)
        {
            let mut builder = PathBuilder::stroke(px(1.));
            builder.move_to(origin_point(px(0.), px(zero_y), bounds.origin));
            builder.line_to(origin_point(px(width), px(zero_y), bounds.origin));
            if let Ok(path) = builder.build() {
                window.paint_path(path, cx.theme().border);
            }
        }

        // Draw bars and connectors
        let origin = bounds.origin;
        for (i, bar) in bars.iter().enumerate() {
            let Some(x_tick) = x.tick(&self.data.iter().nth(i).map(|d| x_fn(d)).unwrap()) else {
                continue;
            };

            let y0_px = y.tick(&bar.y0).unwrap_or(height);
            let y1_px = y.tick(&bar.y1).unwrap_or(height);

            let color: Hsla = match bar.kind {
                WaterfallKind::Increase => cx.theme().chart_bullish,
                WaterfallKind::Decrease => cx.theme().chart_bearish,
                WaterfallKind::Total => cx.theme().chart_2,
            };

            let top_px = y0_px.min(y1_px);
            let bot_px = y0_px.max(y1_px);

            let p1 = origin_point(px(x_tick), px(top_px), origin);
            let p2 = origin_point(px(x_tick + band_width), px(bot_px), origin);
            window.paint_quad(fill(mozui::Bounds::from_corners(p1, p2), color));

            // Connector to next bar
            if self.connector && i + 1 < bars.len() {
                if let Some(next_x_tick) =
                    x.tick(&self.data.iter().nth(i + 1).map(|d| x_fn(d)).unwrap())
                {
                    let connector_px = y.tick(&bar.connector_y).unwrap_or(height);
                    let start = origin_point(px(x_tick + band_width), px(connector_px), origin);
                    let end = origin_point(px(next_x_tick), px(connector_px), origin);

                    let mut builder = PathBuilder::stroke(px(1.));
                    builder = builder.dash_array(&[px(4.), px(2.)]);
                    builder.move_to(start);
                    builder.line_to(end);
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, cx.theme().muted_foreground);
                    }
                }
            }
        }
    }
}
