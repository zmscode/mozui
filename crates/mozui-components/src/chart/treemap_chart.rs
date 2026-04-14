use std::rc::Rc;

use mozui::{App, Bounds, FontWeight, Hsla, Pixels, SharedString, Window, fill, hsla, point, px};
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        Plot,
        label::{PlotLabel, TEXT_GAP, TEXT_SIZE, Text},
        origin_point,
    },
};

// ---------------------------------------------------------------------------
// Layout types
// ---------------------------------------------------------------------------

struct Item {
    normalized_area: f64,
    original_index: usize,
}

struct TileRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

// ---------------------------------------------------------------------------
// Squarify layout algorithm
// ---------------------------------------------------------------------------

fn worst_ratio(row: &[Item], side: f64) -> f64 {
    if side <= 0.0 {
        return f64::MAX;
    }
    let row_area: f64 = row.iter().map(|i| i.normalized_area).sum();
    let row_thickness = row_area / side;
    if row_thickness <= 0.0 {
        return f64::MAX;
    }
    row.iter()
        .map(|item| {
            let item_len = item.normalized_area / row_thickness;
            if item_len <= 0.0 {
                f64::MAX
            } else {
                f64::max(row_thickness / item_len, item_len / row_thickness)
            }
        })
        .fold(0.0_f64, f64::max)
}

fn squarify(items: &[Item], x: f64, y: f64, w: f64, h: f64, result: &mut Vec<(usize, TileRect)>) {
    if items.is_empty() {
        return;
    }
    if w <= 0.0 || h <= 0.0 {
        return;
    }

    // Single item remaining: fill exact remaining bounds to avoid float gaps.
    if items.len() == 1 {
        result.push((
            items[0].original_index,
            TileRect {
                x: x as f32,
                y: y as f32,
                w: w as f32,
                h: h as f32,
            },
        ));
        return;
    }

    let horizontal = w >= h;
    let side = if horizontal { h } else { w };

    // Greedily extend row while aspect ratio improves.
    let mut row_end = 1;
    while row_end < items.len() {
        let current = worst_ratio(&items[..row_end], side);
        let trial = worst_ratio(&items[..row_end + 1], side);
        if trial <= current {
            row_end += 1;
        } else {
            break;
        }
    }

    let row = &items[..row_end];
    let row_area: f64 = row.iter().map(|i| i.normalized_area).sum();
    let row_thickness = (row_area / side).min(if horizontal { w } else { h });

    let mut offset = 0.0f64;
    for item in row {
        let item_len = item.normalized_area / row_thickness;
        let (tx, ty, tw, th) = if horizontal {
            (x, y + offset, row_thickness, item_len)
        } else {
            (x + offset, y, item_len, row_thickness)
        };
        result.push((
            item.original_index,
            TileRect {
                x: tx as f32,
                y: ty as f32,
                w: tw as f32,
                h: th as f32,
            },
        ));
        offset += item_len;
    }

    let (rx, ry, rw, rh) = if horizontal {
        (x + row_thickness, y, w - row_thickness, h)
    } else {
        (x, y + row_thickness, w, h - row_thickness)
    };

    squarify(&items[row_end..], rx, ry, rw, rh, result);
}

// ---------------------------------------------------------------------------
// Chart
// ---------------------------------------------------------------------------

#[derive(IntoPlot)]
pub struct TreemapChart<T: 'static> {
    data: Vec<T>,
    value: Option<Rc<dyn Fn(&T) -> f64>>,
    label: Option<Rc<dyn Fn(&T) -> SharedString>>,
    sublabel: Option<Rc<dyn Fn(&T) -> SharedString>>,
    color: Option<Rc<dyn Fn(&T) -> Hsla>>,
    cell_gap: f32,
    min_label_w: f32,
    min_label_h: f32,
}

impl<T> TreemapChart<T> {
    pub fn new(data: impl IntoIterator<Item = T>) -> Self {
        Self {
            data: data.into_iter().collect(),
            value: None,
            label: None,
            sublabel: None,
            color: None,
            cell_gap: 2.,
            min_label_w: 60.,
            min_label_h: 32.,
        }
    }

    pub fn value(mut self, f: impl Fn(&T) -> f64 + 'static) -> Self {
        self.value = Some(Rc::new(f));
        self
    }

