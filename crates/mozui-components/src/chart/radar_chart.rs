use std::f64::consts::{FRAC_PI_2, PI};

use mozui::{App, Bounds, Hsla, PathBuilder, Pixels, Point, Window, point, px};
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        Plot,
        label::{PlotLabel, TEXT_SIZE, Text},
        origin_point,
    },
};

pub struct RadarSeries {
    pub name: String,
    pub values: Vec<f64>,
    pub color: Hsla,
}

#[derive(IntoPlot)]
pub struct RadarChart {
    axes: Vec<String>,
    series: Vec<RadarSeries>,
    max_value: Option<f64>,
    grid_levels: usize,
    fill_opacity: f32,
}

impl Default for RadarChart {
    fn default() -> Self {
        Self {
            axes: vec![],
            series: vec![],
            max_value: None,
            grid_levels: 4,
            fill_opacity: 0.20,
        }
    }
}

impl RadarChart {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn axes(mut self, axes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.axes = axes.into_iter().map(|s| s.into()).collect();
        self
    }

    pub fn series(
        mut self,
        name: impl Into<String>,
        values: Vec<f64>,
        color: impl Into<Hsla>,
    ) -> Self {
        self.series.push(RadarSeries {
            name: name.into(),
            values,
            color: color.into(),
        });
        self
    }

    pub fn max_value(mut self, max: f64) -> Self {
        self.max_value = Some(max);
        self
    }

    pub fn grid_levels(mut self, n: usize) -> Self {
        self.grid_levels = n.max(1);
        self
    }

    pub fn fill_opacity(mut self, opacity: f32) -> Self {
        self.fill_opacity = opacity.clamp(0., 1.);
        self
    }
}

impl Plot for RadarChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let n_axes = self.axes.len();
        if n_axes < 3 || self.series.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32();

        // Leave some padding for axis labels
        let label_pad = 28.0_f32;
        let radius = ((width.min(height)) / 2.0 - label_pad).max(10.0);
        let cx_px = width / 2.0;
        let cy_px = height / 2.0;

        // Global max across all series
        let max_val = self.max_value.unwrap_or_else(|| {
            self.series
                .iter()
                .flat_map(|s| s.values.iter().copied())
                .fold(0.0_f64, f64::max)
                .max(1.0)
        });

        let angle = |i: usize| i as f64 * 2.0 * PI / n_axes as f64 - FRAC_PI_2;

        // Compute outer-edge pixel position for a given axis and normalized radius
        let axis_point = |axis_i: usize, norm: f64| -> Point<Pixels> {
            let a = angle(axis_i);
            let r = norm * radius as f64;
            origin_point(
                px(cx_px + (r * a.cos()) as f32),
                px(cy_px + (r * a.sin()) as f32),
                bounds.origin,
            )
        };

        // --- Grid rings ---
        let grid_color = cx.theme().border;
        for level in 1..=self.grid_levels {
            let norm = level as f64 / self.grid_levels as f64;
            let pts: Vec<Point<Pixels>> = (0..n_axes).map(|i| axis_point(i, norm)).collect();
            let mut builder = PathBuilder::stroke(px(1.));
            builder.move_to(pts[0]);
            for &p in &pts[1..] {
                builder.line_to(p);
            }
            builder.close();
            if let Ok(path) = builder.build() {
                window.paint_path(path, grid_color);
            }
        }

        // --- Axis spokes ---
        let origin_pt = origin_point(px(cx_px), px(cy_px), bounds.origin);
        for i in 0..n_axes {
            let outer = axis_point(i, 1.0);
            let mut builder = PathBuilder::stroke(px(1.));
            builder.move_to(origin_pt);
            builder.line_to(outer);
            if let Ok(path) = builder.build() {
                window.paint_path(path, grid_color);
            }
        }

        // --- Series polygons ---
        let fill_opacity = self.fill_opacity;
        for s in &self.series {
            let pts: Vec<Point<Pixels>> = (0..n_axes)
                .map(|i| {
                    let v = s.values.get(i).copied().unwrap_or(0.0);
                    let norm = (v / max_val).clamp(0., 1.);
                    axis_point(i, norm)
                })
                .collect();

            // Fill
            let mut fill_builder = PathBuilder::fill();
            fill_builder.move_to(pts[0]);
            for &p in &pts[1..] {
                fill_builder.line_to(p);
            }
            fill_builder.close();
            if let Ok(path) = fill_builder.build() {
                window.paint_path(path, s.color.opacity(fill_opacity));
            }

            // Stroke
            let mut stroke_builder = PathBuilder::stroke(px(1.5));
            stroke_builder.move_to(pts[0]);
            for &p in &pts[1..] {
                stroke_builder.line_to(p);
            }
            stroke_builder.close();
            if let Ok(path) = stroke_builder.build() {
                window.paint_path(path, s.color);
            }
        }

        // --- Axis labels ---
        let muted = cx.theme().muted_foreground;
        let label_offset = radius + label_pad * 0.65;
        let texts: Vec<Text> = self
            .axes
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let a = angle(i);
                let lx = cx_px + (label_offset * a.cos() as f32);
                let ly = cy_px + (label_offset * a.sin() as f32);
                let align = if a.cos() > 0.1 {
                    mozui::TextAlign::Left
                } else if a.cos() < -0.1 {
                    mozui::TextAlign::Right
                } else {
                    mozui::TextAlign::Center
                };
                Text::new(name.clone(), point(px(lx), px(ly - TEXT_SIZE / 2.)), muted).align(align)
            })
            .collect();
        PlotLabel::new(texts).paint(&bounds, window, cx);
    }
}
