use std::rc::Rc;

use mozui::{App, Bounds, FontWeight, Hsla, Pixels, SharedString, TextAlign, Window, point, px};
use mozui_components_macros::IntoPlot;
use num_traits::Zero;

use crate::{
    ActiveTheme,
    plot::{
        Plot,
        label::{PlotLabel, TEXT_SIZE, Text},
        shape::{Arc, ArcData, Pie},
    },
};

#[derive(IntoPlot)]
pub struct PieChart<T: 'static> {
    data: Vec<T>,
    inner_radius: f32,
    inner_radius_fn: Option<Rc<dyn Fn(&ArcData<T>) -> f32 + 'static>>,
    outer_radius: f32,
    outer_radius_fn: Option<Rc<dyn Fn(&ArcData<T>) -> f32 + 'static>>,
    pad_angle: f32,
    value: Option<Rc<dyn Fn(&T) -> f32>>,
    color: Option<Rc<dyn Fn(&T) -> Hsla>>,
    center_value: Option<SharedString>,
    center_label: Option<SharedString>,
}

impl<T> PieChart<T> {
    pub fn new<I>(data: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            data: data.into_iter().collect(),
            inner_radius: 0.,
            inner_radius_fn: None,
            outer_radius: 0.,
            outer_radius_fn: None,
            pad_angle: 0.,
            value: None,
            color: None,
            center_value: None,
            center_label: None,
        }
    }

    /// Set the inner radius of the pie chart.
    pub fn inner_radius(mut self, inner_radius: f32) -> Self {
        self.inner_radius = inner_radius;
        self
    }

    /// Set the inner radius of the pie chart based on the arc data.
    pub fn inner_radius_fn(
        mut self,
        inner_radius_fn: impl Fn(&ArcData<T>) -> f32 + 'static,
    ) -> Self {
        self.inner_radius_fn = Some(Rc::new(inner_radius_fn));
        self
    }

    fn get_inner_radius(&self, arc: &ArcData<T>) -> f32 {
        if let Some(inner_radius_fn) = self.inner_radius_fn.as_ref() {
            inner_radius_fn(arc)
        } else {
            self.inner_radius
        }
    }

    /// Set the outer radius of the pie chart.
    pub fn outer_radius(mut self, outer_radius: f32) -> Self {
        self.outer_radius = outer_radius;
        self
    }

    /// Set the outer radius of the pie chart based on the arc data.
    pub fn outer_radius_fn(
        mut self,
        outer_radius_fn: impl Fn(&ArcData<T>) -> f32 + 'static,
    ) -> Self {
        self.outer_radius_fn = Some(Rc::new(outer_radius_fn));
        self
    }

    fn get_outer_radius(&self, arc: &ArcData<T>) -> f32 {
        if let Some(outer_radius_fn) = self.outer_radius_fn.as_ref() {
            outer_radius_fn(arc)
        } else {
            self.outer_radius
        }
    }

    /// Set the pad angle of the pie chart.
    pub fn pad_angle(mut self, pad_angle: f32) -> Self {
        self.pad_angle = pad_angle;
        self
    }

    pub fn value(mut self, value: impl Fn(&T) -> f32 + 'static) -> Self {
        self.value = Some(Rc::new(value));
        self
    }

    /// Set the color of the pie chart.
    pub fn color<H>(mut self, color: impl Fn(&T) -> H + 'static) -> Self
    where
        H: Into<Hsla> + 'static,
    {
        self.color = Some(Rc::new(move |t| color(t).into()));
        self
    }

    /// Large text shown at the center of a donut hole (requires `inner_radius > 0`).
    pub fn center_value(mut self, value: impl Into<SharedString>) -> Self {
        self.center_value = Some(value.into());
        self
    }

    /// Small label shown below `center_value` in the donut hole.
    pub fn center_label(mut self, label: impl Into<SharedString>) -> Self {
        self.center_label = Some(label.into());
        self
    }
}

impl<T> Plot for PieChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(value_fn) = self.value.as_ref() else {
            return;
        };

        let outer_radius = if self.outer_radius.is_zero() {
            bounds.size.height.as_f32() * 0.4
        } else {
            self.outer_radius
        };

        let arc = Arc::new()
            .inner_radius(self.inner_radius)
            .outer_radius(outer_radius);
        let value_fn = value_fn.clone();
        let mut pie = Pie::<T>::new().value(move |d| Some(value_fn(d)));
        pie = pie.pad_angle(self.pad_angle);
        let arcs = pie.arcs(&self.data);

        for a in &arcs {
            let inner_radius = self.get_inner_radius(a);
            let outer_radius = self.get_outer_radius(a);
            arc.paint(
                a,
                if let Some(color_fn) = self.color.as_ref() {
                    color_fn(a.data)
                } else {
                    cx.theme().chart_2
                },
                Some(inner_radius),
                Some(outer_radius),
                &bounds,
                window,
            );
        }

        // Center text — only draw when there is a visible hole
        if self.inner_radius > 0. && (self.center_value.is_some() || self.center_label.is_some()) {
            let cx_px = bounds.size.width.as_f32() / 2.;
            let cy_px = bounds.size.height.as_f32() / 2.;
            let value_size = 14_f32;
            let gap = 3_f32;

            let mut texts = Vec::new();

            if let Some(ref val) = self.center_value {
                // Value is positioned above center, label below
                let y = if self.center_label.is_some() {
                    cy_px - value_size - gap / 2.
                } else {
                    cy_px - value_size / 2.
                };
                texts.push(
                    Text::new(val.clone(), point(px(cx_px), px(y)), cx.theme().foreground)
                        .font_size(px(value_size))
                        .font_weight(FontWeight::SEMIBOLD)
                        .align(TextAlign::Center),
                );
            }

            if let Some(ref lbl) = self.center_label {
                let y = if self.center_value.is_some() {
                    cy_px + gap / 2.
                } else {
                    cy_px - TEXT_SIZE / 2.
                };
                texts.push(
                    Text::new(
                        lbl.clone(),
                        point(px(cx_px), px(y)),
                        cx.theme().muted_foreground,
                    )
                    .font_size(px(TEXT_SIZE))
                    .align(TextAlign::Center),
                );
            }

            PlotLabel::new(texts).paint(&bounds, window, cx);
        }
    }
}
