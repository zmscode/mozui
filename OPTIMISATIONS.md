# OPTIMISATIONS

Audit of the vendored GPUI + gpui-component codebase. This document identifies concrete
optimisation opportunities, proposes a streamlined project structure, and provides detailed
implementation steps for each migration phase.

The vendor tree is ~190k lines of Rust across 28 crates. Most of the complexity lives in
three places: `gpui` (61k), `gpui-component-ui` (68k), and the platform backends (35k combined).

---

## Table of Contents

1. [Rendering Pipeline](#1-rendering-pipeline)
2. [GPU Backend](#2-gpu-backend)
3. [Layout Engine](#3-layout-engine)
4. [Element System](#4-element-system)
5. [Entity & Reactive System](#5-entity--reactive-system)
6. [Text Rendering](#6-text-rendering)
7. [Platform Layer](#7-platform-layer)
8. [Style System](#8-style-system)
9. [Component Library](#9-component-library)
10. [Proposed Project Structure](#10-proposed-project-structure)

---

## 1. Rendering Pipeline

**Files:** `gpui/src/scene.rs` (896 lines), `gpui/src/window.rs` (5753 lines)

### Current Design

GPUI uses a 3-phase retained/immediate hybrid:

1. **Accumulation** -- elements paint primitives into a `Scene` (quads, shadows, paths, sprites, etc.)
2. **Sorting** -- all primitive vectors sorted by draw order
3. **Batching** -- `BatchIterator` groups consecutive same-type primitives for GPU submission

The `Scene` struct holds 8 separate `Vec`s for each primitive type plus a `paint_operations` log.

### Problem: Every Primitive Is Cloned Twice

In `scene.rs:88-125`, every primitive inserted into the scene is `.clone()`d -- once into the
type-specific vector (for sorting/batching) and once into `paint_operations` (for replay).

```rust
// scene.rs:91-95
Primitive::Quad(quad) => {
    quad.order = order;
    self.quads.push(quad.clone());   // clone 1
}
// ...
self.paint_operations.push(PaintOperation::Primitive(primitive));  // clone 2 (moved)
```

A `Quad` is ~96 bytes, a `Path` is ~200+ bytes. For a 4000-primitive scene (typical complex UI),
this is ~600KB of redundant copies per frame at 60fps.

### Optimisation: Index-Based Scene

Store each primitive once in a flat buffer. Use typed index enums in both the paint operation
log and the per-type sorted vectors.

```rust
/// Index into the appropriate typed vector.
enum PaintOpIndex {
    Shadow(u32),
    Quad(u32),
    Path(u32),
    Underline(u32),
    MonochromeSprite(u32),
    SubpixelSprite(u32),
    PolychromeSprite(u32),
    Surface(u32),
}

struct Scene {
    paint_operations: Vec<PaintOp>,       // PaintOp::Prim(PaintOpIndex)
    shadows: Vec<Shadow>,                 // single source of truth
    quads: Vec<Quad>,
    paths: Vec<Path<ScaledPixels>>,
    underlines: Vec<Underline>,
    monochrome_sprites: Vec<MonochromeSprite>,
    subpixel_sprites: Vec<SubpixelSprite>,
    polychrome_sprites: Vec<PolychromeSprite>,
    surfaces: Vec<PaintSurface>,
    // sorted_* vectors hold indices, not cloned values
    sorted_quads: Vec<u32>,
    sorted_shadows: Vec<u32>,
    // ...
}
```

**Implementation steps:**

1. Replace `insert_primitive` to push directly into the typed vector, storing the index
   in `paint_operations` via `PaintOpIndex` instead of cloning the whole primitive.
2. Change sorting to sort index vectors by `|a, b| vec[*a].order.cmp(&vec[*b].order)`.
3. Update `BatchIterator` to dereference indices when reading primitives for GPU upload.
4. Update `replay()` (scene.rs:127-140) to copy by index rather than cloning primitives.

**Impact:** eliminates ~600KB/frame of cloning. Estimated 15-30% reduction in scene construction time.

### Problem: Root Element Cloned Every Frame

In `window.rs:2385`, `self.root.clone()` clones the entire root element tree each frame.
For complex UIs this can be 50KB+.

### Optimisation

Use `Rc<T>` or pass by reference instead of cloning.

---

## 2. GPU Backend

**Files:** `gpui_wgpu/src/wgpu_renderer.rs` (1700 lines), `gpui_macos/src/metal_renderer.rs` (1709 lines)

### Problem: New Bind Group Per Draw Call (wgpu)

In `wgpu_renderer.rs:1444-1453`, a new wgpu bind group is created for every single draw call:

```rust
let bind_group = resources.device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: &resources.bind_group_layouts.instances,
    entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: self.instance_binding(offset, size),
    }],
    // ...
});
```

This is a GPU-side allocation + state change per draw. A scene with 100+ draw calls means
100+ bind group creations per frame.

### Optimisation: Bind Group Pool

Pre-allocate a pool of bind groups keyed by `(offset, size, texture_id)`. Reuse across frames
with LRU eviction. Most frames have a similar draw pattern, so pool hit rates should be 80%+.

```rust
struct BindGroupPool {
    /// Key: (buffer_offset, buffer_size, optional texture_id)
    cache: HashMap<(u64, u64, Option<u64>), wgpu::BindGroup>,
    /// LRU tracking for eviction
    lru: VecDeque<(u64, u64, Option<u64>)>,
    max_size: usize,
}

impl BindGroupPool {
    fn get_or_create(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        offset: u64,
        size: u64,
        texture: Option<(&wgpu::TextureView, u64)>,
        instance_buffer: &wgpu::Buffer,
    ) -> &wgpu::BindGroup {
        let key = (offset, size, texture.map(|(_, id)| id));
        if !self.cache.contains_key(&key) {
            // Create and insert. Evict oldest if over capacity.
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
                entries: &[/* ... */],
            });
            if self.cache.len() >= self.max_size {
                if let Some(old_key) = self.lru.pop_front() {
                    self.cache.remove(&old_key);
                }
            }
            self.cache.insert(key, bg);
            self.lru.push_back(key);
        }
        &self.cache[&key]
    }

    /// Call at frame start. Bind groups referencing stale buffers must be invalidated.
    fn invalidate_all(&mut self) {
        self.cache.clear();
        self.lru.clear();
    }
}
```

**Implementation steps:**

1. Add `BindGroupPool` as a field on the wgpu renderer struct.
2. Replace the inline `create_bind_group` calls in `draw_instances` (line 1444) and
   `draw_instances_with_texture` (line 1477) with `pool.get_or_create()`.
3. Call `pool.invalidate_all()` when the instance buffer is reallocated (size change).
4. For texture-bearing bind groups, include the texture's atlas ID in the cache key.

**Impact:** 2-5x reduction in bind group creation overhead.

### Problem: Redundant Pipeline State Changes

In both `wgpu_renderer.rs:1454-1457` and `metal_renderer.rs:1024-1400`, every draw function
unconditionally sets the pipeline, vertex buffers, and fragment buffers -- even when the same
pipeline is already bound.

```rust
pass.set_pipeline(pipeline);
pass.set_bind_group(0, &resources.globals_bind_group, &[]);
```

### Optimisation: State Tracking

Track the currently-bound pipeline and skip redundant state changes.

```rust
struct RenderPassState<'a> {
    current_pipeline: Option<*const wgpu::RenderPipeline>,
    current_globals_bound: bool,
}

impl<'a> RenderPassState<'a> {
    fn set_pipeline(
        &mut self,
        pass: &mut wgpu::RenderPass<'a>,
        pipeline: &'a wgpu::RenderPipeline,
    ) {
        let ptr = pipeline as *const _;
        if self.current_pipeline != Some(ptr) {
            pass.set_pipeline(pipeline);
            self.current_pipeline = Some(ptr);
            // Globals bind group must be rebound after pipeline change
            self.current_globals_bound = false;
        }
    }

    fn ensure_globals(
        &mut self,
        pass: &mut wgpu::RenderPass<'a>,
        globals: &'a wgpu::BindGroup,
    ) {
        if !self.current_globals_bound {
            pass.set_bind_group(0, globals, &[]);
            self.current_globals_bound = true;
        }
    }
}
```

**Implementation steps:**

1. Create `RenderPassState` at the start of each frame's render pass.
2. Thread it through `draw_instances`, `draw_instances_with_texture`, and all draw helpers.
3. Replace direct `pass.set_pipeline()` / `pass.set_bind_group(0, ...)` with state-tracked calls.

**Impact:** 5-10% reduction in GPU command buffer size. Eliminates 50-70% of redundant state changes.

### Problem: Global Buffer Written in 3 Separate Calls

`wgpu_renderer.rs:1162-1179` issues three separate `queue.write_buffer()` calls for globals,
path globals, and gamma. These could be a single write of a contiguous struct.

### Problem: Arc Clone Per Texture View Lookup

`wgpu_atlas.rs:61-67` clones the `wgpu::TextureView` Arc for every sprite draw.
500+ sprites = 500+ Arc refcount bumps per frame.

---

## 3. Layout Engine

**Files:** `gpui/src/taffy.rs` (630 lines)

### Current Design

Pure delegation to `taffy::TaffyTree<NodeContext>`. Each measured node carries a
`StackSafe<Box<dyn FnMut>>` closure for custom sizing.

### Problem: Boxed Closure Per Measured Node

```rust
type NodeMeasureFn = StackSafe<Box<dyn FnMut(
    Size<Option<Pixels>>, Size<AvailableSpace>, &mut Window, &mut App
) -> Size<Pixels>>>;
```

Every node with a custom measure function (text nodes, images, etc.) heap-allocates a
boxed closure wrapped in `StackSafe`. This is two levels of indirection per layout computation.

### Optimisation

Use a generic measure callback via a trait object stored once on the layout engine,
dispatched by node type -- instead of per-node closures. Or use a `SmallBox` / inline closure
for the common case where the closure fits in 2 pointers.

### Good Pattern: Stack-Safe Recursion

The `stacksafe` macro converts recursive layout into iterative stack-safe computation.
This is a solid choice for deeply nested UIs.

---

## 4. Element System

**Files:** `gpui/src/element.rs` (797 lines), `gpui/src/elements/` directory

### Current Design

3-phase lifecycle: `request_layout` -> `prepaint` -> `paint`.
All elements are recreated every frame (no diffing, no memoization).
`AnyElement` wraps elements in `ArenaBox<dyn ElementObject>` for type erasure.

### Problem: No Memoization

Every frame rebuilds the entire element tree from scratch. This is by design (immediate-mode
style) but means complex render functions run fully even when nothing changed.

### Optimisation: Optional Memoization

Add a `Memo<V>` wrapper element that caches its output based on a key:

```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    Memo::new(self.data_version, || {
        // expensive element construction
    })
}
```

This preserves the immediate-mode ergonomics while skipping unchanged subtrees.

### Problem: Double Indirection in AnyElement

`ArenaBox<dyn ElementObject>` wraps `Drawable<dyn Element>`. Two levels of dynamic dispatch:
trait object + virtual calls through `ElementObject`.

### Problem: Arc in GlobalElementId

`Arc<[ElementId]>` is used for element path tracking. Every element access bumps the
refcount. A `SmallVec<[ElementId; 8]>` with `Rc` for the rare deep case would avoid atomic
operations on the hot path.

---

## 5. Entity & Reactive System

**Files:** `gpui/src/app/entity_map.rs` (1279 lines), `gpui/src/subscription.rs`

### Problem: Dual Rc Per Subscription

Each subscriber allocates two separate `Rc<Cell<bool>>` -- one for `active` and one for `dropped`:

```rust
struct Subscriber<Callback> {
    active: Rc<Cell<bool>>,    // heap alloc 1
    dropped: Rc<Cell<bool>>,   // heap alloc 2
    callback: Callback,
}
```

For an app with 500+ observers, that's 1000+ unnecessary heap allocations.

### Optimisation

Combine into a single `Rc<Cell<u8>>` using bitflags:

```rust
struct SubscriberFlags(u8);
impl SubscriberFlags {
    const ACTIVE:  u8 = 0b01;
    const DROPPED: u8 = 0b10;

    fn is_active(&self) -> bool  { self.0 & Self::ACTIVE != 0 }
    fn is_dropped(&self) -> bool { self.0 & Self::DROPPED != 0 }
    fn set_active(&mut self)     { self.0 |= Self::ACTIVE; }
    fn set_dropped(&mut self)    { self.0 |= Self::DROPPED; }
}

struct Subscriber<Callback> {
    flags: Rc<Cell<SubscriberFlags>>,  // single heap alloc
    callback: Callback,
}
```

**Implementation steps:**

1. Replace `active: Rc<Cell<bool>>` + `dropped: Rc<Cell<bool>>` with `flags: Rc<Cell<u8>>`.
2. Update `SubscriberSet::insert()` (subscription.rs:46-67) -- create one `Rc<Cell<u8>>` instead of two.
3. Update the `Subscription` drop handler (uses `dropped.set(true)`) to set the DROPPED bit.
4. Update `SubscriberSet::retain()` and `remove()` to check the DROPPED bit.
5. Update the activation closure returned from `insert()` to set the ACTIVE bit.

**Impact:** 50% reduction in subscription allocations.

### Problem: BTreeMap For Subscriber Storage

`SubscriberSet` uses `BTreeMap<EmitterKey, Option<BTreeMap<usize, Subscriber>>>`.
Subscribers are iterated in insertion order -- a `Vec` with stable indices would be faster
for both insertion and iteration, with O(1) removal via swap-remove + free list.

**Implementation steps:**

1. Replace inner `BTreeMap<usize, Subscriber>` with a `SlotMap<SubscriberKey, Subscriber>`.
2. Alternatively, use a `Vec<Option<Subscriber>>` with a free-list for O(1) insert/remove:
   ```rust
   struct SubscriberVec<Callback> {
       slots: Vec<Option<Subscriber<Callback>>>,
       free: Vec<usize>,
   }
   impl<Callback> SubscriberVec<Callback> {
       fn insert(&mut self, sub: Subscriber<Callback>) -> usize {
           if let Some(idx) = self.free.pop() {
               self.slots[idx] = Some(sub);
               idx
           } else {
               self.slots.push(Some(sub));
               self.slots.len() - 1
           }
       }
       fn remove(&mut self, idx: usize) {
           self.slots[idx] = None;
           self.free.push(idx);
       }
   }
   ```
3. Replace outer `BTreeMap<EmitterKey, ...>` with `HashMap<EmitterKey, SubscriberVec>` for
   O(1) lookup instead of O(log n).

### Problem: 11 Separate SubscriberSets on App

The `App` struct maintains 11 independent `SubscriberSet` instances (observers, event_listeners,
keystroke_observers, etc.). Each is `Rc<RefCell<SubscriberSetState>>`.

### Optimisation: Unified Event Bus

Consolidate into a single type-indexed event dispatcher. Reduces struct size by ~300 bytes
and improves cache locality for event dispatch.

### Problem: Arc\<RwLock\> on Entity Ref Counts

`EntityMap.ref_counts: Arc<RwLock<EntityRefCounts>>`. Every entity handle clone acquires
a read lock. Write contention occurs on entity drops.

### Optimisation

Use per-entity `AtomicU32` ref counts instead of a shared locked map. Eliminates lock
contention entirely.

---

## 6. Text Rendering

**Files:** `gpui/src/text_system.rs` (1187 lines), `gpui/src/text_system/` directory

### Current Design

Platform-delegated text shaping (CoreText on macOS) with glyph atlas caching.
Multiple `RwLock`-protected caches for font IDs, metrics, and raster bounds.
Pool reuse for `LineWrapper` and `FontRun` vectors.

### Good Patterns

- `SmallVec<[DecorationRun; 32]>` avoids allocation for typical text
- `wrapper_pool` and `font_runs_pool` reduce allocation pressure
- Subpixel variant caching (4x4 on macOS, 4x1 on Windows)

### Problem: RwLock on Hot Font Lookup Path

Every font resolution acquires a `RwLock`. For text-heavy UIs rendering 1000+ text runs
per frame, this is 1000+ lock acquisitions.

### Optimisation

Use a lock-free concurrent hash map (e.g. `dashmap`) or `thread_local!` caches with
periodic sync. The font cache is read-heavy with infrequent writes -- ideal for
read-optimised concurrency.

---

## 7. Platform Layer

**Files:** `gpui/src/platform.rs` (2399 lines), `gpui_macos/` (11k lines), `gpui_linux/` (12k lines), `gpui_windows/` (11k lines)

### Problem: Excessive Box\<dyn\> For Callbacks

The platform trait uses `Box<dyn FnMut>` for every callback:

```rust
fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>);
fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>);
// 10+ similar
```

Every callback registration is a heap allocation, even for small closures.

### Optimisation

Use `SmallBox<dyn FnMut, S16>` (inline storage for closures <= 16 bytes) or accept
generic `impl FnMut` with type erasure only at the storage boundary.

### Problem: macOS FFI Overhead

`gpui_macos/src/platform.rs` calls `msg_send![APP_CLASS, sharedApplication]` repeatedly
without caching the result. Each `msg_send!` is a full Objective-C message dispatch.

### Optimisation

Cache `NSApplication` and other frequently-accessed Objective-C objects at init time.

---

## 8. Style System

**Files:** `gpui/src/style.rs` (1200+ lines), `gpui/src/styled.rs`

### Problem: Style Struct Size

The `Style` struct has 32+ fields covering every CSS-like property. Estimated size:
**~500 bytes per element**. For 500 elements, that's 250KB in style data alone.

Field breakdown by category:

| Category | Fields | Est. Bytes | Usage Frequency |
|----------|--------|-----------|-----------------|
| Display/position | `display`, `position`, `overflow` | 12 | Every element |
| Flex layout | `flex_direction`, `flex_wrap`, `flex_grow`, `flex_shrink`, `flex_basis`, `align_*`, `justify_*`, `gap` | 80 | ~70% of elements |
| Size | `size`, `min_size`, `max_size`, `aspect_ratio` | 72 | ~60% of elements |
| Spacing | `margin`, `padding`, `inset` | 128 | ~50% of elements |
| Visual | `background`, `border_widths`, `border_color`, `corner_radii`, `box_shadow`, `opacity` | 120+ | ~30% of elements |
| Text | `text.*` (font, size, color, weight, etc.) | 64 | ~20% of elements |
| Transform | `transform` | 32 | <5% of elements |

Most elements only use display + flex + size + padding. The visual and transform fields
are cold for 70%+ of elements.

### Optimisation: Hot/Cold Split

Split `Style` into a compact hot struct (always present) and optional cold extensions:

```rust
/// Hot path: 64 bytes. Present on every element.
struct StyleCore {
    display: Display,           // 1 byte
    position: Position,         // 1 byte
    overflow: Point<Overflow>,  // 2 bytes
    flex_direction: FlexDirection, // 1 byte
    flex_wrap: FlexWrap,        // 1 byte
    align_items: Option<AlignItems>,   // 2 bytes
    justify_content: Option<JustifyContent>, // 2 bytes
    flex_grow: f32,             // 4 bytes
    flex_shrink: f32,           // 4 bytes
    size: Size<Length>,         // 16 bytes
    padding: Edges<DefiniteLength>,    // 16 bytes
    margin: Edges<Length>,      // 16 bytes
    // --- 66 bytes, aligns to 72 ---
}

/// Cold path: heap-allocated only when these properties are set.
struct StyleExtensions {
    background: Option<Background>,
    border: Option<Box<BorderStyle>>,
    shadow: Option<Box<Vec<BoxShadow>>>,
    corner_radii: Option<Corners<AbsoluteLength>>,
    transform: Option<TransformStyle>,
    text: Option<Box<TextStyleRefinement>>,
    inset: Option<Edges<Length>>,
    min_size: Option<Size<Length>>,
    max_size: Option<Size<Length>>,
    opacity: Option<f32>,
}

struct Style {
    core: StyleCore,
    ext: Option<Box<StyleExtensions>>,  // None for simple elements
}
```

**Implementation steps:**

1. Create `StyleCore` with the ~8 most-used fields (display, position, flex, size, padding, margin).
2. Move all remaining fields into `StyleExtensions` behind `Option<Box<_>>`.
3. Update `Styled` trait methods: core fields set directly, extension fields lazily allocate
   the `ext` box on first use.
4. Update `Refineable` derive macro to handle the two-level struct.
5. Update `gpui/src/style.rs` type conversions to/from Taffy `taffy::Style`.

**Impact:** 50-70% reduction in per-element style memory. For the common case of a simple
flex container with padding, the style drops from ~500 bytes to ~72 bytes. Better cache
utilisation for layout-heavy frames.

---

## 9. Component Library

**Files:** `gpui-component-ui/src/` (68k lines, 216 files)

### Problem: IDE Features Bundled With UI Components

The component library conflates basic UI components with text editor infrastructure:

| Feature | Lines | Dependencies | Decision |
|---------|-------|-------------|----------|
| Input (basic text) | ~2,000 | core | **Keep** |
| Input (code editor) | ~7,000 | tree-sitter, lsp-types, ropey | **Remove** |
| Dock (tab layout) | ~4,600 | core | **Keep (default)** |
| Highlighter | ~3,200 | tree-sitter, 32 language parsers | **Remove** |
| Table (virtual scroll) | ~3,500 | sum-tree | **Keep (default)** |
| Rich text rendering | ~1,200 | markdown, html5ever | **Keep (default)** |
| Charts | ~3,500 | core | **Keep (default)** |
| Theme (Oklab color math) | ~2,400 | none, but 918 lines of color math | **Keep** |

### What Gets Removed Entirely

The following IDE-specific features are deleted from the codebase (not feature-gated):

- **Syntax highlighting** (~2,757 lines) -- tree-sitter + 32 language parser dependencies.
  Not relevant to a UI framework. Apps needing syntax highlighting can integrate tree-sitter
  themselves.
- **LSP integration** (~1,500 lines) -- autocomplete, hover docs, go-to-definition, code
  actions. These are IDE features, not UI component concerns.
- **Advanced code input** (~7,000 lines) -- code folding, display maps, line numbers,
  whitespace indicators, multi-cursor. The basic text input covers all standard form/search
  use cases.

This eliminates `tree-sitter` (+ 32 language parsers), `lsp-types`, and the display map
infrastructure. Removes ~11,000 lines and several heavy native compile-time dependencies.

### What Stays as Default

Rich text rendering, charts, virtual scrolling, and dock are all compiled by default with
no feature flags required.

### Problem: Builder Boilerplate

234 component structs follow the same pattern with ~3,000 lines of repetitive setter methods:

```rust
pub fn label(mut self, label: impl Into<SharedString>) -> Self {
    self.label = Some(label.into());
    self
}
```

### Optimisation

A derive macro could generate these. Alternatively, lean into GPUI's `Styled` trait
more aggressively instead of per-component styling methods.

### Problem: Theme Color Struct Bloat

`ThemeColor` has 109 individually-named color fields. No semantic grouping.

### Optimisation

Group into nested structs:

```rust
struct ThemeColor {
    base: BaseColors,         // background, foreground, border, ring
    button: ButtonColors,     // primary, secondary, danger + hover/active variants
    input: InputColors,       // background, border, placeholder, selection
    // ...
}
```

Reduces cognitive load and enables per-group overrides.

---

## 10. Proposed Project Structure

Replace the current 28-crate vendor tree with a streamlined structure:

```
crates/mozui-fork/
  src/
    lib.rs              -- public API re-exports
    app.rs              -- application lifecycle (from gpui/src/app/)
    window.rs           -- window management (from gpui/src/window/)
    element.rs          -- element trait + AnyElement
    elements/           -- built-in elements (div, svg, img, etc.)
    style.rs            -- style system (optimised sparse layout)
    styled.rs           -- Styled trait + helpers
    scene.rs            -- scene graph (index-based, no cloning)
    layout.rs           -- taffy integration
    text/               -- text system (from gpui/src/text_system/)
    input/              -- input handling + key dispatch
    executor.rs         -- async executor
    reactive/           -- entity map, subscriptions, signals
    platform/
      mod.rs            -- platform trait
      macos/            -- macOS Metal backend
      linux/            -- Linux wgpu backend
      windows/          -- Windows wgpu backend
      web/              -- WebGPU/WASM backend
    renderer/
      mod.rs            -- renderer trait
      wgpu.rs           -- wgpu renderer (linux/windows/web)
      metal.rs          -- Metal renderer (macOS)
      atlas.rs          -- texture atlas
      shaders/          -- shader source files
    components/
      mod.rs            -- component prelude
      accordion.rs
      alert.rs
      avatar.rs
      badge.rs
      breadcrumb.rs
      button.rs
      checkbox.rs
      clipboard.rs
      collapsible.rs
      color_picker.rs
      date_picker.rs
      description_list.rs
      dialog.rs
      divider.rs
      dock/             -- dockable panel layout (default)
      form.rs
      group_box.rs
      hover_card.rs
      input.rs          -- text input (no LSP/syntax/folding)
      kbd.rs
      label.rs
      link.rs
      list.rs
      menu.rs
      notification.rs
      pagination.rs
      popover.rs
      progress.rs
      radio.rs
      rating.rs
      resizable.rs
      scroll.rs
      select.rs
      sheet.rs
      sidebar.rs
      skeleton.rs
      slider.rs
      spinner.rs
      stepper.rs
      switch.rs
      tab.rs
      table.rs          -- table with virtual scrolling (default)
      tag.rs
      tooltip.rs
      tree.rs
      virtual_list.rs   -- virtual scrolling (default)
    chart/              -- charting (default)
      line_chart.rs
      bar_chart.rs
      pie_chart.rs
      area_chart.rs
      candlestick_chart.rs
      plot/             -- axes, scales, grids
    text/               -- rich text rendering (default)
      text_view.rs
      format/
        markdown.rs
        html.rs
    theme/
      mod.rs            -- ActiveTheme trait + ThemeColor
      registry.rs       -- theme loading
      colors.rs         -- color palette
```

Removed entirely (not feature-gated):
- `highlighter/` -- syntax highlighting via tree-sitter
- `input/lsp/` -- LSP autocomplete, hover, code actions
- `input/display_map/` -- code folding, line numbers, whitespace indicators
- All tree-sitter language parser dependencies

### Crate Consolidation

| Current (28 crates) | Proposed | Rationale |
|---------------------|----------|-----------|
| gpui + gpui_macros + gpui_shared_string + gpui_util | core module | eliminate 3 crate boundaries |
| gpui_platform + gpui_macos + gpui_linux + gpui_windows + gpui_web | platform/ module | cfg-gated, single crate |
| gpui_wgpu | renderer/ module | fold into renderer module |
| collections + sum_tree + util + util_macros + perf | inline or deps | tiny crates not worth maintaining |
| zlog + ztracing + ztracing_macro | use `tracing` directly | 1,800 lines replaced by existing dep |
| http_client + http_client_tls + reqwest_client | remove entirely | not needed for UI framework |
| scheduler + gpui_tokio + executor | executor module | consolidate async runtime |
| refineable + derive_refineable | style module | fold macro into build.rs or proc-macro |
| media | platform/macos/ | macOS-specific, 400 lines |
| gpui-component-ui + macros + assets | components/ module | inline into main crate |

### Estimated Impact

| Metric | Current | Proposed |
|--------|---------|----------|
| Crate count | 28 | 1 |
| Total lines (Rust) | ~190,000 | ~110,000 |
| Compile time (clean, debug) | ~3 min | ~1.5 min (estimated) |
| External dep count | ~600 | ~250 |
| Binary size | baseline | -20% (IDE deps removed) |

The line count stays higher than the previous ~100k estimate because rich text, charts,
virtual scrolling, and dock are now default rather than feature-gated. The external dep
reduction is significant because tree-sitter (32 language parsers), lsp-types, and their
transitive dependencies are fully removed.

### What Gets Deleted

| Crate / Module | Lines | Reason |
|---------------|------:|--------|
| `zlog` + `ztracing` + `ztracing_macro` | 1,820 | Replace with `tracing` directly |
| `http_client` + `http_client_tls` + `reqwest_client` | 1,400 | Zed cloud features, not UI |
| `util` (9,700 lines, ~200 used by gpui) | ~9,500 | Inline the 8 used functions |
| `perf` | 1,039 | Zed CI tooling |
| `collections` | 416 | Inline the type aliases |
| `input/lsp/` | ~1,500 | IDE feature |
| `input/display_map/` | ~3,500 | IDE feature |
| `highlighter/` | 2,757 | IDE feature (tree-sitter) |
| tree-sitter language parsers (32) | external | Eliminates heavy native deps |
| **Total** | **~22,000** | |

---

## Migration Path — Detailed Implementation

Each phase independently compiles and passes tests before proceeding.

---

### Phase 1: Delete IDE Features From Input Component

**Goal:** Remove LSP, syntax highlighting, display maps, and code editor mode from the
input component. The basic text input (plain text + auto-grow modes) remains fully functional.

#### Step 1.1: Delete directories

Delete the following directories entirely:

| Path | Lines | Contents |
|------|------:|---------|
| `gpui-component-ui/src/input/lsp/` | ~800 | `mod.rs`, `completions.rs`, `code_actions.rs`, `definitions.rs`, `document_colors.rs`, `hover.rs` |
| `gpui-component-ui/src/input/display_map/` | ~2,100 | `mod.rs`, `display_map.rs`, `fold_map.rs`, `folding.rs`, `wrap_map.rs`, `text_wrapper.rs` |
| `gpui-component-ui/src/input/popovers/` | ~1,800 | `mod.rs`, `completion_menu.rs`, `code_action_menu.rs`, `context_menu.rs`, `diagnostic_popover.rs`, `hover_popover.rs` |
| `gpui-component-ui/src/highlighter/` | ~2,757 | `mod.rs`, `highlighter.rs`, `diagnostics.rs`, `languages.rs`, `registry.rs`, `wasm_stub.rs`, `languages/` |

#### Step 1.2: Modify `input/mod.rs`

Current file declares modules and re-exports IDE types. After modification:

```rust
pub(super) const MASK_CHAR: char = '•';

mod blink_cursor;
mod change;
mod clear_button;
mod cursor;
// REMOVED: mod display_map;
mod element;
mod indent;
mod input;
// REMOVED: mod lsp;
mod mask_pattern;
mod mode;
mod movement;
mod number_input;
mod otp_input;
// REMOVED: pub(crate) mod popovers;
mod rope_ext;
mod search;
mod selection;
mod state;

pub(crate) use clear_button::*;
pub use cursor::*;
// REMOVED: pub use display_map::{BufferPoint, DisplayMap, DisplayPoint, FoldRange};
pub use indent::TabSize;
pub use input::*;
// REMOVED: pub use lsp::*;
// REMOVED: pub use lsp_types::Position;
pub use mask_pattern::MaskPattern;
pub use number_input::{NumberInput, NumberInputEvent, StepAction};
pub use otp_input::*;
pub use rope_ext::{InputEdit, Point, RopeExt, RopeLines};
pub use ropey::Rope;
pub use state::*;
```

#### Step 1.3: Modify `input/mode.rs`

Remove the `CodeEditor` variant from `InputMode` and all its associated methods.
Remove imports of `SyntaxHighlighter`, `DiagnosticSet`, `DisplayMap`:

```rust
// REMOVE these imports:
// use super::display_map::DisplayMap;
// use crate::highlighter::DiagnosticSet;
// use crate::highlighter::SyntaxHighlighter;

#[derive(Clone)]
pub(crate) enum InputMode {
    PlainText {
        multi_line: bool,
        tab: TabSize,
        rows: usize,
    },
    AutoGrow {
        rows: usize,
        min_rows: usize,
        max_rows: usize,
    },
    // REMOVED: CodeEditor { ... }
}
```

Delete all methods that reference `CodeEditor`, `highlighter`, `diagnostics`, `folding`,
`line_number`, `indent_guides`, or `language`.

#### Step 1.4: Modify `input/state.rs`

This is the largest change (~2,700 lines). The `InputState` struct carries `display_map: DisplayMap`
and `lsp: Lsp` fields that must be removed.

1. Remove the `display_map` field and all ~30 call sites (lines 298, 411, 543, 815, 822, 880,
   1064-1065, 1092-1093, 1521-1527, 1820-1835, 1995-2006, 2081-2082, 2107, 2299-2301,
   2317, 2364-2366, 2391). Replace wrap/fold logic with direct line-based cursor positioning.
2. Remove the `lsp: Lsp` field (line 356) and its initialization (line 442).
3. Remove imports: `HoverDefinition`, `InlineCompletion`, `Lsp`, `Position`, `FoldRange`,
   `display_map::LineLayout`, `popovers::*`.
4. Replace `DisplayMap`-based cursor positioning with simple offset-based positioning
   (the input already has `Rope`-based text storage that supports direct offset math).

#### Step 1.5: Modify `input/element.rs`

Remove all references to `DisplayMap`, `DisplayPoint`, `BufferPoint`, `FoldRange` (15 occurrences).
Remove rendering of line numbers, fold indicators, indent guides, diagnostic squiggles,
and all popover rendering (completion menu, hover popover, code action menu).

#### Step 1.6: Modify `lib.rs` (gpui-component-ui)

Remove `pub mod highlighter;` declaration (line 43).

#### Step 1.7: Strip Cargo.toml dependencies

Remove from `gpui-component-ui/Cargo.toml`:

```toml
# Remove entirely:
lsp-types
tree-sitter       # core dependency
tree-sitter-json  # non-optional parser

# Remove all 32 optional tree-sitter-* parsers (lines 112-145)

# Remove the tree-sitter-languages feature flag (lines 22-55)
```

#### Step 1.8: Public API items that disappear

The following public types are removed. Any downstream code using them will get compile errors:

- `DisplayMap`, `DisplayPoint`, `BufferPoint`, `FoldRange` (from `input/display_map`)
- `Lsp`, `LspClient`, `LspConfig`, `CompletionItem`, `HoverDefinition`, `InlineCompletion` (from `input/lsp`)
- `CompletionMenu`, `HoverPopover`, `CodeActionMenu`, `DiagnosticPopover` (from `input/popovers`)
- `SyntaxHighlighter`, `LanguageRegistry`, `DiagnosticSet` (from `highlighter`)
- `InputMode::CodeEditor` variant
- All `Input` builder methods: `.language()`, `.line_number()`, `.folding()`, `.indent_guides()`,
  `.diagnostics()`, `.lsp_client()`

The `Input` component's `.code_editor()` constructor is removed. Use `.multi_line()` or
`.auto_grow()` instead.

---

### Phase 2: Delete Zed-Specific Crates

**Goal:** Remove crates that exist only for Zed's monorepo and have trivial replacements.

#### Step 2.1: Replace `zlog` + `ztracing` + `ztracing_macro`

These crates are thin wrappers around the `tracing` crate. They are only used by 2 files
in `sum_tree`:

- `sum_tree/src/sum_tree.rs:13` — `use ztracing::instrument;`
- `sum_tree/src/sum_tree.rs:1401` — `zlog::init_test();`
- `sum_tree/src/cursor.rs:4` — `use ztracing::instrument;`

**Replacement:**

1. Change `use ztracing::instrument` → `use tracing::instrument` (2 files).
2. Change `zlog::init_test()` → `let _ = tracing_subscriber::fmt::try_init()` in test code.
3. Remove `zlog`, `ztracing`, `ztracing_macro` from workspace members and dependencies.
4. Delete the three crate directories.

#### Step 2.2: Remove `http_client` + `http_client_tls` + `reqwest_client`

The `http_client` trait is referenced in several gpui files but only actually used for one
thing: remote image loading in `gpui/src/elements/img.rs`. The `HttpClient` trait is also
stored on the `App` struct.

**Replacement:**

1. Remove the `HttpClient` field from `App` and all associated builder methods.
2. In `img.rs`, replace `HttpClient` usage with a direct `reqwest::Client` stored as a
   window-local resource, or use a simple async fetch function:
   ```rust
   async fn fetch_image(url: &str) -> Result<Vec<u8>> {
       let bytes = reqwest::get(url).await?.bytes().await?;
       Ok(bytes.to_vec())
   }
   ```
3. Delete the three crate directories and remove from workspace.

#### Step 2.3: Delete `util` crate

The `util` crate is 9,700 lines but gpui does NOT actually use it (gpui has its own internal
`util.rs` module). It exists for Zed's other crates.

**Action:** Delete the crate directory and remove from workspace. No code changes needed
in gpui.

#### Step 2.4: Delete `perf` crate

The `perf` crate (1,039 lines) is Zed CI benchmarking tooling. Not used by gpui at runtime.

**Action:** Delete the crate directory and remove from workspace.

#### Step 2.5: Inline `collections` crate

The `collections` crate (416 lines) provides type aliases that use `rustc_hash::FxHasher`:

```rust
pub type HashMap<K, V> = FxHashMap<K, V>;
pub type HashSet<T> = FxHashSet<T>;
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;
pub type IndexSet<T> = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;
```

Plus a `VecMap` implementation and re-exports of `std::collections::*`.

**Replacement:**

1. Add `rustc-hash` and `indexmap` as direct dependencies of gpui.
2. Create an internal `collections.rs` module in gpui with the same type aliases.
3. Mechanically replace `use collections::` → `use crate::collections::` across 21 files
   in gpui that import from it:
   - `taffy.rs`, `view.rs`, `app.rs`, `text_system/line_layout.rs`, `text_system/line_wrapper.rs`,
     `action.rs`, `key_dispatch.rs`, `queue.rs`, `inspector.rs`, `app/entity_map.rs` (2 uses),
     `tab_stop.rs`, `text_system.rs`, `style.rs`, `window.rs`, `subscription.rs`,
     `elements/list.rs`, `keymap.rs`, `platform/keyboard.rs`, `elements/div.rs`,
     `platform/test/platform.rs`, `platform/test/window.rs`
4. Copy `VecMap` into the internal module if used, or replace with `SmallVec<[(K, V); N]>`.
5. Delete the `collections` crate directory.

---

### Phase 3: Merge Leaf Crates Into gpui Core

**Goal:** Reduce crate count by folding small utility crates directly into gpui.

#### Step 3.1: `gpui_shared_string` → `gpui/src/shared_string.rs`

Move the `SharedString` type into gpui core. Update all internal imports. This type is
just a thin newtype around `Arc<str>` with `Into`/`From` impls.

#### Step 3.2: `gpui_util` → `gpui/src/util.rs`

The `gpui_util` crate provides helper functions used internally by gpui (e.g., `post_inc`,
`arc_cow`, `test_util`). Move these into an internal `util` module.

#### Step 3.3: `refineable` + `derive_refineable` → `gpui/src/refineable.rs`

The `Refineable` trait and its derive macro are only used by gpui's style system. Move the
trait into gpui and keep the proc-macro as a separate internal crate (proc-macros must be
their own crate per Rust's rules), but make it a private dependency rather than a separate
published crate.

#### Step 3.4: `scheduler` → `gpui/src/executor/scheduler.rs`

The scheduler crate provides task scheduling primitives. Fold into the executor module.

---

### Phase 4: Fold Platform Backends Into `platform/` Module

**Goal:** Replace 5 platform crates with cfg-gated modules in a single crate.

#### Step 4.1: Create module structure

```
gpui/src/platform/
  mod.rs          -- PlatformApi trait, shared types
  macos/          -- contents of gpui_macos/src/ (11k lines)
  linux/          -- contents of gpui_linux/src/ (12k lines)
  windows/        -- contents of gpui_windows/src/ (11k lines)
  web/            -- contents of gpui_web/src/
  test/           -- test platform mock
```

#### Step 4.2: Apply cfg-gates

```rust
// platform/mod.rs
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_family = "wasm")]
pub mod web;
#[cfg(any(test, feature = "test-support"))]
pub mod test;
```

#### Step 4.3: Merge `gpui_wgpu` into `renderer/` module

Move wgpu renderer code into `gpui/src/renderer/wgpu.rs`. This is used by Linux, Windows,
and WebGPU backends. The Metal renderer (`metal_renderer.rs` from `gpui_macos`) goes into
`renderer/metal.rs`.

#### Step 4.4: Merge `media` crate

The `media` crate (~400 lines) provides macOS-specific media playback. Move into
`platform/macos/media.rs`.

---

### Phase 5: Apply Rendering Optimisations

**Goal:** Implement the scene, bind group, and pipeline state optimisations described in
sections 1 and 2.

#### Step 5.1: Index-based Scene (see Section 1)

Modify `gpui/src/scene.rs`. The key change to `insert_primitive`:

```rust
pub fn insert_primitive(&mut self, primitive: impl Into<Primitive>) {
    let mut primitive = primitive.into();
    let clipped_bounds = primitive.bounds().intersect(&primitive.content_mask().bounds);
    if clipped_bounds.is_empty() { return; }

    let order = self.layer_stack.last().copied()
        .unwrap_or_else(|| self.primitive_bounds.insert(clipped_bounds));

    let index = match &mut primitive {
        Primitive::Shadow(shadow) => {
            shadow.order = order;
            let idx = self.shadows.len() as u32;
            self.shadows.push(*shadow);  // move, not clone
            PaintOpIndex::Shadow(idx)
        }
        Primitive::Quad(quad) => {
            quad.order = order;
            let idx = self.quads.len() as u32;
            self.quads.push(*quad);
            PaintOpIndex::Quad(idx)
        }
        // ... same pattern for all 8 types
    };
    self.paint_operations.push(PaintOp::Prim(index));
}
```

Update sorting: sort `Vec<u32>` indices by the `order` field of the referenced primitive.
Update `BatchIterator` to read through indices.

#### Step 5.2: Bind Group Pool (see Section 2)

Add `BindGroupPool` to `WgpuRenderer`. Replace all `create_bind_group` call sites
(at least `draw_instances` line 1444 and `draw_instances_with_texture` line 1477).

#### Step 5.3: Pipeline State Tracking (see Section 2)

Add `RenderPassState` tracker. Thread through all draw methods.

#### Step 5.4: Coalesce Global Buffer Writes

Replace the 3 separate `queue.write_buffer()` calls (lines 1162-1179) with a single
contiguous write:

```rust
#[repr(C)]
struct GlobalsBlock {
    globals: GlobalParams,
    path_globals: GlobalParams,
    gamma: GammaParams,
}
let block = GlobalsBlock { globals, path_globals, gamma_params };
resources.queue.write_buffer(&resources.globals_buffer, 0, bytemuck::bytes_of(&block));
```

---

### Phase 6: Merge gpui-component Into `components/` Module

**Goal:** Fold the cleaned component library (post-Phase 1 IDE removal) into the main crate.

#### Step 6.1: Move source files

Copy `gpui-component-ui/src/` into `gpui/src/components/`, excluding the already-deleted
`highlighter/`, `input/lsp/`, `input/display_map/`, and `input/popovers/` directories.

#### Step 6.2: Update imports

The component code uses `use gpui::{...}` and `use crate::{...}`. Since it's now inside gpui,
change `use gpui::` to `use crate::` throughout. This is a mechanical find-and-replace across
~200 files.

#### Step 6.3: Merge assets

Move `gpui-component-assets/` into `gpui/assets/`. Update the `icon_named!` macro path.

#### Step 6.4: Merge proc-macro

`gpui-component-macros` stays as a separate proc-macro crate (Rust requirement) but becomes
a private dependency named `mozui-macros` or similar.

#### Step 6.5: Feature flags

Charts, dock, virtual scrolling, and rich text all compile unconditionally. No feature flags.
The `inspector` feature flag is preserved for development tooling.

---

### Phase 7: Optimise Entity/Subscription System

**Goal:** Reduce allocation overhead in the reactive system (see Section 5).

#### Step 7.1: Combine subscriber flags

Replace dual `Rc<Cell<bool>>` with single `Rc<Cell<u8>>` in `subscription.rs`.
See Section 5 for the implementation sketch.

#### Step 7.2: Replace BTreeMap with Vec+freelist

Replace `BTreeMap<usize, Subscriber>` with `SubscriberVec` (Vec + freelist).
Replace outer `BTreeMap<EmitterKey, ...>` with `FxHashMap<EmitterKey, SubscriberVec>`.
See Section 5 for the implementation sketch.

#### Step 7.3: Atomic entity ref counts

Replace `Arc<RwLock<EntityRefCounts>>` in `entity_map.rs` with per-entity `AtomicU32`.
This requires changing `EntityMap` storage from a shared locked map to a `SlotMap` where
each slot contains an `AtomicU32` ref count alongside the entity data.

---

### Phase 8: Sparse Style System

**Goal:** Reduce per-element style memory (see Section 8).

#### Step 8.1: Define `StyleCore` and `StyleExtensions`

Split the `Style` struct as described in Section 8. The `StyleCore` covers the 6 most common
property groups (~72 bytes). Everything else goes into `Option<Box<StyleExtensions>>`.

#### Step 8.2: Update `Styled` trait

Modify all `Styled` trait setter methods. Core field setters remain zero-cost. Extension
field setters lazily allocate the `ext` box on first write:

```rust
fn background(mut self, bg: impl Into<Background>) -> Self {
    let style = self.style();
    let ext = style.ext.get_or_insert_with(|| Box::new(StyleExtensions::default()));
    ext.background = Some(bg.into());
    self
}
```

#### Step 8.3: Update Taffy conversion

The `From<Style> for taffy::Style` conversion must read core fields directly and extension
fields through the `Option`. This is a straightforward mechanical change.

#### Step 8.4: Update `Refineable` derive

The derive macro generates `refine(&mut self, other: &StyleRefinement)` methods. Update
the generated code to handle the two-level structure.
