# Chart Expansion Plan

New chart types following existing visual language. Each section covers data model, builder API, rendering approach, required primitives, and style notes.

---

## Style Reference

Constants inherited across all new charts:

| Constant | Value |
|---|---|
| Grid stroke | `1px`, `[4px, 2px]` dash |
| Axis stroke | `1px` |
| Line stroke | `2px` |
| `AXIS_GAP` | `18px` |
| `TEXT_SIZE` | `10px` |
| `TEXT_HEIGHT` | `12px` |
| Y top margin | `10px` |
| Grid lines | 4 horizontal (0 / 25 / 50 / 75%) |
| Tooltip min-width | `168px`, `p_2`, bg `opacity(0.9)`, radius `theme.radius / 2` |

Color tokens: `chart_1`–`chart_5` (blue ramp light→dark), `chart_bullish`, `chart_bearish`, `warning`, `theme.border`, `theme.muted_foreground`.

Builder pattern via `IntoPlot` macro on all chart types. Sized by parent `div`.

---

## 1. Scatter Chart ✅

> **Implemented** in `crates/mozui-components/src/chart/scatter_chart.rs`

**File:** `chart/scatter_chart.rs`

### Data Model

```rust
pub struct ScatterChart<T: 'static> {
    data: Vec<T>,
    x: Option<Rc<dyn Fn(&T) -> f64>>,
    y: Option<Rc<dyn Fn(&T) -> f64>>,
    color: Option<Rc<dyn Fn(&T) -> Hsla>>,  // per-point color
    dot_size: f32,
    x_ticks: usize,
    x_axis: bool,
    grid: bool,
}
```

### Builder API

```rust
ScatterChart::new(data)
    .x(|d| d.weight)
    .y(|d| d.height)
    .fill(chart_2)                    // constant color for all dots
    .color(|d| match d.cluster {      // OR per-point color accessor
        0 => chart_2,
        1 => bullish,
        _ => bearish,
    })
    .dot_size(5.0)
    .x_axis(true)
    .grid(true)
```

### Rendering

Both axes use `ScaleLinear` with 5% domain padding so edge dots aren't clipped. X axis labels generated at N evenly-spaced tick values across the domain (`x_ticks`, default 5). Each dot: `quad()` with `corner_radius = dot_size / 2` (circle). No stroke — clean filled dots matching `LineChart` dot style.

### Implementation Notes

- `color_fn: Option<Rc<dyn Fn(&T) -> Hsla>>` — set via either `.fill(Hsla)` (captures constant) or `.color(fn)` (per-point)
- Domain padding: `x_pad = (x_max - x_min) * 0.05`, same for Y. Early return if `x_min >= x_max`.
- Dot position: `origin_point(px(x_px - dot_s/2), px(y_px - dot_s/2), bounds.origin)` → `mozui::bounds(top_left, size(dot_px, dot_px))`

---

## 2. Donut Chart ✅

> **Implemented** via `PieChart::new(data).inner_radius(f32).center_value().center_label()`

**File:** Already supported via `PieChart::new(data).inner_radius(f32)`.

`inner_radius > 0` renders a donut. No new chart type needed:

```rust
PieChart::new(segments)
    .value(|d| d.pct)
    .color(|d| d.color)
    .inner_radius(52.)
    .outer_radius(80.)
    .pad_angle(0.025)
    .center_value("84%")     // large semi-bold text in hole
    .center_label("Coverage") // small muted text below
```

Center label renders two `Text` elements via `PlotLabel`:
- Value: `FontWeight::SEMIBOLD`, `14px`, `theme.foreground`
- Label: `TEXT_SIZE = 10px`, `theme.muted_foreground`

---

## 3. Radar Chart ✅

> **Implemented** in `crates/mozui-components/src/chart/radar_chart.rs`

**File:** `chart/radar_chart.rs`

### Data Model

