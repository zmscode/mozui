# mozui-canvas — Infinite Canvas Component

A reusable pan/zoom canvas component for mozui. The document renders normally via flexbox inside a viewport that the user can pan and zoom around, with a dot grid background.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  CanvasViewport (clips to bounds, handles input)    │
│                                                     │
│  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐ │
│  │  Dot grid background (canvas() element)        │ │
│  │  Painted at viewport bounds, offset by pan     │ │
│  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘ │
│                                                     │
│  ┌─────────────────────────────────────┐            │
│  │  Content layer (translated + scaled) │            │
│  │  ┌─────────────────────────────┐    │            │
│  │  │  Children (flexbox document) │    │            │
│  │  └─────────────────────────────┘    │            │
│  └─────────────────────────────────────┘            │
│                                                     │
│  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐ │
│  │  Selection overlay (marquee rect)              │ │
│  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘ │
└─────────────────────────────────────────────────────┘
```

The canvas is a **viewport** over a larger coordinate space. The children (document tree) render via normal flexbox layout. The viewport applies a **pan offset** and **zoom scale** to position them, and paints a dot grid behind everything.

---

## State

```rust
pub struct CanvasState {
    /// Current pan offset in canvas-space pixels.
    /// (0, 0) = document origin is at viewport center.
    pub offset: Point<Pixels>,

    /// Current zoom level. 1.0 = 100%.
    /// Clamped to [MIN_ZOOM, MAX_ZOOM].
    pub zoom: f32,

    /// Active interaction mode.
    pub interaction: InteractionMode,
}

pub enum InteractionMode {
    /// Default — clicks pass through to children.
    None,
    /// Panning the canvas (middle-click drag or space+drag).
    Panning { start_offset: Point<Pixels>, start_mouse: Point<Pixels> },
    /// Drawing a marquee selection rectangle.
    Marquee { origin: Point<Pixels>, current: Point<Pixels> },
}

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 5.0;
const DEFAULT_ZOOM: f32 = 1.0;
const DOT_SPACING: f32 = 20.0;
const DOT_SIZE: f32 = 1.5;
```

---

## Coordinate Spaces

Two coordinate spaces:

- **Screen space** — pixel coordinates relative to the viewport element's top-left. Mouse events arrive in this space.
- **Canvas space** — the infinite coordinate space where the document lives. `(0, 0)` is the document origin.

Conversions:

```
screen_to_canvas(point) = (point - viewport_center - offset) / zoom
canvas_to_screen(point) = (point * zoom) + offset + viewport_center
```

The viewport center is used as the origin so that zoom scales around the center of the visible area by default.

---

## Phases

### Phase 1 — Dot grid + pan

**Goal:** Replace the current static canvas with a pannable viewport and dot grid background.

1. Create `CanvasState` struct holding `offset`, `zoom`, `interaction`.
2. Move canvas state into `BuilderView` (or a dedicated entity).
3. Render the dot grid using mozui's `canvas()` element:
   - In the `paint` callback, compute visible grid positions from `offset`, `zoom`, and viewport bounds.
   - Paint each dot as a `paint_quad()` with corner radii = half size (circle).
   - Dot color: subtle gray, e.g. `hsla(0, 0%, 30%, 0.4)`.
   - Dots scale with zoom (spacing increases when zoomed in, decreases when zoomed out).
4. Implement pan via middle-click drag:
   - `on_mouse_down(MouseButton::Middle)` → enter `Panning` mode, record start position.
   - `on_mouse_move` while panning → update `offset` by delta.
   - `on_mouse_up(MouseButton::Middle)` → exit panning mode.
5. Also support space+left-click drag for pan (common convention).
6. Position the document content using the offset:
   - Wrap the rendered document tree in a div with `position: absolute`, `left: offset.x`, `top: offset.y`.
   - Apply scale via CSS transform if available, or adjust sizes manually.

**Deliverable:** Canvas has a dot grid background. Middle-click or space+drag pans the view.

### Phase 2 — Zoom

**Goal:** Cmd+scroll and pinch-to-zoom, zoom toward cursor.

1. Handle `on_scroll_wheel` with Cmd held:
   - Extract delta from `ScrollDelta::Pixels` or `ScrollDelta::Lines`.
   - Compute new zoom: `zoom * (1.0 + delta.y * ZOOM_SENSITIVITY)`.
   - Clamp to `[MIN_ZOOM, MAX_ZOOM]`.
2. Handle `on_pinch` for trackpad:
   - `PinchEvent.delta` is the zoom delta directly.
   - Same zoom computation.
3. Zoom toward cursor position:
   - Before zoom: compute cursor position in canvas space.
   - Apply new zoom.
   - After zoom: compute where that canvas point would now appear on screen.
   - Adjust offset so the canvas point stays under the cursor.
   - Formula: `offset += (cursor_screen - viewport_center) * (1.0 - old_zoom / new_zoom)`
4. Without Cmd, scroll wheel should pan (two-finger scroll = pan on trackpad).
5. Show zoom percentage in the canvas toolbar or corner.
6. Add zoom-to-fit button: compute the bounding box of the document, set zoom and offset to fit it in the viewport with some padding.

**Deliverable:** Pinch/Cmd+scroll zooms toward cursor. Scroll pans. Zoom indicator visible.

### Phase 3 — Marquee selection

**Goal:** Click+drag on empty canvas area draws a selection rectangle, selects all nodes within it.

1. Change `BuilderView.selected` from `Option<NodeId>` to `HashSet<NodeId>`.
   - Update all code that reads selection to handle multi-select.
   - Property panel shows "N items selected" or shared properties.
   - Tree view highlights all selected nodes.
2. On left-click drag starting on empty canvas (not on a node):
   - Enter `Marquee` mode, record origin in screen space.
   - `on_mouse_move` → update `current` corner.
   - Paint the marquee rectangle as a semi-transparent overlay via `canvas()` or a positioned div.
3. On mouse up:
   - Convert marquee rect to canvas space.
   - Hit-test: for each node, check if its rendered bounds intersect the marquee rect.
   - Set `selected` to the set of intersecting nodes.
4. Shift+click on a node toggles it in/out of the selection set.
5. Click on empty area (without drag) clears selection.

**Deliverable:** Can drag to select multiple nodes. Shift+click adds to selection. Multi-selection reflected in tree view and property panel.

### Phase 4 — Polish

**Goal:** Visual refinements and quality of life.

1. **Minimap** (optional): small overview in corner showing the full document with a viewport indicator rectangle.
2. **Snap to grid**: when dragging nodes (future free-placement mode), snap to the dot grid.
3. **Zoom shortcuts**: Cmd+0 = reset to 100%, Cmd+= zoom in, Cmd+- zoom out, Cmd+1 = zoom to fit.
4. **Smooth animations**: animate pan/zoom transitions with easing (lerp toward target over a few frames).
5. **Cursor changes**: grab cursor while panning, crosshair while marquee selecting.
6. **Pan boundaries**: soft limits so the document can't be panned completely off screen.
7. **Zoom-dependent detail**: hide dot grid when zoomed out past a threshold, show finer grid when zoomed in.

**Deliverable:** Polished canvas interaction that feels native.

---

## Key Technical Details

### Dot Grid Painting

The dot grid is purely visual — painted in a `canvas()` element's paint callback, not part of the element tree.

```
fn paint_dot_grid(bounds, offset, zoom, window) {
    let spacing = DOT_SPACING * zoom;
    let dot_size = DOT_SIZE * zoom;

    // Compute grid start from offset
    let start_x = offset.x % spacing;
    let start_y = offset.y % spacing;

    // Iterate visible positions
    let mut x = bounds.left() + start_x;
    while x < bounds.right() {
        let mut y = bounds.top() + start_y;
        while y < bounds.bottom() {
            window.paint_quad(fill(
                Bounds::centered_at(point(x, y), size(dot_size, dot_size)),
                DOT_COLOR,
            ));
            y += spacing;
        }
        x += spacing;
    }
}
```

At typical zoom levels with 20px spacing and a 900x700 viewport, this paints ~1500 dots per frame — trivial for the GPU.

### Content Positioning

The document content is wrapped in a container positioned relative to the viewport:

```
div()
    .absolute()
    .left(px(viewport_center.x + offset.x))
    .top(px(viewport_center.y + offset.y))
    // zoom applied as a scale multiplier to the content wrapper's size
    .child(rendered_document)
