use std::collections::VecDeque;
use std::rc::Rc;

use mozui::{
    App, Bounds, Hsla, PathBuilder, Pixels, SharedString, TextAlign, Window, fill, point, px,
};
use mozui_components_macros::IntoPlot;

use crate::{
    ActiveTheme,
    plot::{
        Plot,
        label::{PlotLabel, TEXT_SIZE, Text},
        origin_point,
    },
};

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

pub struct SankeyNode {
    pub id: SharedString,
    pub label: SharedString,
}

pub struct SankeyLink {
    pub source: SharedString,
    pub target: SharedString,
    pub value: f64,
}

// ---------------------------------------------------------------------------
// Chart
// ---------------------------------------------------------------------------

#[derive(IntoPlot)]
pub struct SankeyChart {
    nodes: Vec<SankeyNode>,
    links: Vec<SankeyLink>,
    node_width: f32,
    node_gap: f32,
    link_opacity: f32,
    iterations: usize,
    color: Option<Rc<dyn Fn(&SankeyNode) -> Hsla>>,
}

impl Default for SankeyChart {
    fn default() -> Self {
        Self::new()
    }
}

impl SankeyChart {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            links: vec![],
            node_width: 12.,
            node_gap: 8.,
            link_opacity: 0.45,
            iterations: 6,
            color: None,
        }
    }

    pub fn node(mut self, id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        self.nodes.push(SankeyNode {
            id: id.into(),
            label: label.into(),
        });
        self
    }

    pub fn link(
        mut self,
        source: impl Into<SharedString>,
        target: impl Into<SharedString>,
        value: f64,
    ) -> Self {
        self.links.push(SankeyLink {
            source: source.into(),
            target: target.into(),
            value,
        });
        self
    }

    pub fn node_width(mut self, w: f32) -> Self {
        self.node_width = w.max(1.);
        self
    }

    pub fn node_gap(mut self, g: f32) -> Self {
        self.node_gap = g.max(0.);
        self
    }

    pub fn link_opacity(mut self, o: f32) -> Self {
        self.link_opacity = o.clamp(0., 1.);
        self
    }

    pub fn iterations(mut self, n: usize) -> Self {
        self.iterations = n;
        self
    }

    pub fn color(mut self, f: impl Fn(&SankeyNode) -> Hsla + 'static) -> Self {
        self.color = Some(Rc::new(f));
        self
    }
}

// ---------------------------------------------------------------------------
// Internal layout types
// ---------------------------------------------------------------------------

struct LNode {
    col: usize,
    x: f32,
    y: f32,
    height: f32,
}

struct LLink {
    src: usize,
    tgt: usize,
    value: f64,
    height: f32,
    src_y0: f32,
    src_y1: f32,
    tgt_y0: f32,
    tgt_y1: f32,
}

// ---------------------------------------------------------------------------
// Plot impl
// ---------------------------------------------------------------------------