```rust
pub struct RadarChart {
    axes: Vec<String>,
    series: Vec<RadarSeries>,
    max_value: Option<f64>,
    grid_levels: usize,
    fill_opacity: f32,
}

pub struct RadarSeries {
    pub name: String,
    pub values: Vec<f64>,  // one value per axis, same order as .axes()
    pub color: Hsla,
}
```

### Builder API

```rust
RadarChart::new()
    .axes(["Speed", "Power", "Endurance", "Agility", "Technique"])
    .series("Athlete A", vec![82., 65., 74., 88., 77.], chart_2)
    .series("Athlete B", vec![70., 88., 66., 74., 84.], bullish)
    .max_value(100.)       // optional: shared max across all series
    .grid_levels(4)        // concentric polygon rings
    .fill_opacity(0.18)
```

### Rendering

No `ScaleLinear`/`ScaleBand` — polar layout only:

1. **Grid rings**: `grid_levels` concentric regular polygons. Stroke: `1px`, `theme.border`.
2. **Axis spokes**: line from center to each outer vertex. Stroke: `1px`, `theme.border`.
3. **Axis labels**: `TEXT_SIZE = 10px`, `theme.muted_foreground`. Positioned at `radius + label_pad * 0.65`. Alignment: `Left` when `cos(angle) > 0.1`, `Right` when `cos(angle) < -0.1`, `Center` otherwise.
4. **Series polygons**: for each series, compute points via `norm = value / max_value`, then `(cx + norm * radius * cos(angle), cy + norm * radius * sin(angle))`. Fill (semi-transparent) drawn first, then 1.5px stroke.

Angle per axis: `i * 2π / n_axes - π/2` (starts from top).
Outer radius: `(width.min(height) / 2.0 - label_pad).max(10.0)`.

---

## 4. Histogram ✅

> **Implemented** in `crates/mozui-components/src/chart/histogram_chart.rs`

**File:** `chart/histogram_chart.rs`

### Builder API

```rust
HistogramChart::new(data)
    .value(|d| d.response_ms as f64)
    .bins(20)                           // equal-width bins
    .thresholds(vec![0., 50., 100.])   // OR explicit break points
    .fill(chart_2)
    .tick_margin(4)
    .grid(true)
    .x_axis(true)
```

### Rendering

Pre-compute bins in `paint()`:
1. Compute break points: `Count(n)` → evenly spaced; `Thresholds(v)` → sort + insert `min`/`max` if missing.
2. Count values per bin via `breaks.partition_point(|&b| b <= v).saturating_sub(1).min(n_bins-1)` — correctly assigns exact-max values to last bin.
3. X: `ScaleLinear([min_val, max_val], [0, width])`. Y: `ScaleLinear([0, max_count], [height, 10])`.
4. Bars: contiguous quads (no gap — bins touch). X labels at boundary values, `tick_margin` skips.

---

## 5. Heatmap ✅

> **Implemented** in `crates/mozui-components/src/chart/heatmap_chart.rs`

**File:** `chart/heatmap_chart.rs`

### Builder API

```rust
HeatmapChart::new(data)
    .x(|d| d.day.to_string())
    .y(|d| d.hour.to_string())
    .value(|d| d.count as f64)
    .x_order(["Mon","Tue","Wed","Thu","Fri","Sat","Sun"].map(str::to_string))
    .y_order(["06-09","09-12","12-15","15-18","18-21","21-24"].map(str::to_string))
    .low(bullish)      // color at min value; default: chart_bullish (green)
    .mid(warning)      // optional midpoint color; default: none (2-stop lerp)
    .high(bearish)     // color at max value; default: chart_bearish (red)
    .x_axis(true)
```

### Rendering

1. Build ordered category lists from `x_order`/`y_order`, falling back to first-seen order.
2. Cell dimensions computed directly: `cell_w = width / n_cols`, `cell_h = height / n_rows` — avoids `ScaleBand.band_width()` 30px cap.
3. Color: component-wise Hsla lerp. 2-stop: `lerp(low, high, t)`. 3-stop (when `mid` set): `t ≤ 0.5` → `lerp(low, mid, t*2)`, `t > 0.5` → `lerp(mid, high, (t-0.5)*2)`.
4. X axis labels at cell centers. No Y axis labels (future: reserve left margin).

