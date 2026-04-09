# mozui Layout Engine Overhaul

Replace mozui's eager layout system with a GPUI-style three-phase architecture that supports lazy text measurement, constraint-driven sizing, deferred floating elements, and layout caching.

---

## Current Architecture (what we're replacing)

```
build element tree → layout (eager measure + taffy nodes) → compute_layout → collect_layouts → paint
```

**Problems:**
1. **Text is measured eagerly** — `measure_text()` is called during `layout()` with no width constraint, producing a fixed-size leaf. Text cannot wrap to fill available width.
2. **No constraint negotiation** — elements can't respond to available space. A label inside a flex container doesn't know its max width until taffy solves, but it needs to know its width to compute its height (text wrapping).
3. **Floating elements are hacked** — tooltips, hover cards, popovers manually reposition via `push_offset()` after layout. They can't reference other elements' final positions during their own layout.
4. **No layout caching** — the entire taffy tree is rebuilt and solved every frame, even if only a leaf changed.
5. **VirtualList needs explicit viewport size** — can't derive it from layout constraints.

---

## Target Architecture

Three-phase rendering with lazy measurement, deferred elements, and layout caching:

```
Phase 1: request_layout   — build taffy tree, register measure functions (no solving yet)
Phase 2: prepaint         — taffy solves, deferred elements get their constraints, second layout pass
Phase 3: paint            — emit draw commands using final positions
```

### Phase 1: `request_layout`

Every element creates its taffy node(s) and returns a `LayoutId`. Leaf elements that need constraint-aware sizing (text, images, virtual lists) register a **measure context** instead of pre-computing their size.

```rust
trait Element {
    /// Build taffy nodes. Returns the root LayoutId for this element.
    /// Must NOT read layout results — taffy hasn't solved yet.
    fn request_layout(&mut self, cx: &mut LayoutContext) -> LayoutId;

    /// Called after taffy solves. Elements can read their resolved bounds,
    /// finalize text layout, and register deferred children.
    fn prepaint(&mut self, bounds: Bounds, cx: &mut PaintContext);

    /// Emit draw commands and register interactions.
    fn paint(&mut self, bounds: Bounds, cx: &mut PaintContext);
}
```

Key change: `Element` is now `&mut self`, not `&self`. This lets elements cache their resolved state between phases (e.g., a Text element stores its shaped runs after prepaint for use in paint).

### Phase 2: `prepaint`

After taffy solves the root layout:

1. Walk the tree in pre-order.
2. For each element, call `prepaint(bounds)` with its resolved bounds.
3. Elements that need a second layout pass (deferred/floating) can now create new taffy nodes using their parent's resolved size as input, solve those independently, and store the results.
4. Text elements use their resolved width to compute wrapped line layout and update their height if it changed (triggering a re-solve of the parent — see **incremental relayout** below).

### Phase 3: `paint`

Walk the tree again. Elements emit `DrawCommand`s and register interactions using their final resolved positions. Same as today but using `Bounds` (resolved position + size) instead of indexing into a flat `Vec<ComputedLayout>`.

---

## Data Structures

### `LayoutId`

Opaque handle to a taffy `NodeId`. Elements store these to look up their resolved layout.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutId(NodeId);
```

### `Bounds`

Resolved absolute position and size. Replaces `ComputedLayout`.

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct Bounds {
    pub origin: Point,
    pub size: Size,
}
```

### `MeasureContext`

Stored per-node for leaf elements that need lazy measurement. Taffy calls into this during constraint solving.

```rust
/// Context attached to a taffy leaf via `new_leaf_with_context`.
/// The measure function reads this to produce a size.
pub enum MeasureContext {
    /// Text that needs shaping with a width constraint.
    Text {
        text: String,
        style: TextStyle,
    },
    /// Image with intrinsic aspect ratio.
    Image {
        intrinsic: Size,
        object_fit: ObjectFit,
    },
    /// Custom measure function (for user-defined elements).
    Custom(Box<dyn Fn(Size<Option<f32>>, Size<AvailableSpace>) -> Size>),
}
```