impl Plot for SankeyChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let n_nodes = self.nodes.len();
        if n_nodes == 0 {
            return;
        }

        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32();
        let node_width = self.node_width;
        let node_gap = self.node_gap;
        let o = bounds.origin;

        // Build node id → index map.
        let node_idx: std::collections::HashMap<SharedString, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();

        // Resolve links: skip zero-value, self-loops, unknown ids.
        struct RawLink {
            src: usize,
            tgt: usize,
            value: f64,
        }
        let raw: Vec<RawLink> = self
            .links
            .iter()
            .filter_map(|l| {
                if l.value <= 0. {
                    return None;
                }
                let src = *node_idx.get(&l.source)?;
                let tgt = *node_idx.get(&l.target)?;
                if src == tgt {
                    return None;
                }
                Some(RawLink {
                    src,
                    tgt,
                    value: l.value,
                })
            })
            .collect();

        // Build adjacency lists.
        let mut out_adj: Vec<Vec<usize>> = vec![vec![]; n_nodes];
        let mut in_adj: Vec<Vec<usize>> = vec![vec![]; n_nodes];
        for (li, rl) in raw.iter().enumerate() {
            out_adj[rl.src].push(li);
            in_adj[rl.tgt].push(li);
        }

        // --- Phase 1: Column assignment via BFS ---
        let mut col: Vec<usize> = vec![0; n_nodes];
        let mut queue: VecDeque<usize> = VecDeque::new();

        for ni in 0..n_nodes {
            if in_adj[ni].is_empty() {
                queue.push_back(ni);
            }
        }

        // Cap iterations to prevent infinite loops on cyclic inputs.
        let mut budget = n_nodes * n_nodes + raw.len() + 1;
        while let Some(ni) = queue.pop_front() {
            if budget == 0 {
                break;
            }
            budget -= 1;

            for &li in &out_adj[ni] {
                let tgt = raw[li].tgt;
                let new_col = col[ni] + 1;
                if new_col > col[tgt] {
                    col[tgt] = new_col;
                    queue.push_back(tgt);
                }
            }
        }

        let n_cols = col.iter().copied().max().unwrap_or(0) + 1;

        // Column → sorted node list (insertion order for initial stacking).
        let mut col_nodes: Vec<Vec<usize>> = vec![vec![]; n_cols];
        for ni in 0..n_nodes {
            col_nodes[col[ni]].push(ni);
        }

        // --- Phase 2: Node heights and initial vertical positions ---
        let mut node_flow: Vec<f64> = vec![0.; n_nodes];
        for ni in 0..n_nodes {
            let in_f: f64 = in_adj[ni].iter().map(|&li| raw[li].value).sum();
            let out_f: f64 = out_adj[ni].iter().map(|&li| raw[li].value).sum();
            node_flow[ni] = f64::max(in_f, out_f).max(1e-9);
        }

        // Global scale: fit the column that needs the most space.
        let global_scale = col_nodes
            .iter()
            .filter(|nodes| !nodes.is_empty())
            .map(|nodes| {
                let total_flow: f64 = nodes.iter().map(|&ni| node_flow[ni]).sum();
                let avail = (height - (nodes.len() as f32 - 1.).max(0.) * node_gap).max(1.);
                avail / total_flow as f32
            })
            .fold(f32::MAX, f32::min);
        let global_scale = if global_scale == f32::MAX {
            1.0
        } else {
            global_scale
        };

        // Reserve horizontal margins so left/right labels stay within bounds.
        // Nodes are laid out within [label_margin, width - label_margin].
        let label_margin = 80.;
        let inner_w = (width - label_margin * 2. - node_width).max(0.);

        // X positions: evenly space columns within the inner area.
        let col_x: Vec<f32> = (0..n_cols)
            .map(|c| {
                if n_cols == 1 {
                    label_margin
                } else {
                    label_margin + c as f32 * inner_w / (n_cols - 1) as f32
                }
            })
            .collect();

        let mut lnodes: Vec<LNode> = (0..n_nodes)
            .map(|ni| LNode {
                col: col[ni],
                x: col_x[col[ni]],
                y: 0.,
                height: (node_flow[ni] as f32 * global_scale).max(1.),
            })
            .collect();

        // Initial top-to-bottom stacking per column.
        for c in 0..n_cols {
            let mut y = 0.;
            for &ni in &col_nodes[c] {
                lnodes[ni].y = y;
                y += lnodes[ni].height + node_gap;
            }
        }

        // Build layout links (height computed from same global scale).
        let mut llinks: Vec<LLink> = raw
            .iter()
            .map(|rl| {
                let h = (rl.value as f32 * global_scale).max(0.5);
                LLink {
                    src: rl.src,
                    tgt: rl.tgt,
                    value: rl.value,
                    height: h,
                    src_y0: 0.,
                    src_y1: h,
                    tgt_y0: 0.,
                    tgt_y1: h,
                }
            })
            .collect();

        // --- Relaxation passes ---
        for _ in 0..self.iterations {
            // Nudge each node toward the weighted midpoint of its connected links.
            for ni in 0..n_nodes {
                let mut wsum = 0.0f64;
                let mut wtot = 0.0f64;

                for &li in &out_adj[ni] {
                    let t = llinks[li].tgt;
                    let mid = (lnodes[t].y + lnodes[t].height / 2.) as f64;
                    wsum += mid * llinks[li].value;
                    wtot += llinks[li].value;
                }
                for &li in &in_adj[ni] {
                    let s = llinks[li].src;
                    let mid = (lnodes[s].y + lnodes[s].height / 2.) as f64;
                    wsum += mid * llinks[li].value;
                    wtot += llinks[li].value;
                }

                if wtot > 0. {
                    let target_mid = (wsum / wtot) as f32;
                    lnodes[ni].y = (target_mid - lnodes[ni].height / 2.)
                        .max(0.)
                        .min((height - lnodes[ni].height).max(0.));
                }
            }

            // Resolve overlaps per column.
            for c in 0..n_cols {
                col_nodes[c].sort_by(|&a, &b| {
                    lnodes[a]
                        .y
                        .partial_cmp(&lnodes[b].y)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                // Push down to clear overlap.
                let mut cursor = 0.;
                for &ni in &col_nodes[c] {
                    if lnodes[ni].y < cursor {
                        lnodes[ni].y = cursor;
                    }
                    cursor = lnodes[ni].y + lnodes[ni].height + node_gap;
                }

                // If overflowing bottom, push up from bottom.
                if cursor - node_gap > height {
                    let mut cursor = height;
                    for &ni in col_nodes[c].iter().rev() {
                        let bottom = lnodes[ni].y + lnodes[ni].height;
                        if bottom > cursor {
                            lnodes[ni].y = (cursor - lnodes[ni].height).max(0.);
                        }
                        cursor = lnodes[ni].y - node_gap;
                    }
                }
            }
        }

        // --- Phase 3: Link Y-offsets ---

        // Source side: sort outgoing links by target y, assign offsets from node top.
        let mut src_cursor: Vec<f32> = lnodes.iter().map(|n| n.y).collect();
        for ni in 0..n_nodes {
            let mut outgoing = out_adj[ni].clone();
            outgoing.sort_by(|&a, &b| {
                lnodes[llinks[a].tgt]
                    .y
                    .partial_cmp(&lnodes[llinks[b].tgt].y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for li in outgoing {
                llinks[li].src_y0 = src_cursor[ni];
                llinks[li].src_y1 = src_cursor[ni] + llinks[li].height;
                src_cursor[ni] += llinks[li].height;
            }
        }

        // Target side: sort incoming links by source y, assign offsets from node top.
        let mut tgt_cursor: Vec<f32> = lnodes.iter().map(|n| n.y).collect();
        for ni in 0..n_nodes {
            let mut incoming = in_adj[ni].clone();
            incoming.sort_by(|&a, &b| {
                lnodes[llinks[a].src]
                    .y
                    .partial_cmp(&lnodes[llinks[b].src].y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for li in incoming {
                llinks[li].tgt_y0 = tgt_cursor[ni];
                llinks[li].tgt_y1 = tgt_cursor[ni] + llinks[li].height;
                tgt_cursor[ni] += llinks[li].height;
            }
        }

        // --- Node colors ---
        let chart_colors = [
            cx.theme().chart_1,
            cx.theme().chart_2,
            cx.theme().chart_3,
            cx.theme().chart_4,
            cx.theme().chart_5,
        ];
        let node_colors: Vec<Hsla> = (0..n_nodes)
            .map(|ni| {
                self.color
                    .as_ref()
                    .map(|f| f(&self.nodes[ni]))
                    .unwrap_or(chart_colors[ni % 5])
            })
            .collect();

        // --- Draw: links behind nodes ---
        //
        // curveBumpX style (d3-sankey standard): both control points sit at the
        // horizontal midpoint between source-right and target-left, producing
        // horizontal tangents at both ends. Curvature is proportional to the
        // actual gap between nodes, not the total chart width.
        let link_opacity = self.link_opacity;

        for ll in &llinks {
            if ll.height < 0.5 {
                continue;
            }
            let src_right = lnodes[ll.src].x + node_width;
            let tgt_left = lnodes[ll.tgt].x;
            let mid_x = (src_right + tgt_left) / 2.0;
            let color = node_colors[ll.src].opacity(link_opacity);

            let mut path = PathBuilder::fill();
            // Top edge: horizontal departure from source, horizontal arrival at target.
            path.move_to(origin_point(px(src_right), px(ll.src_y0), o));
            path.cubic_bezier_to(
                origin_point(px(tgt_left), px(ll.tgt_y0), o),
                origin_point(px(mid_x), px(ll.src_y0), o),
                origin_point(px(mid_x), px(ll.tgt_y0), o),
            );
            // Bottom edge: reverse direction to close the ribbon.
            path.line_to(origin_point(px(tgt_left), px(ll.tgt_y1), o));
            path.cubic_bezier_to(
                origin_point(px(src_right), px(ll.src_y1), o),
                origin_point(px(mid_x), px(ll.tgt_y1), o),
                origin_point(px(mid_x), px(ll.src_y1), o),
            );
            path.close();
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        }

        // --- Draw: nodes ---
        for (ni, ln) in lnodes.iter().enumerate() {
            let p1 = origin_point(px(ln.x), px(ln.y), o);
            let p2 = origin_point(px(ln.x + node_width), px(ln.y + ln.height), o);
            window.paint_quad(fill(mozui::Bounds::from_corners(p1, p2), node_colors[ni]));
        }

        // --- Draw: labels ---
        // Labels sit within the margin zones, clamped to chart bounds.
        // Left column:  right-aligned, drawn inside [0, label_margin - pad]
        // Right column: left-aligned,  drawn inside [width - label_margin + pad, width]
        let max_col = n_cols.saturating_sub(1);
        let muted = cx.theme().muted_foreground;
        let label_pad = 6.;
        let mut label_texts: Vec<Text> = vec![];

        for (ni, ln) in lnodes.iter().enumerate() {
            let node = &self.nodes[ni];
            // Vertically center label on node.
            let label_y = ln.y + ln.height / 2. - TEXT_SIZE / 2.;

            let (label_x, align) = if ln.col == 0 {
                // Right-align toward the node from within the left margin.
                (ln.x - label_pad, TextAlign::Right)
            } else if ln.col == max_col {
                // Left-align outward from the node into the right margin.
                (ln.x + node_width + label_pad, TextAlign::Left)
            } else if node_width > 40. {
                (ln.x + node_width / 2., TextAlign::Center)
            } else {
                continue;
            };

            label_texts.push(
                Text::new(node.label.clone(), point(px(label_x), px(label_y)), muted)
                    .font_size(px(TEXT_SIZE))
                    .align(align),
            );
        }

        if !label_texts.is_empty() {
            PlotLabel::new(label_texts).paint(&bounds, window, cx);
        }
    }
}