---

## 6. Waterfall Chart ✅

> **Implemented** in `crates/mozui-components/src/chart/waterfall_chart.rs`

**File:** `chart/waterfall_chart.rs`

### Builder API

```rust
WaterfallChart::new(data)
    .x(|d| d.label.to_string())
    .y(|d| d.delta)
    .kind(|d| d.kind)    // if omitted: delta >= 0 → Increase, else Decrease
    .connector(true)
    .grid(true)
    .x_axis(true)
    .tick_margin(1)
```

### Rendering

1. Running total: each bar spans `[running, running + delta]`. `Total` bars span `[0, running]`.
2. Y domain always includes `0.0` explicitly (zero baseline anchoring).
3. Zero baseline: 1px line at `scale_y(0.0)` — stronger than grid, drawn after grid.
4. Connector lines: `PathBuilder::stroke(px(1.)).dash_array([4px, 2px])`, color `theme.muted_foreground` (not `border` — invisible on dark backgrounds).
5. Bar colors: `Increase` → `chart_bullish`, `Decrease` → `chart_bearish`, `Total` → `chart_2`.

---

## 7. Stacked Bar Chart ✅

> **Implemented** in `crates/mozui-components/src/chart/stacked_bar_chart.rs`

**File:** `chart/stacked_bar_chart.rs`

### Builder API

```rust
StackedBarChart::new(data)
    .x(|d| d.month.to_string())
    .series("Organic",  |d: &T| d.organic,  chart_2)
    .series("Direct",   |d: &T| d.direct,   bullish)
    .series("Social",   |d: &T| d.social,   bearish)
    .tick_margin(1)
    .grid(true)
    .x_axis(true)
```

### Implementation Notes

- `T: Clone` required — `Stack<T>` clones data to compute series stacking.
- `X: Clone` required — `ScaleBand<X>` derives `Clone` which requires `X: Clone`.
- Two separate scale clones (`y_scale_y0 = y.clone()`, `y_scale_y1 = y.clone()`) to satisfy move semantics in `Bar.y0()` and `Bar.y1()` closures.

---

## 8. Stacked Area Chart ✅

> **Implemented** in `crates/mozui-components/src/chart/stacked_area_chart.rs`

**File:** `chart/stacked_area_chart.rs`

### Builder API

```rust
StackedAreaChart::new(data)
    .x(|d| d.hour.to_string())
    .series("CPU",     |d: &T| d.cpu,     chart_2, chart_2.opacity(0.45))
    .series("Memory",  |d: &T| d.memory,  bullish, bullish.opacity(0.40))
    .series("Network", |d: &T| d.network, bearish, bearish.opacity(0.35))
    .tick_margin(2)
    .grid(true)
    .x_axis(true)
```

### Implementation Notes