### `LayoutContext`

Passed during `request_layout`. Wraps the taffy tree and provides element-facing API.

```rust
pub struct LayoutContext<'a> {
    engine: &'a mut LayoutEngine,
    font_system: &'a FontSystem,
    /// Deferred element registrations (processed after initial solve).
    deferred: &'a mut Vec<DeferredEntry>,
    /// Layout cache for stable subtrees.
    cache: &'a mut LayoutCache,
}

impl<'a> LayoutContext<'a> {
    /// Create a leaf node with fixed size (no measure function needed).
    pub fn new_leaf(&mut self, style: Style) -> LayoutId;

    /// Create a leaf with a measure context. Taffy will call the global
    /// measure function with this context during solving.
    pub fn new_measured_leaf(&mut self, style: Style, measure: MeasureContext) -> LayoutId;

    /// Create a node with children.
    pub fn new_with_children(&mut self, style: Style, children: &[LayoutId]) -> LayoutId;

    /// Register a deferred element. It will be laid out in Phase 2
    /// after its anchor's position is resolved.
    pub fn defer(&mut self, element: Box<dyn Element>, anchor: LayoutId, placement: Placement);

    /// Get a cached layout for a subtree if the inputs haven't changed.
    pub fn cached(&mut self, key: LayoutCacheKey) -> Option<LayoutId>;

    /// Store a computed layout in the cache.
    pub fn cache(&mut self, key: LayoutCacheKey, layout_id: LayoutId);
}
```

### `PaintContext`

Passed during `prepaint` and `paint`. Provides resolved bounds and draw list access.

```rust
pub struct PaintContext<'a> {
    draw_list: &'a mut DrawList,
    interactions: &'a mut InteractionMap,
    font_system: &'a FontSystem,
    /// All resolved layouts from taffy, keyed by LayoutId.
    layouts: &'a LayoutStore,
    /// Window size (for popover clamping, etc.).
    window_size: Size,
}

impl<'a> PaintContext<'a> {
    /// Look up the resolved bounds for a LayoutId.
    pub fn bounds(&self, id: LayoutId) -> Bounds;

    /// Access the draw list.
    pub fn draw_list(&mut self) -> &mut DrawList;

    /// Access interactions.
    pub fn interactions(&mut self) -> &mut InteractionMap;
}
```

---

## The Global Measure Function

Taffy 0.7's `compute_layout_with_measure` takes a single closure that handles ALL leaf measurements. mozui registers `MeasureContext` on each leaf, then dispatches in the global function:

```rust
fn global_measure(
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
    _node_id: NodeId,
    node_context: Option<&mut MeasureContext>,
    _style: &Style,
) -> Size<f32> {
    let Some(ctx) = node_context else {
        return Size::ZERO;
    };
    match ctx {
        MeasureContext::Text { text, style } => {
            let max_width = match available_space.width {
                AvailableSpace::Definite(w) => Some(w),
                AvailableSpace::MaxContent => None,
                AvailableSpace::MinContent => Some(0.0),
            };
            // Use known_dimensions if taffy already determined one axis
            let width_constraint = known_dimensions.width.or(max_width);
            let measured = measure_text(text, style, width_constraint, &FONT_SYSTEM);
            Size {
                width: known_dimensions.width.unwrap_or(measured.width),
                height: known_dimensions.height.unwrap_or(measured.height),
            }
        }
        MeasureContext::Image { intrinsic, object_fit } => {
            // Preserve aspect ratio within constraints
            let w = known_dimensions.width.unwrap_or(intrinsic.width);
            let h = known_dimensions.height.unwrap_or(intrinsic.height);
            Size { width: w, height: h }
        }
        MeasureContext::Custom(f) => {
            let result = f(known_dimensions, available_space);
            Size { width: result.width, height: result.height }
        }
    }
}
```

**Key insight:** taffy may call the measure function multiple times per node during solving (once for min-content, once for max-content, once for final). The function must be pure — same inputs, same outputs. This is why we pass `&FontSystem` by reference and cache shaped text if needed.

