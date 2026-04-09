# mozui Devtools Implementation Plan

Three developer tools — Performance Overlay, Signal Debugger, Element Inspector — built incrementally across six phases. Earlier phases focus on backend infrastructure and instrumentation; later phases build the visual UI using mozui elements (dogfooding).

All devtools infrastructure lives in a new `mozui-devtools` crate. The devtools UI renders via `DeferredPosition::Overlay` so it always paints on top of the app.

---

## Phase 1: Devtools Crate + Frame Timing Infrastructure

**Goal:** Create the `mozui-devtools` crate and add timing instrumentation to the render loop. No UI yet — just data collection.

**New crate:** `crates/mozui-devtools/`

```
mozui-devtools/
  Cargo.toml          # depends on mozui-style (for Rect/Size/Color types)
  src/
    lib.rs            # re-exports, DevtoolsState
    timing.rs         # FrameTimings, ring buffer
```

### 1.1 — `FrameTimings`

Ring buffer that stores per-frame timing data. Capacity ~240 frames (4 seconds at 60fps).

```rust
pub struct FrameTiming {
    pub layout_us: u64,     // microseconds spent in layout phase
    pub paint_us: u64,      // microseconds spent in paint phase
    pub render_us: u64,     // microseconds spent in GPU render
    pub total_us: u64,      // full frame time
    pub draw_call_count: u32,
    pub element_count: u32, // interaction entry count as proxy
    pub node_count: u32,    // taffy nodes allocated
}

pub struct FrameTimings {
    ring: VecDeque<FrameTiming>,
    capacity: usize,
}
```

Methods: `push(FrameTiming)`, `iter() -> impl Iterator`, `latest() -> Option<&FrameTiming>`, `avg_fps() -> f32`.

### 1.2 — `DevtoolsState`

Central state that lives on `WindowState`. Tracks which tools are active and holds collected data.

```rust
pub struct DevtoolsState {
    pub perf_overlay_active: bool,
    pub signal_debugger_active: bool,
    pub inspector_active: bool,
    pub timings: FrameTimings,
    pub signal_log: SignalLog,       // Phase 3
    pub inspector_state: InspectorState, // Phase 4
}
```

### 1.3 — Instrument the render loop

**File:** `mozui-app/src/app.rs`

Add `DevtoolsState` to `WindowState`. Wrap each phase in `Instant::now()` / `elapsed()`:

```rust
fn rebuild_interactions(&mut self) {
    let t0 = Instant::now();

    // --- layout phase ---
    let root_id = tree.layout(&mut lcx);
    self.layout_engine.compute_layout(...);
    let layout_us = t0.elapsed().as_micros() as u64;

    let t1 = Instant::now();

    // --- paint phase ---
    tree.prepaint(...);
    tree.paint(...);
    paint_deferred(...);
    let paint_us = t1.elapsed().as_micros() as u64;

    // Collect stats
    let draw_call_count = self.draw_list.len() as u32;
    let element_count = self.interactions.entry_count() as u32;
    let node_count = self.layout_engine.node_count() as u32;

    // render_us filled in after renderer.render() in render()
}
```

Add `DrawList::len()` method (returns `self.commands.len()`).
Add `InteractionMap::entry_count()` method (returns `self.entries.len()`).

**Files touched:**
- `crates/mozui-devtools/src/lib.rs` (new)
- `crates/mozui-devtools/src/timing.rs` (new)
- `crates/mozui-app/src/app.rs` (instrument render loop, add DevtoolsState to WindowState)
- `crates/mozui-app/Cargo.toml` (add mozui-devtools dependency)
- `crates/mozui-elements/src/lib.rs` (add `InteractionMap::entry_count()`)
- `crates/mozui-renderer/src/draw.rs` (add `DrawList::len()`)

---

## Phase 2: Element Introspection Infrastructure

**Goal:** Add runtime type information and debug data to the `Element` trait so the inspector can identify and describe elements. No UI yet.

### 2.1 — `ElementInfo` struct

```rust
/// Debug metadata for an element, collected during layout.
pub struct ElementInfo {
    pub type_name: &'static str,  // "Div", "Label", "Button", etc.
    pub layout_id: LayoutId,
    pub children: Vec<LayoutId>,
    pub properties: Vec<(&'static str, String)>, // key-value debug props
}
```

### 2.2 — Extend `Element` trait

Add a default-implemented method so existing elements continue to compile:

```rust
pub trait Element {
    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId;
    fn prepaint(&mut self, _bounds: Rect, _cx: &mut PaintContext) {}
    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext);

    /// Debug information for devtools. Returns None by default.
    fn debug_info(&self) -> Option<ElementInfo> { None }
}
```

### 2.3 — Implement `debug_info` on core elements

Prioritise the most common elements first:

- **Div:** type_name `"Div"`, properties: `flex_direction`, `gap`, `padding`, `bg`, `border`, `overflow`, child count
- **Label:** type_name `"Label"`, properties: `text` (truncated), `font_size`, `color`, `weight`, `single_line`
- **Text:** type_name `"Text"`, properties: `content` (truncated), `font_size`, `color`
- **Button:** type_name `"Button"`, properties: `label`, `variant`, `disabled`
- **Icon:** type_name `"Icon"`, properties: `name`, `size`, `color`

