# mozui Implementation Plan

> Detailed, step-by-step plan for building mozui from zero to a production-quality GUI library.

---

## Table of Contents

1. [Phase 1: Foundation](#phase-1-foundation-weeks-1-4)
2. [Phase 2: Layout & Text](#phase-2-layout--text-weeks-5-8)
3. [Phase 3: Reactivity & Interaction](#phase-3-reactivity--interaction-weeks-9-12)
4. [Phase 4: Focus & Actions](#phase-4-focus--actions-weeks-13-16)
5. [Phase 5: Async & Window Chrome](#phase-5-async--window-chrome-weeks-17-20)
6. [Phase 6: Polish & Cross-Platform](#phase-6-polish--cross-platform-weeks-21-28)
7. [Phase 7: Ecosystem & DX](#phase-7-ecosystem--dx-weeks-29)

---

## Phase 1: Foundation (Weeks 1-4) ✅ COMPLETE

**Goal**: Open a borderless window on macOS, initialize a GPU rendering pipeline, and render a single colored rounded rectangle.

**Milestone**: `App::new().run(|cx| div().w(200.0).h(100.0).bg(Color::RED).rounded(8.0))`

**Status**: All tasks complete. Borderless NSWindow with custom platform shell, wgpu/Metal rendering, SDF rounded rectangle shader with anti-aliasing and border support. Verified working on macOS with Retina display.

---

### 1.1 — Cargo Workspace Setup (Week 1, Days 1-2)

Set up the monorepo workspace structure. All crates are created with stub `lib.rs` files so the workspace compiles from day one.

**Tasks:**

- [x] Create workspace `Cargo.toml` at root with `resolver = "2"`
- [x] Create the following crates under `crates/`:
  - `mozui` — facade crate, re-exports all public API
  - `mozui-app` — `App`, `AppContext`, `AppBuilder`, lifecycle
  - `mozui-platform` — `Platform` trait, `PlatformWindow` trait, `PlatformEvent` enum
  - `mozui-renderer` — wgpu initialization, render pipeline, draw commands
  - `mozui-style` — `Color`, `Fill`, `Shadow`, `Style` struct, `Corners`
  - `mozui-elements` — `Element` trait, `Div` struct, builder methods
  - `mozui-events` — `PlatformEvent`, `MouseEvent`, `KeyEvent`, `Modifiers`, `Key` enum
  - `mozui-layout` — Taffy wrapper (stub — real implementation in Phase 2)
  - `mozui-reactive` — signal system (stub — real implementation in Phase 3)
  - `mozui-executor` — async runtime (stub — real implementation in Phase 5)
- [x] Create `examples/` directory with a `hello.rs` placeholder
- [x] Add `tracing` and `tracing-subscriber` as workspace dependencies
- [x] Verify `cargo build` succeeds with all stubs
- [x] Set up `.gitignore` (target/, .DS_Store, *.swp)

**File structure after this step:**

```
mozui/
├── Cargo.toml
├── crates/
│   ├── mozui/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs          // pub use mozui_app::*; pub use mozui_elements::*; etc.
│   ├── mozui-app/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── mozui-platform/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── traits.rs
│   ├── mozui-renderer/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── mozui-style/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── mozui-elements/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── mozui-events/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── mozui-layout/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── mozui-reactive/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── mozui-executor/
│       ├── Cargo.toml
│       └── src/lib.rs
└── examples/
    └── hello.rs
```

**Key decisions:**
- Each crate has minimal deps — only what it actually needs
- `mozui` (facade) depends on all other crates, users only ever `use mozui::*`
- Platform-specific code uses `#[cfg(target_os = "...")]` in `mozui-platform`

---

### 1.2 — Core Types (Week 1, Days 2-3)

Implement the foundational geometric and color types that everything else builds on.

**Tasks:**

- [x] `mozui-style/src/color.rs`:
  - `Color` struct (`r: f32, g: f32, b: f32, a: f32`) — all values 0.0..1.0
  - `Color::hex(hex: &str) -> Color` — parse "#RRGGBB" and "#RRGGBBAA"
  - `Color::rgb(r: u8, g: u8, b: u8) -> Color`
  - `Color::rgba(r: u8, g: u8, b: u8, a: f32) -> Color`
  - `Color::hsl(h: f32, s: f32, l: f32) -> Color`
  - `Color::lighter(&self, amount: f32) -> Color`
  - `Color::darker(&self, amount: f32) -> Color`
  - `Color::with_alpha(&self, alpha: f32) -> Color`
  - `impl From<&str> for Color` (hex shorthand)
  - Common color constants: `Color::RED`, `Color::GREEN`, `Color::BLUE`, `Color::WHITE`, `Color::BLACK`, `Color::TRANSPARENT`
  - Unit tests for hex parsing, HSL conversion, lighter/darker

- [x] `mozui-style/src/fill.rs`:
  - `Fill` enum: `Solid(Color)`, `LinearGradient { angle, stops }`, `RadialGradient { center, radius, stops }`
  - `impl From<Color> for Fill`
  - `impl From<&str> for Fill` (delegates to Color hex parsing)

- [x] `mozui-style/src/geometry.rs`:
  - `Point { x: f32, y: f32 }`
  - `Size { width: f32, height: f32 }`
  - `Rect { origin: Point, size: Size }`
  - `Corners { top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32 }`
  - `Rect::contains(point)`, `Rect::intersects(other)`, `Rect::union(other)`, `Rect::inset(amount)`
  - `Corners::uniform(radius: f32)`, `Corners::top(radius)`, `Corners::bottom(radius)`
  - `impl From<f32> for Corners` (uniform shorthand)
  - Implement `Copy`, `Clone`, `Debug`, `PartialEq` for all types
  - Unit tests for Rect::contains, Rect::intersects

- [x] `mozui-style/src/shadow.rs`:
  - `Shadow { offset_x, offset_y, blur, spread, color }`

- [x] `mozui-style/src/lib.rs`:
  - Re-export all types from submodules
  - `Style` struct (placeholder — will grow over time):
    ```rust
    pub struct Style {
        pub size: Size,            // explicit width/height
        pub background: Option<Fill>,
        pub corner_radii: Corners,
        pub border_width: f32,
        pub border_color: Color,
        pub shadow: Option<Shadow>,
        pub opacity: f32,
    }
    ```
  - `impl Default for Style`

---

### 1.3 — Platform Traits (Week 1, Days 3-4)

Define the abstract platform interface that all OS implementations must satisfy.

**Tasks:**

- [x] `mozui-platform/src/traits.rs`:
  ```rust
  pub trait Platform: 'static {
      fn run(self, callback: Box<dyn FnMut(PlatformEvent)>) -> !;
      fn open_window(&mut self, options: WindowOptions) -> Box<dyn PlatformWindow>;
      fn screens(&self) -> Vec<Screen>;
      fn set_cursor(&self, cursor: CursorStyle);
      fn clipboard_read(&self) -> Option<String>;
      fn clipboard_write(&self, text: &str);
  }

  pub trait PlatformWindow: 'static {
      fn id(&self) -> WindowId;
      fn bounds(&self) -> Rect;
      fn set_bounds(&mut self, bounds: Rect);
      fn scale_factor(&self) -> f32;
      fn is_focused(&self) -> bool;
      fn set_title(&mut self, title: &str);
      fn minimize(&mut self);
      fn maximize(&mut self);
      fn close(&mut self);
      fn request_redraw(&self);
      fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle;
      fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle;
  }
  ```

- [x] `mozui-platform/src/types.rs`:
  - `WindowOptions { title, size, min_size, max_size, position, resizable, visible, transparent }`
  - `Screen { bounds, work_area, scale_factor }`
  - `CursorStyle` enum (Arrow, Hand, Text, ResizeNS, ResizeEW, ResizeNESW, ResizeNWSE, Crosshair, NotAllowed)
  - `WindowId` newtype (u64)

- [x] `mozui-events/src/lib.rs`:
  - Full `PlatformEvent` enum (MouseMove, MouseDown, MouseUp, ScrollWheel, KeyDown, KeyUp, WindowResize, WindowMove, WindowFocused, WindowBlurred, WindowCloseRequested, ScaleFactorChanged, RedrawRequested)
  - `MouseButton` enum
  - `Key` enum (Character, Enter, Escape, Tab, Backspace, Delete, arrows, F-keys, etc.)
  - `Modifiers` struct (shift, ctrl, alt, meta)
  - `ScrollDelta` enum (Lines, Pixels)
  - All types derive `Debug, Clone, Copy` where applicable

---

### 1.4 — macOS Platform Shell (Week 1, Day 4 - Week 2, Day 3)

Implement the macOS platform shell using `objc2` to create a borderless NSWindow.

**This is the most complex task in Phase 1.** Take it step by step.

**Dependencies:** `objc2`, `objc2-foundation`, `objc2-app-kit`, `block2`, `raw-window-handle`

**Tasks:**

- [x] `mozui-platform/src/macos/mod.rs` — module root, re-exports
- [x] `mozui-platform/src/macos/app.rs` — `MacPlatform` struct:
  - Implements `Platform` trait
  - Creates `NSApplication` with `NSApplicationActivationPolicyRegular`
  - Sets up `NSApplicationDelegate` (via `objc2` class declaration):
    - `applicationDidFinishLaunching:` — activate the app
    - `applicationShouldTerminateAfterLastWindowClosed:` — return YES
  - `run()` method:
    - Creates the NSApplication
    - Calls the user callback with events
    - Enters `[NSApp run]` (this blocks forever)
  - Event translation: NSEvent → PlatformEvent

- [x] `mozui-platform/src/macos/window.rs` — `MacWindow` struct:
  - Creates `NSWindow` with style mask: `borderless | resizable | miniaturizable | closable`
  - `setTitlebarAppearsTransparent(true)`
  - `setTitleVisibility(NSWindowTitleVisibility::Hidden)`
  - `setMovableByWindowBackground(false)` — we handle dragging ourselves
  - Creates a custom `NSView` subclass (`MozuiView`):
    - `wantsLayer` returns YES
    - Layer is a `CAMetalLayer` (for wgpu Metal backend)
    - `acceptsFirstResponder` returns YES
    - Mouse event overrides (`mouseDown:`, `mouseUp:`, `mouseMoved:`, `mouseDragged:`, `scrollWheel:`, `mouseEntered:`, `mouseExited:`)
    - Keyboard event overrides (`keyDown:`, `keyUp:`, `flagsChanged:`)
    - `viewDidChangeBackingProperties` — detect scale factor changes
    - Sets up `NSTrackingArea` for mouse tracking
  - Window delegate (via `NSWindowDelegate`):
    - `windowDidResize:` → emit `PlatformEvent::WindowResize`
    - `windowDidMove:` → emit `PlatformEvent::WindowMove`
    - `windowDidBecomeKey:` → emit `PlatformEvent::WindowFocused`
    - `windowDidResignKey:` → emit `PlatformEvent::WindowBlurred`
    - `windowShouldClose:` → emit `PlatformEvent::WindowCloseRequested`
  - `raw_window_handle()` returns the NSView pointer
  - `raw_display_handle()` returns AppKit display handle

- [x] `mozui-platform/src/macos/event.rs` — event translation helpers:
  - `ns_event_to_key(NSEvent) -> Key`
  - `ns_modifiers_to_modifiers(NSEventModifierFlags) -> Modifiers`
  - `ns_point_to_point(NSPoint, view_height, scale_factor) -> Point` (flip Y axis — AppKit is bottom-left origin, mozui is top-left)

- [x] `mozui-platform/src/lib.rs` — platform factory:
  ```rust
  pub fn create_platform() -> Box<dyn Platform> {
      #[cfg(target_os = "macos")]
      { Box::new(macos::MacPlatform::new()) }
      // Other platforms added later
  }
  ```

**Verification**: At this point, you should be able to open a blank borderless window on macOS that receives and logs mouse/keyboard events.

**Pitfalls to watch for:**
- `objc2` API can be tricky — lean on the `objc2-app-kit` crate for typed AppKit bindings rather than raw message sends
- NSView Y-axis is flipped (origin at bottom-left) — convert early in the event pipeline
- Retina: `convertPointToBacking:` for pixel-precise coordinates, `backingScaleFactor` for DPI
- `NSTrackingArea` must be recreated when the view resizes — handle `updateTrackingAreas`
- The main thread must run the NSApplication event loop — all UI work happens on this thread

---

### 1.5 — wgpu Initialization (Week 2, Days 3-5)

Set up the wgpu rendering pipeline, create a surface from the platform window, and clear the screen to a solid color.

**Dependencies:** `wgpu`

**Tasks:**

- [x] `mozui-renderer/src/gpu.rs` — GPU context:
  ```rust
  pub struct GpuContext {
      instance: wgpu::Instance,
      adapter: wgpu::Adapter,
      device: wgpu::Device,
      queue: wgpu::Queue,
  }

  impl GpuContext {
      pub async fn new() -> Self {
          let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
              backends: wgpu::Backends::all(),
              ..Default::default()
          });
          let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
              power_preference: wgpu::PowerPreference::LowPower, // UI doesn't need high-perf GPU
              ..Default::default()
          }).await.expect("No suitable GPU adapter found");
          let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default())
              .await.expect("Failed to create GPU device");
          Self { instance, adapter, device, queue }
      }
  }
  ```

- [x] `mozui-renderer/src/surface.rs` — window surface:
  ```rust
  pub struct WindowSurface {
      surface: wgpu::Surface<'static>,
      config: wgpu::SurfaceConfiguration,
  }

  impl WindowSurface {
      pub fn new(gpu: &GpuContext, window: &dyn PlatformWindow) -> Self { ... }
      pub fn resize(&mut self, gpu: &GpuContext, width: u32, height: u32) { ... }
      pub fn get_current_texture(&self) -> wgpu::SurfaceTexture { ... }
      pub fn present(&self, frame: wgpu::SurfaceTexture) { ... }
  }
  ```
  - Surface format: prefer `Bgra8UnormSrgb` (native on macOS/Metal)
  - Present mode: `Fifo` (vsync) for smooth UI
  - Alpha mode: `Opaque` (or `PreMultiplied` if we want window transparency later)

- [x] `mozui-renderer/src/lib.rs` — `Renderer` struct:
  ```rust
  pub struct Renderer {
      gpu: GpuContext,
      surface: WindowSurface,
  }

  impl Renderer {
      pub fn new(window: &dyn PlatformWindow) -> Self { ... }
      pub fn resize(&mut self, size: Size) { ... }
      pub fn begin_frame(&mut self) -> Frame { ... }
  }

  pub struct Frame {
      encoder: wgpu::CommandEncoder,
      texture: wgpu::SurfaceTexture,
      view: wgpu::TextureView,
  }

  impl Frame {
      pub fn clear(&mut self, color: Color) { ... } // wgpu render pass with clear color
      pub fn finish(self, gpu: &GpuContext) { ... }  // submit + present
  }
  ```

**Verification**: Open a window and clear to a solid color (e.g., dark gray `#1e1e2e`). The window should resize correctly and re-render at the new size.

---

### 1.6 — Rect Shader (Week 3, Days 1-4)

Implement the SDF-based rounded rectangle shader that will render most UI elements.

**Tasks:**

- [x] `mozui-renderer/src/shaders/rect.wgsl` — the WGSL shader:
  - **Vertex shader**: Takes a per-instance vertex buffer with:
    - `bounds: vec4<f32>` (x, y, width, height)
    - `background: vec4<f32>` (RGBA color)
    - `corner_radii: vec4<f32>` (top-left, top-right, bottom-right, bottom-left)
    - `border_width: f32`
    - `border_color: vec4<f32>`
  - Expands each instance into a quad (4 vertices, 6 indices or triangle strip)
  - Passes local UV coordinates to fragment shader
  - **Fragment shader**:
    - Compute SDF for a rounded rectangle
    - Anti-aliased edge via `smoothstep`
    - Border rendering via SDF offset
    - Output premultiplied alpha

  ```wgsl
  // Signed distance function for a rounded rectangle
  fn rounded_rect_sdf(p: vec2<f32>, size: vec2<f32>, radii: vec4<f32>) -> f32 {
      // Select corner radius based on quadrant
      let r = select(
          select(radii.w, radii.z, p.x > 0.0),  // bottom-left or bottom-right
          select(radii.x, radii.y, p.x > 0.0),   // top-left or top-right
          p.y > 0.0
      );
      let q = abs(p) - size + r;
      return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r;
  }
  ```

- [x] `mozui-renderer/src/rect_pipeline.rs` — render pipeline setup:
  - Create `wgpu::RenderPipeline` with the rect shader
  - Vertex buffer layout matching the shader inputs
  - Uniform buffer for the view projection matrix (orthographic, maps logical pixels to clip space)
  - Blend state: premultiplied alpha blending
  - A method to draw a batch of rects:
    ```rust
    pub fn draw_rects(&mut self, frame: &mut Frame, rects: &[RectInstance]) { ... }
    ```

- [x] `mozui-renderer/src/draw.rs` — draw command abstraction:
  ```rust
  pub enum DrawCommand {
      Rect {
          bounds: Rect,
          background: Fill,
          corner_radii: Corners,
          border: Option<Border>,
      },
      // Text, Image, PushClip, PopClip — added in later phases
  }

  pub struct Border {
      pub width: f32,
      pub color: Color,
  }

  pub struct DrawList {
      commands: Vec<DrawCommand>,
  }
  ```

- [x] `mozui-renderer/src/lib.rs` — extend `Renderer`:
  - `fn render_draw_list(&mut self, draw_list: &DrawList)` — processes DrawCommands, batches rects, issues draw calls

**Verification**: Render a solid-colored rounded rectangle centered in the window. Try different colors, sizes, corner radii, and borders.

**Pitfalls:**
- Coordinate system: the vertex shader must transform from logical pixels (origin top-left) to wgpu clip space (-1..1, origin center, Y-up). Use an orthographic projection matrix.
- Anti-aliasing: the `smoothstep` width for the SDF edge should be ~0.5 logical pixels, scaled by the display's scale factor for Retina.
- Premultiplied alpha: multiply RGB by A before output. wgpu blend state must use `PremultipliedAlpha`.

---

### 1.7 — Element & Builder Foundations (Week 3, Day 4 - Week 4, Day 2)

Implement the `Element` trait and the `Div` struct with basic builder methods.

**Tasks:**

- [x] `mozui-elements/src/element.rs` — `Element` trait:
  ```rust
  pub trait Element: 'static {
      fn paint(&self, bounds: Rect, draw_list: &mut DrawList);
      fn children(&self) -> &[Box<dyn Element>];
  }
  ```
  (Minimal for now — `layout_style()` added in Phase 2, `handle_event()` in Phase 3)

- [x] `mozui-elements/src/div.rs` — `Div` struct and builder:
  - Struct fields:
    ```rust
    pub struct Div {
        style: Style,
        children: Vec<Box<dyn Element>>,
    }
    ```
  - Builder methods (consume `self`, return `Self`):
    - `.w(f32)`, `.h(f32)`, `.size(f32)` — set explicit size
    - `.w_full()`, `.h_full()` — 100% of parent (placeholder until layout works)
    - `.bg(impl Into<Fill>)` — background fill
    - `.rounded(f32)` — uniform corner radius
    - `.rounded_t(f32)`, `.rounded_b(f32)` — top/bottom corners
    - `.rounded_full()` — pill shape (radius = 9999)
    - `.border(f32, Color)` — border width and color
    - `.opacity(f32)` — element opacity
    - `.child(impl Element)` — add a single child
    - `.children(impl IntoIterator<Item = impl Element>)` — add multiple children
  - `impl Element for Div`:
    - `paint()`: emit a `DrawCommand::Rect` for self, then call `paint()` on each child
    - `children()`: return `&self.children`

- [x] `mozui-elements/src/lib.rs`:
  - `pub fn div() -> Div` — constructor function

**Verification**: Build a `Div` with builder methods and paint it. This is still hardcoded positioning (no layout) — children are painted at the parent's origin.

---

### 1.8 — App Lifecycle & Event Loop (Week 4)

Wire everything together: platform shell → event loop → renderer → element tree → screen.

**Tasks:**

- [x] `mozui-app/src/context.rs` — `Context` stub:
  ```rust
  pub struct Context {
      // Will grow significantly in later phases
      theme: Theme,
  }

  impl Context {
      pub fn theme(&self) -> &Theme { &self.theme }
  }
  ```

- [x] `mozui-app/src/app.rs` — `App` and `AppBuilder`:
  ```rust
  pub struct AppBuilder {
      theme: Option<Theme>,
      window_options: Option<WindowOptions>,
  }

  impl AppBuilder {
      pub fn theme(mut self, theme: Theme) -> Self { ... }
      pub fn window(mut self, options: WindowOptions) -> Self { ... }
      pub fn run(self, root: fn(&mut Context) -> Box<dyn Element>) -> ! { ... }
  }
  ```
  - `run()` implementation:
    1. Create platform via `create_platform()`
    2. Open window with options
    3. Create `Renderer` from the window
    4. Build initial element tree by calling `root(&mut cx)`
    5. Enter event loop:
       - On `RedrawRequested`: paint element tree → render → present
       - On `WindowResize`: resize renderer surface, re-render
       - On `WindowCloseRequested`: exit process
       - All other events: log for now (interaction added in Phase 3)

- [x] `mozui-style/src/theme.rs` — `Theme` struct:
  - Full theme struct as defined in IMPLEMENTATION.md
  - `Theme::dark() -> Self` — sensible dark theme defaults
  - `Theme::light() -> Self` — sensible light theme defaults

- [x] `mozui/src/lib.rs` — facade re-exports:
  ```rust
  pub use mozui_app::{App, Context};
  pub use mozui_elements::{div, Element};
  pub use mozui_style::{Color, Fill, Theme, Size, Point, Rect};
  pub use mozui_platform::WindowOptions;
  ```

- [x] `examples/hello.rs` — the Phase 1 milestone:
  ```rust
  use mozui::*;

  fn main() {
      App::new()
          .theme(Theme::dark())
          .window(WindowOptions {
              title: "Hello mozui".into(),
              size: Size::new(800.0, 600.0),
              ..Default::default()
          })
          .run(app);
  }

  fn app(cx: &mut Context) -> Box<dyn Element> {
      Box::new(
          div()
              .w(200.0)
              .h(100.0)
              .bg(Color::hex("#3b82f6"))
              .rounded(8.0)
      )
  }
  ```

**Verification**: `cargo run --example hello` opens a dark window with a blue rounded rectangle.

**Phase 1 complete checklist:**
- [x] Workspace builds with no warnings
- [x] macOS borderless window opens
- [x] wgpu renders to the window surface
- [x] At least one colored rounded rectangle is visible
- [x] Window resizes correctly (surface reconfigured, rect re-rendered)
- [x] Window close terminates the process
- [x] Keyboard/mouse events are received and logged

---

## Phase 2: Layout & Text (Weeks 5-8) ✅ COMPLETE

**Goal**: Lay out multiple elements using Flexbox and render text.

**Milestone**: A vertical stack of styled text labels with padding, spacing, and different font sizes.

**Status**: All core tasks complete. Taffy Flexbox layout with two-pass render (layout tree → compute → paint). Full set of Tailwind-style builder methods on Div. font-kit text shaping and glyph rasterization with shelf-packed R8 texture atlas. Text element with font_size, bold, color, italic. Verified working.

---

### 2.1 — Taffy Integration (Week 5)

Connect the element tree to Taffy for Flexbox layout.

**Tasks:**

- [x] Add `taffy` dependency to `mozui-layout`
- [x] `mozui-layout/src/lib.rs` — `LayoutEngine`:
  ```rust
  pub struct LayoutEngine {
      taffy: TaffyTree<()>,
      node_map: HashMap<ElementId, NodeId>,   // mozui element → Taffy node
      reverse_map: HashMap<NodeId, ElementId>, // Taffy node → mozui element
      dirty: HashSet<ElementId>,
  }
  ```
  - `fn build_tree(&mut self, root: &dyn Element)` — recursively create Taffy nodes mirroring the element tree
  - `fn compute(&mut self, available_space: Size)` — call `taffy.compute_layout()` on root
  - `fn get_layout(&self, element_id: ElementId) -> Layout` — returns computed `{ x, y, width, height }` for an element
  - `fn mark_dirty(&mut self, element_id: ElementId)` — mark a subtree for relayout
  - `fn rebuild_node(&mut self, element_id: ElementId, element: &dyn Element)` — update a single node's style

- [x] `mozui-elements/src/element.rs` — extend `Element` trait:
  ```rust
  fn layout_style(&self) -> taffy::Style;
  fn id(&self) -> ElementId;
  ```

- [x] `mozui-elements/src/element_id.rs`:
  - `ElementId` — unique ID for each element instance (u64, atomic counter)

- [x] `mozui-style/src/style.rs` — extend `Style` to include layout properties:
  ```rust
  pub struct Style {
      // Visual (existing)
      pub background: Option<Fill>,
      pub corner_radii: Corners,
      pub border_width: f32,
      pub border_color: Color,
      pub shadow: Option<Shadow>,
      pub opacity: f32,

      // Layout (new)
      pub display: Display,               // Flex (default)
      pub size: taffy::Size<Dimension>,
      pub min_size: taffy::Size<Dimension>,
      pub max_size: taffy::Size<Dimension>,
      pub padding: taffy::Rect<LengthPercentage>,
      pub margin: taffy::Rect<LengthPercentageAuto>,
      pub gap: taffy::Size<LengthPercentage>,
      pub flex_direction: FlexDirection,
      pub flex_wrap: FlexWrap,
      pub flex_grow: f32,
      pub flex_shrink: f32,
      pub flex_basis: Dimension,
      pub align_items: Option<AlignItems>,
      pub align_self: Option<AlignSelf>,
      pub justify_content: Option<JustifyContent>,
      pub position: Position,
      pub inset: taffy::Rect<LengthPercentageAuto>,
      pub overflow: taffy::Point<Overflow>,
  }
  ```

- [x] `mozui-elements/src/div.rs` — add layout builder methods:
  - `.flex()`, `.flex_row()`, `.flex_col()`
  - `.flex_grow(f32)`, `.flex_shrink(f32)`, `.flex_basis(f32)`
  - `.flex_wrap()`
  - `.items_start()`, `.items_center()`, `.items_end()`, `.items_stretch()`
  - `.justify_start()`, `.justify_center()`, `.justify_end()`, `.justify_between()`, `.justify_around()`, `.justify_evenly()`
  - `.padding(f32)`, `.padding_x(f32)`, `.padding_y(f32)`, `.padding_t(f32)`, `.padding_b(f32)`, `.padding_l(f32)`, `.padding_r(f32)`
  - `.margin(f32)`, `.margin_x(f32)`, `.margin_y(f32)`, etc.
  - `.gap(f32)`, `.gap_x(f32)`, `.gap_y(f32)`
  - `.min_w(f32)`, `.max_w(f32)`, `.min_h(f32)`, `.max_h(f32)`
  - `.absolute()`, `.relative()`
  - `.top(f32)`, `.right(f32)`, `.bottom(f32)`, `.left(f32)`, `.inset(f32)`
  - `.overflow_hidden()`, `.overflow_x_scroll()`, `.overflow_y_scroll()`

- [x] `mozui-elements/src/div.rs` — implement `layout_style()`:
  - Convert `Style` fields to `taffy::Style`
  - Map mozui's `f32` values to Taffy's `LengthPercentage::Length(value)`

- [x] Update `App::run()` to:
  1. Build element tree
  2. Build Taffy layout tree from elements
  3. Compute layout with `available_space = window_size`
  4. When painting, use computed layout bounds instead of hardcoded positions

**Verification**: Render three colored `div()`s in a row with `.flex_row().gap(16.0)` — they should be spaced correctly.

---

### 2.2 — Font Loading (Week 6, Days 1-3)

Set up font-kit for font discovery and loading.

**Dependencies:** `font-kit`, `pathfinder_geometry` (font-kit dependency)

**Tasks:**

- [x] `mozui-text/src/font_system.rs`:
  ```rust
  pub struct FontSystem {
      source: SystemSource,           // font-kit's system font source
      loaded: HashMap<FontId, Font>,  // Cache of loaded fonts
      next_id: FontId,
  }

  pub struct FontId(u32);

  impl FontSystem {
      pub fn new() -> Self { ... }
      pub fn load_family(&mut self, family: &str, weight: FontWeight, style: FontStyle) -> FontId { ... }
      pub fn get_font(&self, id: FontId) -> &Font { ... }
      pub fn system_default(&mut self) -> FontId { ... }
      pub fn system_monospace(&mut self) -> FontId { ... }
  }
  ```
  - Use `font_kit::source::SystemSource` for font discovery
  - Handle fallback: if requested family not found, fall back to system default
  - Load font into memory via `font_kit::font::Font::from_handle()`

- [x] `mozui-text/src/types.rs`:
  ```rust
  pub struct TextStyle {
      pub font_family: FontFamily,
      pub font_size: f32,
      pub font_weight: FontWeight,
      pub font_style: FontStyle,
      pub line_height: LineHeight,
      pub letter_spacing: f32,
      pub color: Color,
  }

  pub enum FontFamily {
      System,
      Monospace,
      Named(String),
  }

  pub enum FontWeight { Thin, Light, Regular, Medium, SemiBold, Bold, ExtraBold, Black }
  pub enum FontStyle { Normal, Italic }
  pub enum LineHeight { Relative(f32), Absolute(f32) } // 1.5x or 24px
  ```

---

### 2.3 — Text Shaping & Layout (Week 6, Day 3 - Week 7, Day 2)

Shape text into positioned glyphs using font-kit.

**Tasks:**

- [x] `mozui-text/src/shaping.rs` — text shaping:
  ```rust
  pub struct ShapedRun {
      pub glyphs: Vec<ShapedGlyph>,
      pub width: f32,
  }

  pub struct ShapedGlyph {
      pub glyph_id: u32,
      pub x_offset: f32,
      pub y_offset: f32,
      pub x_advance: f32,
      pub font_id: FontId,
  }
  ```
  - Use `font_kit::font::Font::glyph_for_char()` for character → glyph mapping
  - Use `font_kit::font::Font::advance()` for glyph advances
  - Apply `letter_spacing` between glyphs
  - For v1: simple left-to-right shaping without HarfBuzz. This works for Latin/Cyrillic. Complex scripts (Arabic, Devanagari) need HarfBuzz — defer to later.

- [x] `mozui-text/src/layout.rs` — text layout (line breaking):
  ```rust
  pub struct TextLayout {
      pub lines: Vec<TextLine>,
      pub size: Size,
  }

  pub struct TextLine {
      pub glyphs: Vec<PositionedGlyph>,
      pub baseline_y: f32,
      pub width: f32,
      pub height: f32,
  }

  pub struct PositionedGlyph {
      pub glyph_id: u32,
      pub x: f32,
      pub y: f32,
      pub font_id: FontId,
  }
  ```
  - `fn layout_text(text: &str, style: &TextStyle, max_width: Option<f32>, font_system: &FontSystem) -> TextLayout`
  - Simple line breaking: break at whitespace when exceeding max_width
  - Compute line height from font metrics + `TextStyle::line_height`
  - Position glyphs with baseline alignment

- [x] `mozui-text/src/cache.rs` — text layout cache:
  - Cache `TextLayout` results keyed by `(text, style, max_width)`
  - LRU eviction with configurable max entries (default: 1024)

---

### 2.4 — Glyph Rasterization & Atlas (Week 7, Days 2-5)

Rasterize glyphs to bitmaps and store them in a GPU texture atlas.

**Tasks:**

- [x] `mozui-text/src/rasterizer.rs`:
  - Use `font_kit::font::Font::rasterize_glyph()` to produce glyph bitmaps
  - Rasterize at the requested size × scale_factor for crisp rendering
  - Output: `GlyphBitmap { width, height, data: Vec<u8>, bearing_x, bearing_y }`
  - Quantize sizes to avoid excessive cache entries (round to nearest 0.5px)

- [x] `mozui-renderer/src/atlas.rs` — texture atlas:
  ```rust
  pub struct TextureAtlas {
      texture: wgpu::Texture,
      size: u32,                              // 1024 initially
      allocator: ShelfAllocator,
      entries: HashMap<AtlasKey, AtlasRegion>,
  }

  pub struct AtlasKey {
      font_id: FontId,
      glyph_id: u32,
      size_px: u16,
  }

  pub struct AtlasRegion {
      x: u32, y: u32,
      width: u32, height: u32,
      bearing_x: f32, bearing_y: f32,
  }
  ```
  - **Shelf allocator**: Simple row-based packing. Track rows (shelves) of varying heights. Each glyph goes into the first shelf with enough vertical space. If no shelf fits, create a new one.
  - `fn get_or_insert(&mut self, key: AtlasKey, bitmap: &GlyphBitmap, queue: &wgpu::Queue) -> AtlasRegion`
  - When atlas is full:
    1. Try to grow (double size, up to 4096)
    2. If at max size, evict LRU entries and repack
    3. Log a warning via `tracing::warn!`

---

### 2.5 — Glyph Shader (Week 7, Day 5 - Week 8, Day 2)

Add a shader for rendering text glyphs from the atlas texture.

**Tasks:**

- [x] `mozui-renderer/src/shaders/glyph.wgsl`:
  - **Vertex shader**: Per-instance data:
    - `bounds: vec4<f32>` (x, y, width, height — screen-space quad)
    - `uv: vec4<f32>` (u_min, v_min, u_max, v_max — atlas coordinates)
    - `color: vec4<f32>` (text color, premultiplied alpha)
  - Expand instance to quad, interpolate UVs
  - **Fragment shader**:
    - Sample atlas texture
    - For grayscale glyphs: use sampled alpha × text color
    - Output premultiplied alpha

- [x] `mozui-renderer/src/glyph_pipeline.rs`:
  - Create `wgpu::RenderPipeline` for glyph rendering
  - Bind group for the atlas texture + sampler
  - `fn draw_glyphs(&mut self, frame: &mut Frame, glyphs: &[GlyphInstance])`

- [x] Extend `DrawCommand` enum:
  ```rust
  DrawCommand::Text {
      glyphs: Vec<PositionedGlyph>,
      color: Color,
      clip: Option<Rect>,
  }
  ```

- [x] `Renderer::render_draw_list()` — handle `DrawCommand::Text`:
  - For each glyph: look up (or rasterize + insert) in atlas → create `GlyphInstance`
  - Batch all glyph instances and draw in one call

---

### 2.6 — Text Element (Week 8, Days 2-4)

Create the `text()` element and builder.

**Tasks:**

- [x] `mozui-elements/src/text.rs`:
  ```rust
  pub struct Text {
      content: String,
      style: TextStyle,
      id: ElementId,
  }

  impl Text {
      pub fn font_size(mut self, size: f32) -> Self { ... }
      pub fn color(mut self, color: Color) -> Self { ... }
      pub fn bold(mut self) -> Self { ... }
      pub fn italic(mut self) -> Self { ... }
      pub fn font(mut self, family: impl Into<FontFamily>) -> Self { ... }
      pub fn line_height(mut self, lh: impl Into<LineHeight>) -> Self { ... }
      pub fn letter_spacing(mut self, spacing: f32) -> Self { ... }
      pub fn center(mut self) -> Self { ... }  // text-align center
  }

  impl Element for Text {
      fn layout_style(&self) -> taffy::Style {
          // Text nodes are leaf nodes in Taffy
          // Use taffy's `measure` function to report intrinsic size based on shaped text
      }
      fn paint(&self, bounds: Rect, draw_list: &mut DrawList) {
          // Layout text within bounds, emit DrawCommand::Text
      }
  }
  ```

- [x] `mozui-elements/src/lib.rs` — `pub fn text(content: impl Into<String>) -> Text`

- [x] Taffy leaf node measurement:
  - Register a `MeasureFunc` for text nodes that returns the computed text size
  - The measure function calls `layout_text()` with the available width constraint

---

### 2.7 — Theme Integration (Week 8, Days 4-5)

Flesh out the theme struct and make it available in components.

**Tasks:**

- [x] Complete `Theme::dark()` with all fields populated (sensible defaults):
  - Background: `#1e1e2e`, Surface: `#313244`, Primary: `#cba6f7`
  - Text colors, borders, shadows, spacing scale, font sizes, radii
  - Default font: system sans-serif
  - Default mono font: system monospace

- [x] Complete `Theme::light()` — inverted luminance, same hues

- [x] Ensure `cx.theme()` is available in the render path

- [x] Update hello example:
  ```rust
  fn app(cx: &mut Context) -> impl Element {
      div()
          .w_full()
          .h_full()
          .bg(cx.theme().background)
          .flex_col()
          .gap(cx.theme().spacing.md)
          .padding(cx.theme().spacing.lg)
          .child(text("Hello, mozui!").font_size(32.0).bold().color(cx.theme().text_primary))
          .child(text("A modern GUI library for Rust.").color(cx.theme().text_secondary))
          .child(text("Built with wgpu.").font_size(12.0).color(cx.theme().text_tertiary))
  }
  ```

**Phase 2 complete checklist:**
- [x] Multiple elements lay out correctly using Flexbox
- [x] `.flex_row()`, `.flex_col()`, `.gap()`, `.padding()`, `.items_center()` all work
- [x] Text renders with proper font loading and shaping
- [x] Text wraps correctly within constrained widths
- [x] Multiple font sizes and weights work
- [x] Theme colors apply correctly
- [x] No visual artifacts on Retina displays

---

## Phase 3: Reactivity & Interaction (Weeks 9-12) ✅ COMPLETE

**Goal**: Interactive components with reactive state. A working counter app.

**Milestone**: Click a button to increment a counter displayed on screen.

**Status**: All tasks complete. Signal system with hook-style `cx.use_signal()`, NSEvent→PlatformEvent translation, hit-test-based click dispatch, keyboard events, hover cursor changes, `div().on_click()` and `div().on_key_down()` handlers, automatic re-render on state mutation. Counter example fully working with buttons and keyboard input.

---

### 3.1 — Arena & Signal Implementation (Week 9)

Build the centralized arena and signal system.

**Tasks:**

- [ ] `mozui-reactive/src/arena.rs` — arena storage:
  ```rust
  pub struct Arena {
      slots: Vec<Option<Box<dyn Any>>>,  // Type-erased signal storage
      free_list: Vec<usize>,              // Reusable slot indices
  }

  impl Arena {
      pub fn insert<T: 'static>(&mut self, value: T) -> SlotId { ... }
      pub fn get<T: 'static>(&self, id: SlotId) -> &T { ... }
      pub fn get_mut<T: 'static>(&mut self, id: SlotId) -> &mut T { ... }
      pub fn remove(&mut self, id: SlotId) { ... }
  }
  ```

- [ ] `mozui-reactive/src/signal.rs` — signal handles:
  ```rust
  #[derive(Copy, Clone)]
  pub struct Signal<T> {
      slot_id: SlotId,
      _marker: PhantomData<T>,
  }

  #[derive(Copy, Clone)]
  pub struct SetSignal<T> {
      slot_id: SlotId,
      _marker: PhantomData<T>,
  }

  impl<T: 'static> Signal<T> {
      pub fn get<'a>(&self, cx: &'a Context) -> &'a T {
          cx.track_read(self.slot_id);
          cx.arena.get(self.slot_id)
      }
  }

  impl<T: 'static> SetSignal<T> {
      pub fn set(&self, cx: &mut Context, value: T) {
          *cx.arena.get_mut(self.slot_id) = value;
          cx.notify_subscribers(self.slot_id);
      }

      pub fn update(&self, cx: &mut Context, f: impl FnOnce(&mut T)) {
          f(cx.arena.get_mut(self.slot_id));
          cx.notify_subscribers(self.slot_id);
      }
  }
  ```

- [ ] `mozui-reactive/src/subscriptions.rs` — dependency tracking:
  ```rust
  pub struct SubscriptionManager {
      /// signal → set of views that depend on it
      signal_to_views: HashMap<SlotId, HashSet<ViewId>>,
      /// view → set of signals it read during last render
      view_to_signals: HashMap<ViewId, HashSet<SlotId>>,
  }

  impl SubscriptionManager {
      pub fn track_read(&mut self, signal: SlotId, view: ViewId) { ... }
      pub fn clear_view(&mut self, view: ViewId) { ... }
      pub fn get_subscribers(&self, signal: SlotId) -> &HashSet<ViewId> { ... }
  }
  ```

- [ ] `mozui-reactive/src/tracking.rs` — tracking scope:
  ```rust
  pub struct TrackingScope {
      pub view_id: ViewId,
      pub reads: Vec<SlotId>,
  }
  ```

- [ ] Extend `Context`:
  - Add `arena: Arena`
  - Add `subscriptions: SubscriptionManager`
  - Add `current_scope: Option<TrackingScope>`
  - Add `dirty_views: HashSet<ViewId>`
  - `fn use_signal<T: 'static>(&mut self, initial: T) -> (Signal<T>, SetSignal<T>)`
  - `fn track_read(&mut self, slot_id: SlotId)` — records dependency if inside a tracking scope
  - `fn notify_subscribers(&mut self, slot_id: SlotId)` — marks dependent views dirty
  - `fn is_dirty(&self) -> bool`

- [ ] Unit tests:
  - Signal create/read/write
  - Dependency tracking (read during scope → subscription created)
  - Dirty marking (write signal → subscribed views marked dirty)
  - Subscription cleanup (clear old deps on re-render)

---

### 3.2 — View System & Re-rendering (Week 10)

Introduce the `View` concept — a component instance with identity and state.

**Tasks:**

- [ ] `mozui-reactive/src/view.rs`:
  ```rust
  pub struct ViewId(u64);

  pub struct View {
      id: ViewId,
      component: Box<dyn Fn(&mut Context) -> Box<dyn Element>>,
      element: Option<Box<dyn Element>>,  // Cached output from last render
      signals: Vec<SlotId>,                // Signals owned by this view
  }
  ```

- [ ] `mozui-app/src/view_tree.rs` — view tree management:
  ```rust
  pub struct ViewTree {
      views: HashMap<ViewId, View>,
      root: ViewId,
      dirty: HashSet<ViewId>,
  }

  impl ViewTree {
      pub fn mount(&mut self, component: ..., cx: &mut Context) -> ViewId { ... }
      pub fn render_dirty(&mut self, cx: &mut Context) { ... }
      pub fn unmount(&mut self, view_id: ViewId, cx: &mut Context) { ... }
  }
  ```

- [ ] Render cycle integration:
  1. Signals mutated → views marked dirty
  2. Before layout: re-render dirty views (call component function, get new element tree)
  3. During re-render: old subscriptions cleared, new ones tracked
  4. After re-render: dirty layout nodes marked, Taffy recomputes

- [ ] Signal lifecycle:
  - `cx.use_signal()` — on first render, creates a new signal. On re-render, returns the existing signal (keyed by call-site order within the component, like React hooks).
  - Track which signals belong to which view → clean up on unmount

- [ ] Hook ordering:
  - Like React, `use_signal` calls must be in consistent order across re-renders
  - Track by index within the view's render call
  - Panic in debug mode if hook count changes between renders

---

### 3.3 — Mouse Event Dispatch (Week 11, Days 1-3)

Dispatch mouse events from the platform to the correct element.

**Tasks:**

- [ ] `mozui-events/src/dispatch.rs` — event dispatcher:
  ```rust
  pub struct EventDispatcher {
      hovered_element: Option<ElementId>,
      pressed_element: Option<(ElementId, MouseButton)>,
      click_detector: ClickDetector,
  }
  ```

- [ ] Hit testing:
  - `fn hit_test(root: &dyn Element, layout: &LayoutEngine, point: Point) -> Option<ElementId>`
  - Walk element tree in reverse paint order (children back-to-front, depth-first)
  - Check if point is within element's computed layout bounds
  - Respect `overflow_hidden` (don't hit children outside parent clip)
  - Return the frontmost interactive element under the cursor

- [ ] Click detection:
  - Track mouse down position and timestamp
  - Mouse up on same element within threshold → click
  - Track consecutive clicks within 500ms / 4px → double/triple click
  - `ClickEvent { position, button, click_count, modifiers }`

- [ ] Hover tracking:
  - Compare current hovered element with previous frame
  - If changed: fire `on_mouse_leave` on old, `on_mouse_enter` on new
  - Update cursor style based on hovered element's `.cursor()` setting

- [ ] Element event handlers — extend `Div`:
  ```rust
  struct EventHandlers {
      on_click: Option<Box<dyn Fn(&ClickEvent, &mut Context)>>,
      on_mouse_down: Option<Box<dyn Fn(&MouseEvent, &mut Context)>>,
      on_mouse_up: Option<Box<dyn Fn(&MouseEvent, &mut Context)>>,
      on_mouse_enter: Option<Box<dyn Fn(&MouseEvent, &mut Context)>>,
      on_mouse_leave: Option<Box<dyn Fn(&MouseEvent, &mut Context)>>,
      on_scroll: Option<Box<dyn Fn(&ScrollEvent, &mut Context)>>,
  }
  ```
  - Builder methods: `.on_click(handler)`, `.on_mouse_down(handler)`, etc.

- [ ] Wire into event loop:
  - Platform `MouseMove` → hit test → hover tracking
  - Platform `MouseDown` → hit test → dispatch to element → start click detection
  - Platform `MouseUp` → hit test → dispatch to element → complete click detection
  - Platform `ScrollWheel` → hit test → dispatch to target

---

### 3.4 — Keyboard Event Dispatch (Week 11, Days 3-5)

Dispatch keyboard events to the focused element (focus system is simplified for now — full focus scopes come in Phase 4).

**Tasks:**

- [ ] Simple focus tracking (placeholder before full focus system):
  - Track `focused_element: Option<ElementId>` in `Context`
  - Clicking an element with `on_key_down` handler focuses it
  - Keyboard events dispatch to focused element

- [ ] Element keyboard handlers — extend `Div`:
  ```rust
  on_key_down: Option<Box<dyn Fn(&KeyEvent, &mut Context)>>,
  on_key_up: Option<Box<dyn Fn(&KeyEvent, &mut Context)>>,
  ```
  - Builder: `.on_key_down(handler)`, `.on_key_up(handler)`

---

### 3.5 — Interactive States (Week 12, Days 1-3)

Implement hover and active visual states.

**Tasks:**

- [ ] `mozui-style/src/style.rs` — style modifiers:
  ```rust
  pub struct StyleModifiers {
      pub hover: Option<Box<dyn Fn(StyleOverride) -> StyleOverride>>,
      pub active: Option<Box<dyn Fn(StyleOverride) -> StyleOverride>>,
      pub focused: Option<Box<dyn Fn(StyleOverride) -> StyleOverride>>,
  }

  pub struct StyleOverride {
      pub background: Option<Fill>,
      pub border_color: Option<Color>,
      pub opacity: Option<f32>,
      // ... subset of visual style properties
  }
  ```

- [ ] Builder methods:
  ```rust
  .hover(|s| s.bg(Color::hex("#444")))
  .active(|s| s.bg(Color::hex("#333")))
  .focused(|s| s.border_color(Color::hex("#3b82f6")))
  ```

- [ ] During paint: check if element is hovered/pressed/focused → apply style overrides before emitting draw commands

---

### 3.6 — Button Element (Week 12, Days 3-5)

Create a basic interactive button.

**Tasks:**

- [ ] `mozui-elements/src/button.rs`:
  ```rust
  pub struct Button {
      label: String,
      div: Div,  // Internally, button is a styled div with a text child
  }

  pub fn button(label: impl Into<String>) -> Button { ... }
  ```
  - Default styling: padding, rounded corners, background from theme, cursor: hand
  - Default hover/active states from theme
  - `.on_click(handler)` — delegates to inner div
  - Supports all div builder methods (composition)

- [ ] `examples/counter.rs` — Phase 3 milestone:
  ```rust
  fn counter(cx: &mut Context) -> impl Element {
      let (count, set_count) = cx.use_signal(0i32);

      div()
          .flex_col().items_center().justify_center().gap(16.0)
          .child(text(&format!("{}", count.get(cx))).font_size(48.0).bold())
          .child(div().flex_row().gap(8.0)
              .child(button("-").on_click(move |_, cx| set_count.update(cx, |n| *n -= 1)))
              .child(button("+").on_click(move |_, cx| set_count.update(cx, |n| *n += 1)))
          )
  }
  ```

**Phase 3 complete checklist:**
- [ ] Signals create, read, and write correctly
- [ ] Changing a signal triggers re-render of dependent views only
- [ ] Mouse clicks dispatch to the correct element
- [ ] Hover state visually changes elements
- [ ] Button click increments/decrements the counter
- [ ] No stale state after multiple rapid clicks
- [ ] Memory doesn't grow unboundedly (signal cleanup works)

---

## Phase 4: Focus & Actions (Weeks 13-16) ✅ COMPLETE

**Goal**: Full keyboard navigation with focus scopes, and a keybinding system for named actions.

**Milestone**: A form with text inputs, tab navigation between them, and keyboard shortcuts.

**Status**: All core tasks complete. Focus system with click-to-focus, Tab/Shift+Tab cycling, text cursor differentiation. TextInput element with full editing support. Action system with `actions!` macro for defining named action types. Keybinding registry with `KeyCombo::parse()` for human-readable combos (e.g. "cmd-q", "ctrl-shift-z"), contextual binding support, and dispatch integration in the event loop. Form example demonstrates text inputs with Tab navigation and keybinding-driven Quit action. Deferred: FocusHandle/FocusScope abstractions, focus ring visuals (not needed until complex nested UIs).

---

### 4.1 — FocusHandle Implementation (Week 13)

**Tasks:**

- [ ] `mozui-elements/src/focus.rs`:
  ```rust
  #[derive(Clone)]
  pub struct FocusHandle {
      id: FocusId,
  }

  pub struct FocusId(u64);

  impl FocusHandle {
      pub fn focus(&self, cx: &mut Context) { ... }
      pub fn blur(&self, cx: &mut Context) { ... }
      pub fn is_focused(&self, cx: &Context) -> bool { ... }
  }
  ```

- [ ] `cx.use_focus_handle() -> FocusHandle` — allocates a new focus handle scoped to the current view

- [ ] Focus manager in `Context`:
  ```rust
  struct FocusManager {
      focused: Option<FocusId>,
      focus_order: Vec<FocusId>,   // All focusable elements in tree order
      scopes: Vec<FocusScope>,
  }
  ```

- [ ] Builder: `.focusable(&focus_handle)` — marks an element as focusable and associates the handle

- [ ] Clicking a focusable element focuses it (update event dispatch from 3.3)

---

### 4.2 — Focus Scopes (Week 14, Days 1-3)

**Tasks:**

- [ ] `FocusScope` struct:
  ```rust
  struct FocusScope {
      id: FocusScopeId,
      children: Vec<FocusableEntry>,  // Focusable elements within this scope
      parent: Option<FocusScopeId>,
  }

  struct FocusableEntry {
      focus_id: FocusId,
      element_id: ElementId,
      tab_index: Option<i32>,         // -1 = skip tab, None = auto (tree order)
  }
  ```

- [ ] Builder: `.focus_scope()` — marks an element as a focus scope boundary

- [ ] Build focus scope tree during element tree construction:
  - Walk element tree, collect focusable elements into their nearest scope
  - Order by tree position (depth-first)

- [ ] Tab navigation:
  - Intercept `Tab` key at the event loop level (before element dispatch)
  - Find current focused element's position in its scope
  - Move to next (Tab) or previous (Shift+Tab)
  - Wrap at scope boundaries
  - If a scope is a "trap" (like a modal), don't escape

---

### 4.3 — Tab Navigation (Week 14, Days 3-5)

**Tasks:**

- [ ] Tab key handling in event loop:
  ```rust
  fn handle_tab(&mut self, shift: bool, cx: &mut Context) {
      let current = cx.focus_manager.focused;
      let scope = cx.focus_manager.scope_for(current);
      let focusables = &scope.children;

      let current_idx = focusables.iter().position(|f| Some(f.focus_id) == current);

      let next_idx = if shift {
          current_idx.map(|i| if i == 0 { focusables.len() - 1 } else { i - 1 })
              .unwrap_or(focusables.len() - 1)
      } else {
          current_idx.map(|i| (i + 1) % focusables.len())
              .unwrap_or(0)
      };

      focusables[next_idx].focus_id.focus(cx);
  }
  ```

- [ ] Skip elements with `tab_index(-1)`
- [ ] Visual focus indicator: elements with `.focused()` style modifier show a focus ring
- [ ] Default focus ring: 2px outline in `cx.theme().border_focus` color

---

### 4.4 — Action System (Week 15)

**Tasks:**

- [ ] `mozui-app/src/actions.rs`:
  ```rust
  pub trait Action: std::any::Any + std::fmt::Debug + Send + Sync {
      fn name(&self) -> &'static str;
      fn namespace(&self) -> &'static str;
      fn boxed_clone(&self) -> Box<dyn Action>;
      fn as_any(&self) -> &dyn Any;
  }
  ```

- [ ] `actions!` macro:
  ```rust
  macro_rules! actions {
      ($namespace:ident, [$($action:ident),* $(,)?]) => {
          $(
              #[derive(Debug, Clone, Copy)]
              pub struct $action;

              impl Action for $action {
                  fn name(&self) -> &'static str { stringify!($action) }
                  fn namespace(&self) -> &'static str { stringify!($namespace) }
                  fn boxed_clone(&self) -> Box<dyn Action> { Box::new(*self) }
                  fn as_any(&self) -> &dyn Any { self }
              }
          )*
      };
  }
  ```

- [ ] Action handler on elements:
  ```rust
  .on_action::<Copy>(|action, cx| { ... })
  .context("Editor")  // Names the action context for this element and its children
  ```

---

### 4.5 — Keybinding Registry (Week 15-16)

**Tasks:**

- [ ] `mozui-app/src/keybindings.rs`:
  ```rust
  pub struct KeybindingRegistry {
      global: Vec<Keybinding>,
      contextual: HashMap<String, Vec<Keybinding>>,  // context name → bindings
  }

  pub struct Keybinding {
      combo: KeyCombo,
      action: Box<dyn Action>,
  }

  pub struct KeyCombo {
      key: Key,
      modifiers: Modifiers,
  }
  ```

- [ ] Key combo parsing:
  - `KeyCombo::parse("cmd-s")` → `KeyCombo { key: Key::Character('s'), modifiers: Modifiers { meta: true, .. } }`
  - `cmd` maps to `meta` on macOS, `ctrl` on Windows/Linux
  - Support: `cmd`, `ctrl`, `alt`, `shift`, `super`
  - Validate key names at parse time, return `Result`

- [ ] Registration API:
  ```rust
  app.keybindings(|kb| {
      kb.bind("cmd-q", Quit);
      kb.context("Editor")
          .bind("cmd-c", Copy)
          .bind("cmd-v", Paste);
  });
  ```

- [ ] Dispatch:
  - On `KeyDown` event, before element dispatch:
  - Build `KeyCombo` from the event
  - Walk up from focused element, check each element's `.context()` for matching bindings
  - Check global bindings last
  - If match found: find the nearest ancestor with an `on_action::<T>` handler and invoke it
  - If no match: fall through to element's `on_key_down`

---

### 4.6 — Text Input Element (Week 16)

Build a basic single-line text input.

**Tasks:**

- [ ] `mozui-elements/src/text_input.rs`:
  ```rust
  pub struct TextInput {
      div: Div,
      focus_handle: FocusHandle,
      // Props
      placeholder: Option<String>,
      value: Option<Signal<String>>,
      on_change: Option<Box<dyn Fn(String, &mut Context)>>,
      on_submit: Option<Box<dyn Fn(&mut Context)>>,
  }

  pub fn text_input() -> TextInput { ... }
  ```

- [ ] Features:
  - Display current value text (or placeholder in tertiary color when empty)
  - Cursor (blinking caret) — vertical line at insertion point
  - Text insertion: `on_key_down` with `event.text` → append to value
  - Backspace: delete character before cursor
  - Delete: delete character after cursor
  - Arrow keys: move cursor left/right
  - Home/End: move cursor to start/end
  - Cmd+A: select all (visual selection deferred — just move cursor for now)
  - Enter: fire `on_submit`
  - Visual: border, focus ring when focused, padding

- [ ] Builder methods:
  ```rust
  .placeholder("Enter text...")
  .value(signal)
  .on_change(|new_value, cx| { ... })
  .on_submit(|cx| { ... })
  ```

- [ ] `examples/form.rs` — Phase 4 milestone:
  ```rust
  fn form(cx: &mut Context) -> impl Element {
      let (name, set_name) = cx.use_signal(String::new());
      let (email, set_email) = cx.use_signal(String::new());

      div().flex_col().gap(16.0).padding(24.0)
          .child(text("Sign Up").font_size(24.0).bold())
          .child(
              div().flex_col().gap(4.0)
                  .child(text("Name").font_size(13.0))
                  .child(text_input().placeholder("Your name").value(name).on_change(move |v, cx| set_name.set(cx, v)))
          )
          .child(
              div().flex_col().gap(4.0)
                  .child(text("Email").font_size(13.0))
                  .child(text_input().placeholder("you@example.com").value(email).on_change(move |v, cx| set_email.set(cx, v)))
          )
          .child(button("Submit").on_click(move |_, cx| { /* ... */ }))
  }
  ```

**Phase 4 complete checklist:**
- [ ] Tab cycles through focusable elements
- [ ] Shift+Tab moves backwards
- [ ] Focus scopes trap tab navigation
- [ ] Focus ring is visible on focused elements
- [ ] `actions!` macro compiles and creates action types
- [ ] Keybindings dispatch to action handlers
- [ ] Contextual keybindings override global ones
- [ ] Text input accepts and displays typed text
- [ ] Cursor moves with arrow keys
- [ ] Backspace/Delete work

---

## Phase 5: Async & Window Chrome (Weeks 17-20)

**Goal**: Async task execution and custom window decorations.

**Milestone**: A window with custom title bar, minimize/maximize/close buttons, that fetches data asynchronously.

---

### 5.1 — Main Thread Executor (Week 17)

**Tasks:**

- [ ] `mozui-executor/src/task.rs`:
  ```rust
  pub struct Task {
      future: Pin<Box<dyn Future<Output = ()>>>,
      waker: Option<Waker>,
  }
  ```

- [ ] `mozui-executor/src/executor.rs`:
  ```rust
  pub struct Executor {
      ready_queue: VecDeque<TaskId>,
      tasks: HashMap<TaskId, Task>,
      next_id: TaskId,
  }

  impl Executor {
      pub fn spawn(&mut self, future: impl Future<Output = ()> + 'static) -> TaskHandle { ... }
      pub fn poll_ready(&mut self, cx: &mut Context, max_polls: usize) { ... }
      pub fn has_pending(&self) -> bool { ... }
  }
  ```

- [ ] Custom `Waker` implementation:
  - When a task's waker is called, enqueue the task ID into `ready_queue`
  - The waker holds a `Sender` that signals the event loop to wake up
  - Platform integration: the sender triggers a platform-specific "wake" (e.g., `[NSApp postEvent]` on macOS) so the event loop processes pending tasks

- [ ] `TaskHandle<T>`:
  ```rust
  pub struct TaskHandle<T> {
      result: Arc<Mutex<Option<T>>>,
      task_id: TaskId,
  }

  impl<T> Future for TaskHandle<T> {
      type Output = T;
      fn poll(self: Pin<&mut Self>, cx: &mut FutContext) -> Poll<T> { ... }
  }
  ```

- [ ] `cx.spawn()` integration — creates a task on the executor, returns `TaskHandle`

---

### 5.2 — Background Thread Pool (Week 17-18)

**Tasks:**

- [ ] `mozui-executor/src/thread_pool.rs`:
  ```rust
  pub struct ThreadPool {
      workers: Vec<JoinHandle<()>>,
      sender: Sender<BoxedFuture>,
  }

  impl ThreadPool {
      pub fn new(num_threads: usize) -> Self { ... }
      pub fn spawn<F>(&self, future: F) -> TaskHandle<F::Output>
      where
          F: Future + Send + 'static,
          F::Output: Send + 'static
      { ... }
  }
  ```

- [ ] Worker threads:
  - Each thread runs a simple block_on loop
  - Pull tasks from the shared channel
  - When a task completes, send the result back to the main thread via a result channel

- [ ] `cx.background()` integration:
  - Submits future to thread pool
  - Returns `TaskHandle` that can be `.await`ed from a main-thread task

- [ ] Integration with event loop:
  - Each frame: check result channel for completed background tasks
  - Deliver results, which may mutate signals and trigger re-renders

---

### 5.3 — Timer Support (Week 18)

**Tasks:**

- [ ] `mozui-executor/src/timer.rs`:
  ```rust
  pub struct TimerManager {
      pending: BinaryHeap<Reverse<TimerEntry>>,  // Min-heap by deadline
      next_id: TimerId,
  }

  struct TimerEntry {
      id: TimerId,
      deadline: Instant,
      callback: TimerCallback,
  }

  enum TimerCallback {
      Once(Box<dyn FnOnce(&mut Context)>),
      Repeating {
          callback: Box<dyn Fn(&mut Context)>,
          interval: Duration,
      },
  }
  ```

- [ ] `cx.set_timeout(duration, callback)` — one-shot timer
- [ ] `cx.set_interval(duration, callback) -> IntervalHandle` — repeating timer
- [ ] `IntervalHandle::cancel()` — stop a repeating timer

- [ ] Event loop integration:
  - Before blocking for events, compute `min(next_timer_deadline - now, max_wait)`
  - Use platform-specific timed wait (e.g., `[NSApp nextEventMatchingMask:untilDate:]`)
  - After waking: fire all expired timers

---

### 5.4 — Custom Title Bar (Week 19, Days 1-3)

**Tasks:**

- [ ] `mozui-elements/src/title_bar.rs`:
  ```rust
  pub fn default_title_bar(cx: &mut Context) -> impl Element { ... }
  ```
  - Height: 38px (matches macOS standard)
  - Background from `cx.theme().title_bar_background`
  - Title text centered
  - Platform-aware window controls placement

- [ ] `.drag_region()` builder method:
  - Marks an element as a window drag area
  - On mouse down in a drag region: call `platform_window.begin_drag_move()`
  - Platform implementation:
    - macOS: `[window performWindowDragWithEvent:event]` (macOS 10.11+)
    - Windows: `PostMessage(hwnd, WM_NCLBUTTONDOWN, HTCAPTION, ...)` (Phase 6)

---

### 5.5 — Window Control Buttons (Week 19, Days 3-5)

**Tasks:**

- [ ] macOS traffic lights (close, minimize, zoom):
  - Option A: Embed native `NSWindow` standard buttons, repositioned
    - `window.standardWindowButton(.closeButton)` — get the button, reposition it
    - Simpler, native hover/click behavior for free
  - Option B: Fully custom-drawn
    - Draw colored circles (red/yellow/green) with hover icons (x, -, +)
    - Handle click: `window.close()`, `window.miniaturize(nil)`, `window.zoom(nil)`
  - **Recommendation**: Option A for macOS (less work, native feel), custom for Windows/Linux

- [ ] Close button: `cx.window().close()` on click
- [ ] Minimize button: `cx.window().minimize()` on click
- [ ] Maximize button: `cx.window().maximize()` / restore toggle on click

---

### 5.6 — Resize Handles (Week 19-20)

**Tasks:**

- [ ] Invisible resize handles around window edges:
  ```rust
  fn resize_handles(cx: &mut Context) -> impl Element {
      let border = 8.0;  // 8px hit zone

      div()
          .absolute().inset(0.0)
          // Top edge
          .child(div().absolute().top(0.0).left(border).right(border).h(border)
              .cursor(CursorStyle::ResizeNS)
              .on_mouse_down(|_, cx| cx.window().begin_resize(ResizeEdge::Top)))
          // ... Bottom, Left, Right, and 4 corners
  }
  ```

- [ ] Platform resize:
  - macOS: No built-in API for edge-specific resize from custom chrome. Workaround: track mouse drag delta, call `window.setFrame()` each frame. Alternatively, use `NSWindowStyleMask::Resizable` and let the system handle resize from the very edge.
  - Windows: handled via `WM_NCHITTEST` returning `HTTOP`, `HTLEFT`, etc. (Phase 6)

---

### 5.7 — Clipboard & Cursors (Week 20)

**Tasks:**

- [ ] Clipboard (macOS):
  - `clipboard_read()`: `NSPasteboard.generalPasteboard.stringForType(.string)`
  - `clipboard_write(text)`: `NSPasteboard.generalPasteboard.setString(text, forType: .string)`

- [ ] Cursor management:
  - Track desired cursor from hovered element's `.cursor()` setting
  - macOS: `NSCursor.pointingHandCursor().set()`, `NSCursor.iBeamCursor().set()`, etc.
  - Reset to arrow when no element specifies a cursor

- [ ] `examples/async_fetch.rs` — Phase 5 milestone

**Phase 5 complete checklist:**
- [ ] `cx.spawn()` runs async tasks on the main thread
- [ ] `cx.background()` runs tasks on background threads
- [ ] Background results update UI correctly
- [ ] Timers fire at the right time
- [ ] Custom title bar renders with correct styling
- [ ] Window can be dragged by the title bar
- [ ] Window controls (close/minimize/maximize) work
- [ ] Window can be resized from edges/corners
- [ ] Clipboard read/write works
- [ ] Cursor changes on hover (hand for buttons, I-beam for text inputs)

---

## Phase 6: Polish & Cross-Platform (Weeks 21-28)

**Goal**: Ship Windows and Linux support, add visual polish, and flesh out remaining elements.

**Milestone**: The same app runs identically on macOS, Windows, and Linux.

---

### 6.1 — Windows Platform Shell (Weeks 21-23)

**Dependencies:** `windows` crate (windows-rs)

**Tasks:**

- [ ] `mozui-platform/src/windows/app.rs` — `WindowsPlatform`:
  - Register window class via `RegisterClassExW`
  - Message pump: `GetMessage` / `TranslateMessage` / `DispatchMessage`
  - DPI awareness: `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`

- [ ] `mozui-platform/src/windows/window.rs` — `WindowsWindow`:
  - `CreateWindowExW` with `WS_POPUP | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU`
  - `WM_NCCALCSIZE` → return 0 (remove default chrome)
  - `WM_NCHITTEST` → custom hit testing for resize borders and title bar
  - Handle Windows Snap Layouts: when hovering maximize button, return `HTMAXBUTTON`
  - Surface creation via `wgpu` DXGI/Vulkan backend

- [ ] `mozui-platform/src/windows/event.rs` — event translation:
  - `WM_MOUSEMOVE`, `WM_LBUTTONDOWN`, `WM_LBUTTONUP`, etc. → `PlatformEvent`
  - `WM_KEYDOWN`, `WM_KEYUP`, `WM_CHAR` → `PlatformEvent::KeyDown` with proper key mapping
  - `WM_SIZE` → `PlatformEvent::WindowResize`
  - `WM_SETFOCUS` / `WM_KILLFOCUS` → `PlatformEvent::WindowFocused` / `WindowBlurred`
  - `WM_DPICHANGED` → `PlatformEvent::ScaleFactorChanged`
  - Virtual key code → `Key` enum mapping
  - IME: `WM_IME_COMPOSITION` for international text input

- [ ] Windows-specific title bar:
  - Window controls on the right side (minimize, maximize, close)
  - Match Windows 11 styling (rounded corners if supported, snap layout hover)

- [ ] Clipboard: `OpenClipboard` / `GetClipboardData` / `SetClipboardData`
- [ ] Cursors: `SetCursor` with `LoadCursor` for standard cursors

---

### 6.2 — Linux Platform Shell (Weeks 23-26)

**Dependencies:** `wayland-client`, `wayland-protocols`, `x11rb`, `xkbcommon`

**Tasks:**

- [ ] `mozui-platform/src/linux/mod.rs` — auto-detect Wayland vs X11:
  ```rust
  pub fn create_linux_platform() -> Box<dyn Platform> {
      if std::env::var("WAYLAND_DISPLAY").is_ok() {
          Box::new(WaylandPlatform::new())
      } else {
          Box::new(X11Platform::new())
      }
  }
  ```

- [ ] **Wayland** (`mozui-platform/src/linux/wayland/`):
  - Connect to `wl_display`
  - Create `wl_surface` + `xdg_surface` + `xdg_toplevel`
  - Client-side decorations (CSD) — mozui already draws its own, so this is natural
  - Input: `wl_keyboard` (with `xkbcommon` for keymap), `wl_pointer`
  - HiDPI: `wp_fractional_scale_v1` protocol
  - Surface for wgpu via `wl_surface`
  - Event loop: `wl_display.dispatch()` in a loop with timer integration

- [ ] **X11** (`mozui-platform/src/linux/x11/`):
  - Connect to X server via `x11rb`
  - Create window with `create_window`
  - Remove decorations via `_MOTIF_WM_HINTS`
  - Input: X11 events (`ButtonPress`, `KeyPress`, `MotionNotify`)
  - Keyboard: XKB via `xkbcommon`
  - `_NET_WM_STATE` for maximize/minimize
  - Surface for wgpu via X11 window
  - Event loop: `x11rb` event polling with timer integration

- [ ] Linux-specific:
  - Clipboard: Wayland `wl_data_device` / X11 selections (`XA_CLIPBOARD`)
  - Cursors: `xcursor` / `wayland-cursor`
  - File dialogs: DBus portal (`org.freedesktop.portal.FileChooser`) — deferred to Phase 7

---

### 6.3 — Renderer Polish (Weeks 25-26)

**Tasks:**

- [ ] **Shadows**:
  - Extend rect shader to support box shadows
  - Shadow is a larger SDF rect drawn behind the element with a Gaussian blur approximation
  - Multiple shadow layers for realistic depth
  - Shadow parameters: `offset_x`, `offset_y`, `blur`, `spread`, `color`

- [ ] **Gradients**:
  - Linear gradient support in the rect fragment shader
  - Evaluate gradient based on angle and position within the rect
  - Gradient stops array passed via uniform buffer

- [ ] **Images**:
  - `DrawCommand::Image` — textured quad with corner rounding
  - Image loading: use `image` crate to decode PNG/JPEG/WebP
  - Images stored in the texture atlas (large images get their own texture)
  - `image()` element with `.src()`, `.rounded()`, `.opacity()` builder methods

- [ ] **SVG** (basic):
  - Use `resvg` crate to rasterize SVGs to bitmaps
  - Treat as images after rasterization
  - Cache rasterized SVGs at current display scale

---

### 6.4 — Scroll Containers (Week 26-27)

**Tasks:**

- [ ] `mozui-elements/src/scroll_view.rs`:
  ```rust
  pub fn scroll_view() -> ScrollView { ... }
  ```
  - Tracks scroll offset (`scroll_x`, `scroll_y`) as internal signals
  - Content is laid out at full size, then clipped to the scroll view bounds
  - `DrawCommand::PushClip` / `DrawCommand::PopClip` for clipping
  - Scroll bar rendering:
    - Overlay style (semi-transparent, appears on scroll, fades out after 1s)
    - Thumb size proportional to visible/total content ratio
    - Click and drag to scroll

- [ ] Scroll physics:
  - macOS: momentum scrolling from `ScrollDelta::Pixels` (trackpad)
  - Discrete scrolling from `ScrollDelta::Lines` (mouse wheel) — multiply by line height
  - Smooth interpolation for visual smoothness

- [ ] `.overflow_y_scroll()` on `Div` — shorthand for wrapping children in a scroll view

---

### 6.5 — Derived Signals & Effects (Week 27)

**Tasks:**

- [ ] `cx.use_memo()`:
  ```rust
  pub fn use_memo<T: 'static + PartialEq>(
      &mut self,
      f: impl Fn(&Context) -> T + 'static
  ) -> Signal<T>
  ```
  - Creates a derived signal that recomputes when its dependencies change
  - Caches the value — only re-runs the closure if a dependency signal changed
  - Compares new value with old via `PartialEq` — if equal, don't notify subscribers

- [ ] `cx.use_effect()`:
  ```rust
  pub fn use_effect(&mut self, f: impl Fn(&mut Context) + 'static)
  ```
  - Runs the closure whenever its dependency signals change
  - Does NOT run during initial render — only on subsequent changes
  - Used for side effects (logging, persistence, network calls)

---

### 6.6 — Multi-Window Support (Week 27-28)

**Tasks:**

- [ ] `cx.open_window()` — create a second window with its own element tree
- [ ] Each window has its own: surface, element tree, layout tree, focus manager
- [ ] Shared across windows: arena (signals), executor, keybindings, theme
- [ ] Closing the last window exits the app (configurable)
- [ ] Window-specific context: `cx.window()` returns the current window's handle

---

### 6.7 — Light Theme & Examples (Week 28)

**Tasks:**

- [ ] Complete `Theme::light()` with tested color palette
- [ ] `examples/counter.rs` — simple counter (already exists from Phase 3, polish it)
- [ ] `examples/todo_app.rs` — full todo app as shown in IMPLEMENTATION.md
- [ ] `examples/custom_theme.rs` — demonstrate theming
- [ ] `examples/multi_window.rs` — demonstrate multi-window
- [ ] Verify all examples run on macOS, Windows, and Linux

**Phase 6 complete checklist:**
- [ ] Windows platform shell works (borderless window, events, rendering)
- [ ] Linux Wayland shell works
- [ ] Linux X11 shell works
- [ ] Shadows render correctly
- [ ] Gradients render correctly
- [ ] Images load and display
- [ ] Scroll containers scroll smoothly
- [ ] `use_memo` and `use_effect` work correctly
- [ ] Multiple windows can be open simultaneously
- [ ] All examples run on all three platforms with identical appearance
- [ ] No platform-specific visual bugs

---

## Phase 7: Ecosystem & DX (Weeks 29+)

**Goal**: Transform mozui from a working library into a delightful developer experience.

**This phase is ongoing and not strictly timeboxed.**

---

### 7.1 — Documentation

**Tasks:**

- [ ] Rustdoc for all public APIs (`///` doc comments with examples)
- [ ] `docs/` directory with guides:
  - Getting Started (10-minute tutorial)
  - Core Concepts (signals, elements, layout, events)
  - Styling Guide (theme customization, inline styles)
  - Keyboard Shortcuts (action system guide)
  - Async Patterns (spawning tasks, loading states)
  - Platform Notes (per-platform quirks and capabilities)
- [ ] README.md with:
  - Feature overview
  - Quick start example
  - Comparison with alternatives
  - Platform support matrix
  - Contributing guide link

---

### 7.2 — Devtools

**Tasks:**

- [ ] **Element Inspector**:
  - Toggle with `F12` or `Cmd+Shift+I`
  - Overlay that highlights hovered elements with their bounds
  - Side panel showing: element type, computed layout (x, y, width, height), style properties, signal values
  - Implemented as mozui elements themselves (dogfooding!)

- [ ] **Signal Debugger**:
  - Shows all active signals with current values
  - Highlights signals that changed in the last frame
  - Shows subscriber count for each signal
  - Shows dependency graph (which views depend on which signals)

- [ ] **Performance Overlay**:
  - Frame time graph
  - Layout time, render time, paint time breakdown
  - Draw call count
  - Element count
  - Toggle with `F11`

---

### 7.3 — Hot Reload Exploration

**Tasks:**

- [ ] Research approach:
  - Option A: `cargo-watch` style rebuild + re-launch (simple but loses state)
  - Option B: Dynamic library loading — compile components as `cdylib`, hot-swap. Complex but preserves state.
  - Option C: Interpret a subset of Rust expressions for style changes only
- [ ] Prototype the most viable option
- [ ] Document limitations and trade-offs

---

### 7.4 — Component Library

Build a set of commonly needed components that ship with mozui:

**Tasks:**

- [ ] `checkbox()` — toggle with checkmark icon
- [ ] `radio_button()` — mutually exclusive selection
- [ ] `switch()` — iOS/Android style toggle
- [ ] `slider(min, max)` — draggable range input
- [ ] `select()` / `dropdown()` — dropdown menu selection
- [ ] `tooltip()` — hover tooltip
- [ ] `modal()` — overlay dialog with focus trap
- [ ] `context_menu()` — right-click menu
- [ ] `tabs()` — tab bar with content panels
- [ ] `progress_bar()` — determinate/indeterminate
- [ ] `spinner()` — loading indicator
- [ ] `avatar()` — circular image with fallback initials
- [ ] `badge()` — small label/counter
- [ ] `divider()` / `separator()` — horizontal/vertical line
- [ ] `toast()` / `notification()` — transient messages

Each component should:
- Follow the builder pattern
- Use the theme for default styling
- Be fully keyboard accessible
- Include Rustdoc with usage examples

---

### 7.5 — Accessibility (accesskit Integration)

**Tasks:**

- [ ] Add `accesskit` and `accesskit_winit` (or custom platform adapters) as dependencies
- [ ] Build an accessibility tree from the element tree:
  - `role` and `label` fields (already on elements) map directly to `accesskit::Node`
  - Focus state maps to `accesskit::NodeState::focused`
  - Actions map to `accesskit::Action`

- [ ] Platform bridges:
  - macOS: `accesskit_macos` — NSAccessibility protocol
  - Windows: `accesskit_windows` — UI Automation provider
  - Linux: `accesskit_unix` — AT-SPI2 via DBus

- [ ] Testing:
  - macOS: VoiceOver
  - Windows: NVDA / Narrator
  - Linux: Orca

---

### 7.6 — WASM Target

**Tasks:**

- [ ] **Platform shell for web**:
  - Canvas element as the rendering surface
  - wgpu WebGPU backend (or fall back to WebGL)
  - JavaScript event listeners for mouse/keyboard → `PlatformEvent`
  - No custom window chrome (browser handles this)
  - `window.requestAnimationFrame` for the render loop

- [ ] **Build tooling**:
  - `wasm-pack` or `trunk` for building
  - Example: build a mozui app as a web page

- [ ] **Constraints**:
  - No filesystem access (font loading via web fonts or embedded fonts)
  - No threads (`cx.background()` falls back to main thread or web workers)
  - Binary size optimization (wasm-opt, LTO, codegen-units=1)

- [ ] **font-kit on WASM**:
  - font-kit doesn't work on WASM (no system fonts)
  - Need a WASM-specific font strategy: embedded fonts or a pure-Rust shaping/rasterization path (swap to `cosmic-text` + `swash` on WASM only)
  - Feature flag: `#[cfg(target_arch = "wasm32")]`

---

### 7.7 — Performance Profiling & Optimization

**Tasks:**

- [ ] Add `tracing` spans to all hot paths (layout, render, paint, hit testing)
- [ ] Profile with `cargo-flamegraph` and `tracing-chrome`
- [ ] Optimize based on real data:
  - Batching efficiency — minimize draw calls
  - Layout caching — avoid unnecessary Taffy recomputation
  - Signal notification — avoid cascading re-renders
  - Memory — arena fragmentation, element allocation
- [ ] Benchmark suite using `criterion`:
  - Layout: 100 / 1000 / 10000 element trees
  - Rendering: draw call count vs element count
  - Signal: propagation latency with deep dependency chains
  - Hit testing: 1000 overlapping elements
- [ ] Damage tracking (optional):
  - Track dirty screen regions
  - Only repaint damaged areas
  - Use scissor rects to limit GPU work

---

### 7.8 — Animations (Future)

Not in scope for initial phases but worth designing for:

- **Transition system**: Animate between style states over time
  ```rust
  div()
      .bg(Color::RED)
      .transition(Property::Background, cx.theme().transition_normal)
      .hover(|s| s.bg(Color::BLUE))  // Smoothly transitions over 200ms
  ```
- **Spring-based animations**: More natural feel for interactive elements
- **Animation runtime**: Integrate with the event loop timer system

---

## Appendix: Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| `objc2` API instability | Medium | Pin dependency versions, wrap in thin abstraction layer |
| macOS custom chrome edge cases | High | Test on multiple macOS versions (13, 14, 15), handle edge cases incrementally |
| font-kit doesn't cover all platforms well | Medium | Abstract behind a `FontBackend` trait, can swap implementations |
| Taffy layout bugs | Low | Taffy is well-tested, but complex Flexbox cases may hit bugs. Contribute fixes upstream. |
| wgpu version churn | Medium | Pin wgpu version, upgrade deliberately between phases |
| WASM binary size | Medium | Use `wasm-opt`, enable LTO, audit dependency tree |
| Text input complexity (IME, BiDi, selection) | High | Keep text input simple initially, iterate. IME is the hardest part. |
| Performance with large element trees | Medium | Profile early (Phase 3), optimize in Phase 7. Incremental layout helps. |

---

## Appendix: Development Environment

**Recommended setup:**

- Rust nightly (for some proc-macro features and better error messages) — but ensure stable compatibility
- macOS as primary development platform (Phase 1-5)
- Cross-compilation tested in Phase 6:
  - Windows: cross-compile from macOS via `cargo build --target x86_64-pc-windows-msvc` or use a Windows VM/CI
  - Linux: test in a VM or container with Wayland/X11
- CI: GitHub Actions with macOS, Windows, Linux runners
- `cargo clippy` and `cargo fmt` enforced in CI