---

## Deferred Elements (Floating Layers)

Tooltips, popovers, menus, hover cards, and sheets need to know their anchor's resolved position before they can lay themselves out. These are **deferred elements**.

### Registration

During `request_layout`, a tooltip/popover calls `cx.defer(...)` instead of creating its content's taffy nodes inline. It creates a placeholder zero-size leaf in the main tree so the index count stays stable.

```rust
// Inside Tooltip::request_layout
fn request_layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
    let anchor_id = self.trigger.request_layout(cx);
    if self.visible {
        cx.defer(self.content.take().unwrap(), anchor_id, self.placement);
    }
    anchor_id // tooltip doesn't add size, it's a wrapper
}
```

### Resolution

After the main layout solve, the engine processes deferred entries:

```rust
for deferred in &mut self.deferred_entries {
    let anchor_bounds = self.bounds(deferred.anchor_id);
    // Create an independent taffy tree for the deferred content
    let mut sub_engine = LayoutEngine::new();
    let sub_root = deferred.element.request_layout(&mut sub_cx);
    sub_engine.compute_layout_with_measure(sub_root, available, global_measure);
    // Position relative to anchor
    let content_size = sub_engine.layout(sub_root).size;
    let position = compute_popover_position(
        anchor_bounds, content_size, deferred.placement, window_size
    );
    deferred.resolved_bounds = Some(Bounds { origin: position, size: content_size });
    deferred.resolved_layouts = sub_engine.collect_layouts(sub_root);
}
```

This eliminates `push_offset` / `pop_offset` hacks. Deferred elements have their own layout tree with correct absolute positions from the start. They paint after the main tree, naturally appearing on top.

---

## Layout Caching

### Cache Key

A subtree's layout depends on:
1. The element's style properties (hashed)
2. The available space constraints from the parent
3. The children's cache keys (recursive)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutCacheKey {
    style_hash: u64,
    children_hash: u64,
}
```

### Cache Entry

```rust
struct CacheEntry {
    key: LayoutCacheKey,
    layout_id: LayoutId,
    /// Generation counter — entries not accessed this frame are evicted.
    last_used: u64,
}
```

### How It Works

1. Elements compute their `LayoutCacheKey` from their style + children.
2. In `request_layout`, they check `cx.cached(key)`. If hit, return the cached `LayoutId` directly — skip creating taffy nodes entirely.
3. If miss, create nodes normally, then call `cx.cache(key, layout_id)`.
4. After each frame, evict entries not used in the last N frames.

**Impact:** For a 1000-item list where only 20 are visible, the 980 off-screen items skip taffy node creation entirely. For static UI (headers, sidebars, labels that didn't change), layout is a hash check.

### Style Hashing

To compute `style_hash` cheaply, style builder methods update an incremental hash:

```rust
impl Div {
    pub fn w(mut self, w: f32) -> Self {
        self.taffy_style.size.width = length(w);
        self.style_hash.write_u32(w.to_bits());
        self
    }
}
```

This avoids hashing the entire `Style` struct every frame.

---

## Text Layout Integration

The biggest win from this overhaul. Currently:

```rust
// OLD: eager, no width constraint
fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> NodeId {
    let size = measure_text(&self.content, &style, None, font_system);
    engine.new_leaf(Style { size: fixed(size), ..default() })
}
```

New approach:

```rust
// NEW: lazy, constraint-aware
fn request_layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
    cx.new_measured_leaf(
        Style {
            // Let taffy determine width from flex constraints
            size: Size { width: Dimension::Auto, height: Dimension::Auto },
            ..default()
        },
        MeasureContext::Text {
            text: self.content.clone(),
            style: self.text_style(),
        },
    )
}
```

Taffy calls the measure function with the available width from flex layout. The text wraps to that width and returns the correct height. The parent container sizes itself around the wrapped text. No manual `max_width` parameter needed.

### Shaped Text Caching

Text shaping is expensive. Cache shaped runs keyed on `(text, style, width_constraint)`:

```rust
struct ShapedTextCache {
    entries: HashMap<ShapedTextKey, ShapedTextEntry>,
    generation: u64,
}