Other elements: return basic `type_name` + layout_id, add properties incrementally.

### 2.4 — Element tree collection

Add an `element_tree` collector to `LayoutContext` that builds a parallel debug tree during layout. Gated behind `devtools.inspector_active` so there's zero cost when devtools is off.

```rust
pub struct ElementTreeCollector {
    entries: Vec<ElementTreeEntry>,
}

pub struct ElementTreeEntry {
    pub info: ElementInfo,
    pub bounds: Rect,       // filled in during paint phase
    pub depth: u32,
}
```

During layout, each element that implements `debug_info()` pushes an entry. During paint, bounds are resolved and attached.

**Files touched:**
- `crates/mozui-devtools/src/inspector.rs` (new — ElementInfo, ElementTreeCollector, InspectorState)
- `crates/mozui-elements/src/lib.rs` (add `debug_info()` to Element trait, wire collector into LayoutContext)
- `crates/mozui-elements/src/div.rs` (implement debug_info)
- `crates/mozui-elements/src/label.rs` (implement debug_info)
- `crates/mozui-elements/src/text.rs` (implement debug_info)
- `crates/mozui-elements/src/button.rs` (implement debug_info)
- `crates/mozui-elements/src/icon.rs` (implement debug_info)
- Remaining elements: minimal `type_name`-only implementations

---

## Phase 3: Signal Instrumentation

**Goal:** Add logging and introspection to `SignalStore` so the signal debugger can show values and mutation history. No UI yet.

### 3.1 — `SignalLog`

```rust
pub struct SignalMutation {
    pub slot_id: usize,
    pub type_name: &'static str,   // std::any::type_name::<T>()
    pub old_debug: String,          // Debug repr of old value
    pub new_debug: String,          // Debug repr of new value
    pub frame: u64,
    pub timestamp: Instant,
}

pub struct SignalLog {
    mutations: VecDeque<SignalMutation>,
    capacity: usize,                     // keep last ~500 mutations
    current_frame: u64,
}
```

Methods: `push(SignalMutation)`, `mutations_this_frame(frame) -> impl Iterator`, `iter() -> impl Iterator`, `clear()`.

### 3.2 — Instrument `SignalStore`

Gate behind a runtime flag so there's no overhead when devtools is off.

**File:** `crates/mozui-reactive/src/signal.rs`

Add an optional `log: Option<Rc<RefCell<SignalLog>>>` to `SignalStore`. In `set()` and `update()`, if the log is present:

```rust
pub fn set<T: Any + Debug>(&mut self, signal: SetSignal<T>, value: T) {
    if let Some(ref log) = self.log {
        let old = format!("{:?}", self.get::<T>(Signal { id: signal.id, .. }));
        let new = format!("{:?}", &value);
        log.borrow_mut().push(SignalMutation {
            slot_id: signal.id,
            type_name: std::any::type_name::<T>(),
            old_debug: old,
            new_debug: new,
            frame: log.borrow().current_frame,
            timestamp: Instant::now(),
        });
    }
    // existing set logic...
}
```

**Note:** This requires adding a `Debug` bound to the `set`/`update` methods. This is a breaking change — assess the impact on existing code. If too disruptive, use `format!("{:?}", &value as &dyn Any)` as a fallback (less useful but non-breaking), or gate the `Debug` bound behind a cargo feature.

### 3.3 — Signal snapshot

Add a method to `SignalStore` that returns a snapshot of all current signal values (for display in the debugger):

```rust
pub struct SignalSnapshot {
    pub slot_id: usize,
    pub type_name: &'static str,
    pub value_debug: String,
    pub dirty: bool,
}
```

This requires tracking per-slot type names. Add a `type_names: Vec<&'static str>` to `SignalStore`, populated during `get_or_create`.

**Files touched:**
- `crates/mozui-devtools/src/signals.rs` (new — SignalLog, SignalMutation, SignalSnapshot)
- `crates/mozui-reactive/src/signal.rs` (add log, type_names, snapshot method)
- `crates/mozui-reactive/Cargo.toml` (add mozui-devtools dependency, or keep it decoupled via callback)
- `crates/mozui-app/src/context.rs` (wire log into SignalStore when devtools active)

---

## Phase 4: Performance Overlay UI

**Goal:** First visual devtools component. Render a frame-time graph and stats panel as a deferred overlay. Toggle with `Cmd+Shift+O`.

### 4.1 — Keybinding registration

In `App::run()`, register devtools keybindings on the `KeybindingRegistry`:

```rust
keybindings.bind("cmd-shift-o", TogglePerfOverlay);
keybindings.bind("cmd-shift-i", ToggleInspector);
keybindings.bind("cmd-shift-s", ToggleSignalDebugger);
```

Action handler toggles the corresponding `devtools_state` flag and triggers re-render.

### 4.2 — Performance overlay element

**File:** `crates/mozui-devtools/src/perf_overlay.rs`

A mozui element (using `div`, `label`, `text` from mozui-elements) that renders:

- **FPS counter:** large text showing current FPS (from `avg_fps()`)
- **Frame time bar chart:** last ~120 frames as vertical bars, colored by severity (green < 16ms, yellow < 33ms, red > 33ms)
- **Stats panel:** layout time, paint time, render time, draw call count, element count, node count
- **Semi-transparent dark background** with rounded corners

Positioned at top-right via flex alignment. Rendered as a `DeferredPosition::Overlay`.

### 4.3 — Wire into render loop

After `rebuild_interactions()`, if `devtools_state.perf_overlay_active`, insert the perf overlay element into the deferred list. The overlay reads from `DevtoolsState.timings`.

**Files touched:**
- `crates/mozui-devtools/src/perf_overlay.rs` (new — PerfOverlay element)
- `crates/mozui-devtools/Cargo.toml` (add mozui-elements dependency)
- `crates/mozui-app/src/app.rs` (register keybindings, inject overlay into deferred list)

---

## Phase 5: Signal Debugger UI

**Goal:** Visual signal debugger panel. Shows all signals, highlights mutations, scrollable mutation log. Toggle with `Cmd+Shift+S`.

### 5.1 — Signal debugger panel element

**File:** `crates/mozui-devtools/src/signal_panel.rs`

A side panel (similar to Sheet, slides in from the left) containing:

- **Signal table:** columns for slot ID, type, current value, dirty flag
  - Rows highlighted yellow if the signal mutated this frame
  - Scrollable if many signals
- **Mutation log:** chronological list of recent mutations
  - Shows: frame number, slot ID, old → new value
  - Color-coded by recency (bright = recent, faded = old)
  - Scrollable with the last mutation at the top

### 5.2 — Wire into render loop

Similar to perf overlay: if `devtools_state.signal_debugger_active`, inject the signal panel as a deferred overlay. Pass a reference to the `SignalLog` and call `SignalStore::snapshot()` for current values.

**Files touched:**
- `crates/mozui-devtools/src/signal_panel.rs` (new)
- `crates/mozui-app/src/app.rs` (inject panel, pass signal log/snapshot)

---

## Phase 6: Element Inspector UI

**Goal:** Visual element inspector. Hover to highlight elements, click to select, side panel shows element details. Toggle with `Cmd+Shift+I`.

### 6.1 — Hover highlight overlay

When inspector is active, intercept mouse position and find the element under the cursor using the `ElementTreeCollector` data. Draw a highlight rectangle (semi-transparent blue border + fill) over the hovered element's bounds.

Display a floating tooltip near the cursor showing: element type name, dimensions (w x h), position (x, y).

### 6.2 — Selection and detail panel

Clicking an element selects it. A side panel (slides in from the right) shows:

- **Element type** (bold heading)
- **Layout section:** x, y, width, height, layout_id
- **Properties section:** key-value pairs from `debug_info().properties`
- **Tree breadcrumb:** parent chain from root to selected element (clickable to navigate up)

### 6.3 — Element tree view

Below the detail panel (or as a toggleable tab), a collapsible tree view showing the full element hierarchy. Each node shows type name and dimensions. Selected element is highlighted. Expanding a node shows its children.

Uses the existing `TreeView` / `tree_node` elements from mozui-elements.

### 6.4 — Bounds visualization mode

Optional toggle: draw all element bounds as faint outlines across the entire app (CSS-style "show all boxes"). Useful for debugging layout issues without hovering individual elements.

**Files touched:**
- `crates/mozui-devtools/src/inspector_overlay.rs` (new — highlight overlay, tooltip)
- `crates/mozui-devtools/src/inspector_panel.rs` (new — detail panel, tree view)
- `crates/mozui-app/src/app.rs` (inject inspector overlays, intercept mouse for hover detection)

---

## Dependency Graph

```
Phase 1 ──→ Phase 4 (perf overlay needs timing data)
Phase 2 ──→ Phase 6 (inspector needs element introspection)
Phase 3 ──→ Phase 5 (signal panel needs signal log)
Phase 1 ──→ Phase 2 (devtools crate must exist)
Phase 1 ──→ Phase 3 (devtools crate must exist)
```

Phases 2 and 3 can run in parallel after Phase 1.
Phases 4, 5, and 6 can run in parallel after their respective prerequisites.

```
         ┌── Phase 2 ──→ Phase 6
Phase 1 ─┼── Phase 3 ──→ Phase 5
         └── Phase 4
```

---

## Cargo Feature Gating

All devtools code is behind a `devtools` cargo feature:

```toml
# mozui/Cargo.toml
[features]
default = []
devtools = ["mozui-app/devtools"]

# mozui-app/Cargo.toml
[features]
devtools = ["mozui-devtools"]

[dependencies]
mozui-devtools = { path = "../mozui-devtools", optional = true }
```

When `devtools` is disabled:
- `DevtoolsState` is a zero-size struct
- Timing instrumentation compiles to no-ops
- `debug_info()` returns `None` (default impl)
- Signal logging is disabled
- Keybindings are not registered
- No overlay elements are injected

This ensures zero runtime cost in release builds.