The `Area` primitive only supports a fixed `f32` for `y0` — it cannot vary per-point. Since stacked area requires a variable baseline (the previous series' y1), `Area` is bypassed entirely. Fill and stroke are built manually via `PathBuilder`:

- **Fill path**: `move_to(x0, y1_0)` → forward along top edge → reverse along bottom edge (`points.iter().rev()`) → `close()`
- **Stroke path**: 2px stroke on top edge only

---

## 9. Ridge Line Chart ✅

> **Implemented** in `crates/mozui-components/src/chart/ridge_line_chart.rs`

**File:** `chart/ridge_line_chart.rs`

### Builder API

```rust
RidgeLineChart::new(data)
    .x(|d| d.time.to_string())
    .series("App",   |d: &T| d.app,   chart_2, chart_2.opacity(0.40))
    .series("DB",    |d: &T| d.db,    bullish, bullish.opacity(0.40))
    .series("Cache", |d: &T| d.cache, bearish, bearish.opacity(0.35))
    .overlap(0.6)      // how far each band extends above its lane (fraction of lane_step)
    .tick_margin(2)
    .x_axis(true)
```

### Rendering

Each series gets a horizontal lane:
1. `lane_step = height / n_series` — spacing between baselines.
2. Series `i` baseline: `(i + 1) * lane_step` in pixel coords (top-down).
3. Peak pixel: `(baseline - lane_step * (1.0 + overlap)).max(5.0)` — clamped to chart top.
4. Each series: `ScaleLinear([0, max_val_for_series], [baseline_px, peak_px])`, then `Area` primitive with fixed `y0 = baseline_px`. Catmull-Rom stroke style.

Uses `Area` directly since each series has a fixed (per-series) baseline — unlike stacked area where the baseline varies per data point.

---

## 10. Treemap ✅

> **Implemented** in `crates/mozui-components/src/chart/treemap_chart.rs`

**File:** `chart/treemap_chart.rs`

### Data Model

```rust
pub struct TreemapChart<T> {
    data: Vec<T>,
    value: Rc<dyn Fn(&T) -> f64>,
    label: Rc<dyn Fn(&T) -> SharedString>,
    sublabel: Option<Rc<dyn Fn(&T) -> SharedString>>,
    color: Option<Rc<dyn Fn(&T) -> Hsla>>,
    cell_gap: f32,   // gap between cells, default: 2.
    min_label_w: f32, // minimum cell width for label rendering, default: 60.
    min_label_h: f32, // minimum cell height for label rendering, default: 32.
}
```

### Builder API

```rust
TreemapChart::new(data)
    .value(|d| d.size as f64)
    .label(|d| d.name.clone())
    .sublabel(|d| format!("{:.1}%", d.pct))
    .color(|d| d.color)          // if omitted, cycles chart_1→chart_5
    .cell_gap(2.)
```

### Layout Algorithm: Squarify

The squarify algorithm minimises the aspect ratio (max/min side) of placed rectangles, producing visually balanced tiles. Implemented recursively against a remaining `Bounds<f32>`:

```
fn squarify(items: &[Item], bounds: Rect, result: &mut Vec<TileRect>):
    if items is empty: return
    
    // Determine split axis from longer side
    let horizontal = bounds.width >= bounds.height
    
    // Greedily add items to current row while aspect ratio improves
    let mut row = vec![items[0]]
    for item in items[1..]:
        let trial_row = row + [item]
        if worst_ratio(trial_row, bounds) <= worst_ratio(row, bounds):
            row = trial_row
        else:
            break  // stop adding — ratio is getting worse
    
    // Lay out the current row perpendicular to split axis
    let row_area: f64 = row.iter().map(|i| i.normalized_area).sum()
    let row_thickness = if horizontal {
        row_area / bounds.height  // row occupies this much width
    } else {
        row_area / bounds.width   // row occupies this much height
    }
    
    for (i, item) in row.iter().enumerate():
        let item_len = item.normalized_area / row_thickness
        let item_offset = sum of previous items' item_len values
        result.push(TileRect {
            x: bounds.x + if horizontal { row_thickness already committed } ...,
            // compute final (x, y, w, h) from axis + offset
        })
    
    // Recurse on the remaining bounds after placing this row
    let remaining_bounds = bounds.shrink(row_thickness on the split side)
    squarify(items[row.len()..], remaining_bounds, result)

fn worst_ratio(row: &[Item], bounds: Rect) -> f64:
    let row_area: f64 = row.iter().map(|i| i.area).sum()
    let row_len = row_area / (if horizontal { bounds.height } else { bounds.width })
    row.iter().map(|item| {
        let item_len = item.area / row_len
        f64::max(row_len / item_len, item_len / row_len)
    }).fold(0., f64::max)
```

**Pre-processing:**
1. Filter out zero/negative values.
2. Sort descending by value — squarify produces better aspect ratios starting from largest.
3. Normalize: `normalized_area_i = value_i / total_value * chart_area` (in pixels²).

**Cell inset:** Subtract `cell_gap / 2.` from each edge of the computed `TileRect` before drawing — this creates uniform gaps between all adjacent cells without accumulating double-gaps.

### Drawing

Each tile: `fill(Bounds::from_corners(p1, p2), color)` — no corner radius on tile bodies (unlike heatmap) since cells vary wildly in size and fixed radius reads poorly on very small tiles.

**Labels** (only when tile is large enough):
- Skip rendering if `tile.width < min_label_w (60px)` or `tile.height < min_label_h (32px)`.
- Primary label: `text_sm`, `font_semibold`, `theme.foreground`, drawn at `(tile.x + 8, tile.y + 8)`.
- Sublabel: `text_xs`, `theme.muted_foreground`, `TEXT_GAP = 2px` below primary label.
- Text clipped to tile bounds — no overflow.

**Color assignment** (when no `.color()` accessor):
- Cycle through `[chart_1, chart_2, chart_3, chart_4, chart_5]` by sorted index.
- Optionally darken by depth if hierarchical data is added later.

### Edge Cases

- **Single item**: fills entire bounds. Label always shown.
- **Two items**: degenerate squarify — first item takes proportional slice, second takes remainder. Both labeled if large enough.
- **Very small items** (normalized area < 4px²): skip drawing entirely (invisible, would create paint artifacts).
- **All equal values**: squarify degenerates toward a grid — acceptable, still looks correct.
- **Floating point**: squarify accumulates rounding error on `normalized_area`. Ensure last item's rect is computed as `bounds.remainder` to avoid 1px gaps.

### Style Notes

- No axes, no grid — cells are the structure
- `cell_gap = 2px` matches heatmap for visual consistency between grid-style charts
- Hover: lighten cell fill slightly, tooltip showing label + value + `(value/total * 100)%`

---

## 11. Sankey Diagram ✅

> **Implemented** in `crates/mozui-components/src/chart/sankey_chart.rs`

**File:** `chart/sankey_chart.rs`

Most complex chart. Requires a custom layout engine and fill-width Bezier path rendering.

### Data Model

```rust
pub struct SankeyChart {
    nodes: Vec<SankeyNode>,
    links: Vec<SankeyLink>,
    node_width: f32,      // default: 12.
    node_gap: f32,        // vertical gap between nodes in the same column, default: 8.
    link_opacity: f32,    // default: 0.3
    iterations: usize,    // layout relaxation passes, default: 6
    color: Option<Rc<dyn Fn(&SankeyNode) -> Hsla>>,
}

pub struct SankeyNode {
    pub id: SharedString,
    pub label: SharedString,
}

pub struct SankeyLink {
    pub source: SharedString,
    pub target: SharedString,
    pub value: f64,
}
```

### Builder API

```rust
SankeyChart::new()
    .node("visits",  "Visits")
    .node("direct",  "Direct")
    .node("social",  "Social Search")
    .node("pages",   "Pages/Session")
    .node("bounce",  "Bounce Rate")
    .link("visits",  "direct",  4000.)
    .link("visits",  "social",  2000.)
    .link("direct",  "pages",   3500.)
    .link("social",  "bounce",  1200.)
    .node_width(12.)
    .node_gap(8.)
    .link_opacity(0.3)
    .color(|node| /* map node.id → Hsla */)
```

### Layout Algorithm

The layout runs entirely in `paint()` against the actual `Bounds<Pixels>`. It has two phases: **column assignment** and **vertical positioning**.

#### Phase 1: Column Assignment (Topological Sort)

```
// Assign each node to the leftmost valid column
fn assign_columns(nodes, links) -> HashMap<NodeId, usize>:
    let in_links: HashMap<NodeId, Vec<NodeId>> = build from links
    let mut col = HashMap::new()
    
    // Nodes with no incoming links start at column 0
    for node in nodes where in_links[node].is_empty():
        col[node] = 0
    
    // BFS / Kahn's algorithm forward pass
    let mut queue = deque of col-0 nodes
    while queue not empty:
        let n = queue.pop_front()
        for target in outgoing links from n:
            col[target] = col[target].max(col[n] + 1)
            queue.push_back(target)
    
    return col

// Nodes that are targets of nothing AND sources of nothing are placed in column 0.
// Cycle detection: if a back-edge is found during BFS, break the cycle by treating
// it as a "same column" link (drawn as a curved self-link, skipped in layout).
```

The number of columns = `max(col.values()) + 1`.

#### Phase 2: Vertical Positioning

Node heights are proportional to total flow through them:

```
// Total flow through a node = max(sum of incoming link values, sum of outgoing link values)
// (use max to handle source/sink nodes that only have one side)

// For each column:
//   available_height = chart_height - (n_nodes_in_col - 1) * node_gap
//   node_height_i = (node_flow_i / total_flow_in_col) * available_height

// Position nodes top-to-bottom within each column:
//   y = sum of previous node heights + previous gaps
```

**Relaxation passes** (`iterations` times): iteratively adjust each node's vertical position to minimise link curvature by centering each node on the weighted midpoint of its link endpoints. This is a simplified version of the d3-sankey algorithm:

```
for _ in 0..iterations:
    // Push nodes toward average y-position of their connected links
    for each node:
        let weighted_y = links.iter()
            .filter(|l| l.source == node || l.target == node)
            .map(|l| (link_source_y_mid or link_target_y_mid) * l.value)
            .sum() / total_flow
        node.y = clamp(weighted_y - node.height / 2, 0, col_height - node.height)
    
    // Resolve overlaps (push nodes apart if they intersect)
    sort nodes in column by y, then push apart with node_gap minimum separation
```

#### Phase 3: Link Y-Offsets

Each link occupies a horizontal slice of its source and target node proportional to its value. Track a running `y_offset` per node port:

```
// Sort links by target y-position (top links first) for visual clarity
for source_node, sort outgoing links by target node y:
    link.source_y0 = source_node.y + source_offset
    link.source_y1 = link.source_y0 + link_height
    source_offset += link_height

for target_node, sort incoming links by source node y:
    link.target_y0 = target_node.y + target_offset
    link.target_y1 = link.target_y0 + link_height
    target_offset += link_height
```

### Drawing

#### Links (fill-width Bezier)

Each link is a closed cubic Bezier ribbon (four control points):

```
let cx = bounds.width * 0.4   // horizontal control point offset

// Build closed fill path:
fill_path.move_to(source_right, link.source_y0)
fill_path.cubic_bezier_to(
    (target_left, link.target_y0),
    (source_right + cx, link.source_y0),
    (target_left  - cx, link.target_y0),
)
fill_path.line_to(target_left, link.target_y1)
fill_path.cubic_bezier_to(
    (source_right, link.source_y1),
    (target_left  - cx, link.target_y1),
    (source_right + cx, link.source_y1),
)
fill_path.close()

window.paint_path(fill_path, source_color.opacity(link_opacity))
```

This produces the classic tapered ribbon look: wider where flow is large, narrowing as links merge at nodes.

#### Nodes

```rust
let p1 = origin_point(px(node.x), px(node.y), origin);
let p2 = origin_point(px(node.x + node_width), px(node.y + node.height), origin);
window.paint_quad(fill(Bounds::from_corners(p1, p2), color));
```

No corner radius — node bars should look like deliberate vertical dividers.

#### Labels

- **Leftmost column** (source nodes): label rendered to the LEFT of node bar. Right-aligned, vertically centered on node.
- **Rightmost column** (sink nodes): label to the RIGHT of node bar. Left-aligned.
- **Middle columns**: label inside node bar if `node_width > 40px`, else omit.
- Font: `TEXT_SIZE = 10px`, `theme.muted_foreground`.

### Edge Cases

- **Cycles**: detect with DFS back-edge check before layout. Remove cycle-creating links from the layout pass; draw them as thin curved arcs above the diagram (or skip entirely with a console warning).
- **Disconnected nodes**: nodes with no links still get a column assignment (column 0) and are drawn but produce no links.
- **Zero-value links**: skip drawing (would produce zero-height ribbons).
- **Single column**: degenerate — all nodes stacked vertically with no links crossing columns. Still renders correctly.
- **Very tall nodes**: if a node would exceed `chart_height`, clamp and scale all nodes in that column uniformly.

### Style Notes

- Link fill (not stroke) with opacity is standard Sankey — avoids harsh edges on the ribbon
- `node_width = 12.` matches the visual weight of bar chart bars at minimum width
- Relaxation `iterations = 6` is the d3-sankey default — more passes improve layout at marginal cost
- Curved links reuse `PathBuilder::cubic_bezier_to` already present in the `Area` spline rendering

---

## Phased Implementation Plan

### Phase 1 — Existing primitive reuse ✅ Complete

| Status | Chart | Notes |
|---|---|---|
| ✅ | **Donut center label** | Extended `PieChart` with `center_value` / `center_label`. Two `Text` draws via `PlotLabel`. |
| ✅ | **Histogram** | Bin pre-computation + `ScaleLinear` X + contiguous `quad` bars. `partition_point` edge-case fix. |
| ✅ | **Stacked Bar** | `Stack<T>` → `Bar<T>`. Required `T: Clone`, `X: Clone`, and two separate `y_scale` clones. |
| ✅ | **Stacked Area** | `Stack<T>` + manual `PathBuilder` (bypasses `Area` fixed-y0 limitation). |
| ✅ | **Waterfall** | Floating `Bar` + running total + connector lines. `muted_foreground` for connector visibility. |

### Phase 2 — Simple new primitives ✅ Complete

| Status | Chart | Notes |
|---|---|---|
| ✅ | **Scatter** | Dual `ScaleLinear`, per-point `color` accessor, `quad()` dots. 5% domain padding. |
| ✅ | **Ridge Line** | `Area` primitive per series with per-series fixed baseline. `overlap` factor controls layering. |
| ✅ | **Radar** | Polar math, `PathBuilder` fill + stroke polygons, `PlotLabel` for axis labels. No new primitive. |
| ✅ | **Heatmap** | Direct cell sizing (bypasses `band_width` 30px cap). 3-stop Hsla lerp via `.mid()`. |

### Phase 3 — Complex layout algorithms

| Status | Chart | Why hard |
|---|---|---|
| ✅ | **Treemap** | Squarify recursive layout. Needs careful bounds subdivision, aspect-ratio minimisation, and label clipping. See detailed spec above. |
| ✅ | **Sankey** | Topological column assignment + per-column node stacking + relaxation passes + cubic Bezier fill-width ribbons. Most complex layout in the set. See detailed spec above. |

---

## New Primitives Required

| Primitive | Status | File | Notes |
|---|---|---|---|
| `Scatter<T>` | ✅ Inlined in chart | `chart/scatter_chart.rs` | Dot positioning via `quad()`, no separate shape file needed |
| `Radar<T>` | ✅ Inlined in chart | `chart/radar_chart.rs` | Polar polygon via `PathBuilder` directly |
| `Heatmap<T>` | ✅ Inlined in chart | `chart/heatmap_chart.rs` | Grid of quads, Hsla lerp |
| `Treemap<T>` | ✅ Inlined in chart | `chart/treemap_chart.rs` | Squarify layout algorithm inlined — no separate shape file needed |
| `Sankey` | ✅ Inlined in chart | `chart/sankey_chart.rs` | Node layout + Bezier link fills inlined — no separate shape file needed |

Charts reusing existing primitives without new shape files: `HistogramChart` (`quad` directly), `WaterfallChart` (`quad` + `PathBuilder`), `StackedBarChart` (`Stack` + `Bar`), `StackedAreaChart` (`Stack` + `PathBuilder`), `RidgeLineChart` (`Area`), `DonutChart` (existing `PieChart`).

---

## Theme Additions

No new color tokens needed — `chart_1`–`chart_5`, `chart_bullish`, `chart_bearish`, `warning`, `theme.border`, `theme.muted_foreground` cover all cases.

Optional future addition: `chart_neutral` (gray tone for `WaterfallKind::Total` bars). Falls back to `chart_2` if absent.