struct ShapedTextKey {
    text_hash: u64,
    style_hash: u64,
    width_px: Option<u32>, // quantized to pixels to avoid float comparison issues
}

struct ShapedTextEntry {
    runs: Vec<ShapedRun>,
    size: Size,
    last_used: u64,
}
```

**Quantization:** Width constraints are rounded to the nearest pixel before cache lookup. This prevents cache thrashing from sub-pixel layout differences across frames.

---

## Incremental Relayout

When prepaint discovers that a measured element's height changed (e.g., text wrapped to more lines than expected), we need a targeted re-solve rather than rebuilding the entire tree.

**Strategy:** Mark dirty nodes and re-solve only affected subtrees.

```rust
impl LayoutEngine {
    /// Mark a node as needing re-layout. Propagates up to the root.
    pub fn mark_dirty(&mut self, node: LayoutId) {
        self.taffy.mark_dirty(node.0).unwrap();
    }

    /// Re-solve layout for dirty subtrees only.
    pub fn resolve_dirty(&mut self, root: LayoutId, available: Size<AvailableSpace>) {
        // Taffy's compute_layout already skips clean subtrees internally
        // when using its built-in cache. We just need to call it again.
        self.taffy.compute_layout_with_measure(root.0, available, global_measure);
    }
}
```

Taffy's internal cache means that re-solving after marking one node dirty only recomputes the path from that node to the root, plus any siblings affected by the size change.

---

## Migration Path

### Step 1: `LayoutEngine` with `MeasureContext` (non-breaking)

Upgrade `LayoutEngine` to use `TaffyTree<MeasureContext>` and `compute_layout_with_measure`. Add `new_measured_leaf`. Keep the old `Element` trait working — the `layout()` method can still create fixed-size leaves. Text elements can opt-in to measured leaves.

**Files:** `mozui-layout/src/lib.rs`

**Changes:**
- `TaffyTree` → `TaffyTree<MeasureContext>`
- Add `new_measured_leaf(style, context) -> NodeId`
- `compute_layout` → `compute_layout_with_measure` with `global_measure`
- `FontSystem` reference threaded through (or stored in engine)

### Step 2: Three-phase `Element` trait

Replace:
```rust
trait Element {
    fn layout(&self, ...) -> NodeId;
    fn paint(&self, layouts: &[ComputedLayout], index: &mut usize, ...);
}
```

With:
```rust
trait Element {
    fn request_layout(&mut self, cx: &mut LayoutContext) -> LayoutId;
    fn prepaint(&mut self, bounds: Bounds, cx: &mut PaintContext) {}
    fn paint(&mut self, bounds: Bounds, cx: &mut PaintContext);
}
```

This is a **breaking change** to every element. Do it in one pass — the compiler will flag every element that needs updating. The mapping is mechanical:

- `layout()` → `request_layout()` — same taffy node creation, but now `&mut self`
- The flat `layouts[*index]` pattern → `cx.bounds(self.layout_id)` — each element stores its own `LayoutId` from `request_layout`
- `paint()` gets `Bounds` directly instead of indexing

**Files:** Every file in `mozui-elements/src/`

### Step 3: Deferred elements

Add `DeferredEntry`, deferred resolution loop, and `LayoutContext::defer()`. Migrate Tooltip, HoverCard, Popover, Dialog, Sheet, Notification, Menu to use deferred layout instead of `push_offset`.

**Files:** `mozui-layout/src/lib.rs`, `mozui-app/src/app.rs`, overlay elements

### Step 4: Layout caching

Add `LayoutCache`, `LayoutCacheKey`, style hashing. Opt in elements that benefit most: Div, VirtualList, static components.

**Files:** `mozui-layout/src/cache.rs`, `mozui-elements/src/div.rs`

### Step 5: Text wrapping

Update `Text` and `Label` to use `MeasureContext::Text` measured leaves. Remove manual `max_width` threading. Text now wraps automatically to its container's width.

**Files:** `mozui-elements/src/text.rs`, `mozui-elements/src/label.rs`, `mozui-text/src/lib.rs`

---

## Optimisations

### 1. Quantized text measurement cache

Text measure functions are called multiple times per node per solve (min-content, max-content, final). Cache results keyed on quantized width:

```rust
fn measure_text_cached(text: &str, style: &TextStyle, max_width: Option<f32>) -> Size {
    let key = (hash(text, style), max_width.map(|w| (w * 2.0) as u32)); // half-pixel quantization
    if let Some(cached) = TEXT_MEASURE_CACHE.get(&key) {
        return cached;
    }
    let result = measure_text(text, style, max_width, &FONT_SYSTEM);
    TEXT_MEASURE_CACHE.insert(key, result);
    result
}
```

### 2. Skip unchanged subtrees

If a Div's children list, style hash, and signal dependencies haven't changed since last frame, skip `request_layout` entirely and return the cached `LayoutId`. Taffy's internal cache handles the rest.

### 3. Parallel deferred layout

Deferred elements are independent — they each get their own sub-engine. These can be solved in parallel with `rayon`:

```rust
deferred_entries.par_iter_mut().for_each(|entry| {
    let mut sub_engine = LayoutEngine::new();
    // ... layout and solve independently
});
```

### 4. Arena allocation for element trees

Currently elements are `Box<dyn Element>` heap-allocated. Use a typed arena per frame to batch allocations and improve cache locality:

```rust
pub struct ElementArena {
    storage: bumpalo::Bump,
}