    pub fn label(mut self, f: impl Fn(&T) -> SharedString + 'static) -> Self {
        self.label = Some(Rc::new(f));
        self
    }

    pub fn sublabel(mut self, f: impl Fn(&T) -> SharedString + 'static) -> Self {
        self.sublabel = Some(Rc::new(f));
        self
    }

    /// Per-item color accessor. If omitted, cycles through `chart_1`–`chart_5`.
    pub fn color(mut self, f: impl Fn(&T) -> Hsla + 'static) -> Self {
        self.color = Some(Rc::new(f));
        self
    }

    pub fn cell_gap(mut self, gap: f32) -> Self {
        self.cell_gap = gap;
        self
    }
}

/// Returns (label_color, sublabel_color) with sufficient contrast against `bg`.
///
/// Uses HSL lightness as a perceptual brightness proxy. L > 0.55 → light
/// background → dark text; otherwise light text.
fn contrast_text_colors(bg: Hsla) -> (Hsla, Hsla) {
    if bg.l > 0.55 {
        (hsla(0., 0., 0.08, 0.90), hsla(0., 0., 0.08, 0.55))
    } else {
        (hsla(0., 0., 1.00, 0.92), hsla(0., 0., 1.00, 0.60))
    }
}

impl<T> Plot for TreemapChart<T> {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(val_fn) = self.value.as_ref() else {
            return;
        };
        if self.data.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32();
        let chart_area = width as f64 * height as f64;

        // Filter, sort descending, normalize areas.
        let mut indexed: Vec<(usize, f64)> = self
            .data
            .iter()
            .enumerate()
            .filter_map(|(i, d)| {
                let v = val_fn(d);
                if v > 0.0 { Some((i, v)) } else { None }
            })
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let total: f64 = indexed.iter().map(|(_, v)| v).sum();
        if total <= 0.0 {
            return;
        }

        let items: Vec<Item> = indexed
            .iter()
            .map(|(orig_idx, v)| Item {
                normalized_area: v / total * chart_area,
                original_index: *orig_idx,
            })
            .collect();

        let mut tiles: Vec<(usize, TileRect)> = Vec::with_capacity(items.len());
        squarify(&items, 0.0, 0.0, width as f64, height as f64, &mut tiles);

        let chart_colors = [
            cx.theme().chart_1,
            cx.theme().chart_2,
            cx.theme().chart_3,
            cx.theme().chart_4,
            cx.theme().chart_5,
        ];
        let origin = bounds.origin;
        let inset = self.cell_gap / 2.;
        let min_label_w = self.min_label_w;
        let min_label_h = self.min_label_h;

        let mut label_texts: Vec<Text> = vec![];

        for (sorted_pos, (orig_idx, tile)) in tiles.iter().enumerate() {
            // Skip invisible tiles.
            if tile.w * tile.h < 4.0 {
                continue;
            }

            let ix = tile.x + inset;
            let iy = tile.y + inset;
            let iw = (tile.w - self.cell_gap).max(0.);
            let ih = (tile.h - self.cell_gap).max(0.);

            if iw <= 0. || ih <= 0. {
                continue;
            }

            let d = &self.data[*orig_idx];
            let color = self
                .color
                .as_ref()
                .map(|f| f(d))
                .unwrap_or(chart_colors[sorted_pos % 5]);

            let p1 = origin_point(px(ix), px(iy), origin);
            let p2 = origin_point(px(ix + iw), px(iy + ih), origin);
            window.paint_quad(fill(mozui::Bounds::from_corners(p1, p2), color));

            if iw >= min_label_w && ih >= min_label_h {
                let label_x = ix + 8.;
                let label_y = iy + 8.;
                let (label_color, sub_color) = contrast_text_colors(color);

                if let Some(label_fn) = &self.label {
                    label_texts.push(
                        Text::new(label_fn(d), point(px(label_x), px(label_y)), label_color)
                            .font_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD),
                    );

                    if let Some(sub_fn) = &self.sublabel {
                        let sub_y = label_y + 12. + TEXT_GAP;
                        label_texts.push(
                            Text::new(sub_fn(d), point(px(label_x), px(sub_y)), sub_color)
                                .font_size(px(TEXT_SIZE)),
                        );
                    }
                }
            }
        }

        if !label_texts.is_empty() {
            PlotLabel::new(label_texts).paint(&bounds, window, cx);
        }
    }
}