```

For zoom, since mozui doesn't have a CSS `transform: scale()` equivalent on divs, we scale by adjusting the content wrapper's width/height and relying on the document's flexbox layout to reflow. At zoom != 1.0, the document renders at its natural size inside a scaled container.

Alternative: use a custom `Element` implementation that applies scale in `prepaint`/`paint` by manipulating bounds. This gives pixel-perfect zoom but requires more low-level work.

### Interaction Priority

Input events need careful routing:

1. **Space held + left drag** → pan (highest priority, intercept before children)
2. **Middle-click drag** → pan
3. **Cmd + scroll wheel** → zoom
4. **Scroll wheel** → pan
5. **Pinch** → zoom
6. **Left click on node** → select (handled by existing `on_click` on canvas nodes)
7. **Left drag on empty area** → marquee select
8. **Right click on node** → context menu (existing)

The canvas viewport registers its pan/zoom handlers. Node click/drag handlers use `stop_propagation()` so they don't conflict.

### Integration with Builder

The canvas component lives in the `mozui-builder` crate's `builder::canvas` module (or a new `mozui-canvas` crate if we want it reusable). `BuilderView` owns the `CanvasState` and passes it to the render function alongside the document tree and selection handlers.

The existing `render_canvas()` function becomes the inner content renderer. A new outer function wraps it with the viewport, dot grid, and interaction handling.

---

## What Changes in Existing Code

- `BuilderView` gains a `canvas_state: CanvasState` field.
- `BuilderView.selected` changes from `Option<NodeId>` to `HashSet<NodeId>` (Phase 3).
- `render_canvas()` is renamed to `render_document_content()` — it still renders the node tree.
- A new `render_canvas_viewport()` wraps it with the dot grid, pan/zoom transform, and marquee overlay.
- Property panel, tree view, and context menu updated for multi-select.
- Keyboard shortcuts: Cmd+0, Cmd+=, Cmd+- for zoom control.