impl ElementArena {
    pub fn alloc<T: Element>(&self, element: T) -> &mut dyn Element {
        self.storage.alloc(element)
    }

    pub fn reset(&mut self) {
        self.storage.reset();
    }
}
```

### 5. Coalesce taffy tree clears

Instead of `engine.clear()` + rebuilding all nodes every frame, keep the taffy tree alive and update changed nodes in-place via `set_style` / `set_children`. Unchanged nodes keep their cached layout results.

```rust
impl LayoutEngine {
    pub fn update_style(&mut self, node: LayoutId, style: Style) {
        if self.taffy.style(node.0) != &style {
            self.taffy.set_style(node.0, style).unwrap();
        }
    }
}
```

### 6. Batch draw command allocation

Pre-allocate the draw list to the previous frame's command count to avoid reallocation:

```rust
impl DrawList {
    pub fn clear_preserving_capacity(&mut self) {
        self.commands.clear(); // Vec::clear keeps capacity
    }
}
```

(Already happens implicitly with `Vec::clear`, but worth noting for arena-based draw lists.)

---

## What Changes for Element Authors

Before:
```rust
impl Element for MyComponent {
    fn layout(&self, engine: &mut LayoutEngine, font: &FontSystem) -> NodeId {
        let child = self.child.layout(engine, font);
        engine.new_with_children(self.style.clone(), &[child])
    }

    fn paint(&self, layouts: &[ComputedLayout], index: &mut usize, draw_list: &mut DrawList, interactions: &mut InteractionMap, font: &FontSystem) {
        let layout = layouts[*index];
        *index += 1;
        let bounds = Rect::new(layout.x, layout.y, layout.width, layout.height);
        // draw...
        self.child.paint(layouts, index, draw_list, interactions, font);
    }
}
```

After:
```rust
impl Element for MyComponent {
    fn request_layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let child_id = self.child.request_layout(cx);
        self.layout_id = cx.new_with_children(self.style.clone(), &[child_id]);
        self.layout_id
    }

    fn paint(&mut self, bounds: Bounds, cx: &mut PaintContext) {
        // bounds is already resolved — no index tracking
        cx.draw_list().push(DrawCommand::Rect { bounds: bounds.into(), ... });
        let child_bounds = cx.bounds(self.child_layout_id);
        self.child.paint(child_bounds, cx);
    }
}
```

The `layouts[*index]` / `*index += 1` pattern is eliminated. Elements store their `LayoutId` and look up bounds directly. No more fragile index tracking that breaks when node counts change.
