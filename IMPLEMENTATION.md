# mozui Implementation Guide

> A cross-platform, GPU-accelerated GUI library for Rust with a focus on developer experience.

---

## Table of Contents

1. [Design Philosophy](#1-design-philosophy)
2. [Architecture Overview](#2-architecture-overview)
3. [Crate Structure](#3-crate-structure)
4. [Rendering Engine](#4-rendering-engine)
5. [Text Rendering](#5-text-rendering)
6. [Platform Shells](#6-platform-shells)
7. [Custom Window Chrome](#7-custom-window-chrome)
8. [Layout System](#8-layout-system)
9. [Reactivity & State Management](#9-reactivity--state-management)
10. [Component Model](#10-component-model)
11. [Element Tree & Builder Pattern](#11-element-tree--builder-pattern)
12. [Styling & Theming](#12-styling--theming)
13. [Event Handling](#13-event-handling)
14. [Focus System](#14-focus-system)
15. [Action & Keybinding System](#15-action--keybinding-system)
16. [Async Runtime](#16-async-runtime)
17. [Accessibility](#17-accessibility)
18. [Application Lifecycle](#18-application-lifecycle)
19. [Error Handling Strategy](#19-error-handling-strategy)
20. [Testing Strategy](#20-testing-strategy)
21. [Performance Considerations](#21-performance-considerations)
22. [Implementation Phases](#22-implementation-phases)
23. [Dependencies](#23-dependencies)
24. [API Reference Sketches](#24-api-reference-sketches)

---

## 1. Design Philosophy

### Core Principles

**DX-first**: Every API decision is evaluated through the lens of "how does this feel to use?" Developer ergonomics is the top priority, followed by binary size, correctness, and runtime performance — in that order. A slightly slower abstraction that is dramatically easier to use is the right choice for mozui.

**React-inspired, Rust-native**: mozui draws heavy inspiration from React's mental model — function components, local reactive state, declarative UI — but adapts these ideas to Rust's ownership system rather than fighting it. Developers familiar with React should feel at home; developers coming from Rust should find the APIs idiomatic.

**Consistent, not native**: mozui renders its own pixels on every platform. There is no delegation to native widgets. The same UI looks identical on macOS, Windows, and Linux. This is a deliberate choice — consistency and full visual control are prioritized over matching the host OS's native look and feel.

**Full ownership**: mozui owns the entire window, including the title bar, window controls, and all chrome. This allows for deeply integrated, custom application designs (like Zed, Figma, or Spotify) where the boundary between "chrome" and "content" is blurred.

**Explicit over implicit**: There is no CSS cascade, no selector specificity, no inherited styles. Styles are inline, applied directly to elements via builder methods. What you write is what you get. This eliminates an entire class of "why does my element look like that?" debugging.

### Non-Goals

- Native widget rendering
- CSS-compatible styling engine
- Embedding inside other GUI frameworks
- Mobile platforms (for now)
- Server-side rendering

### Inspirations

| Project | What we take from it |
|---------|---------------------|
| GPUI (Zed) | Custom platform shells, action system, focus scopes, arena-based ownership, background executor |
| React | Function components, hooks-like signals, declarative UI, component composition |
| SwiftUI | Builder-pattern element construction, inline styling |
| Vello/Linebender | GPU-accelerated 2D rendering approaches |
| Taffy | Flexbox layout engine |

---

## 2. Architecture Overview

### High-Level Data Flow

```
User Input (OS events)
        |
        v
  Platform Shell (macOS / Windows / Linux)
        |
        v
  Event Loop & Async Executor
        |
        v
  Event Dispatch (flat, to target element)
        |
        v
  Signal Mutations (state changes)
        |
        v
  Dirty Tracking (mark affected views)
        |
        v
  Layout (Taffy, incremental — only dirty subtrees)
        |
        v
  Render (function components produce element trees)
        |
        v
  Paint (elements emit draw commands)
        |
        v
  GPU Renderer (wgpu — batched draw calls)
        |
        v
  Present (swap chain, display frame)
```

### Core Subsystems

```
mozui
 ├── app          # Application lifecycle, context, arena
 ├── platform     # OS-specific window/event code
 │   ├── macos
 │   ├── windows
 │   └── linux
 ├── renderer     # wgpu-based 2D rendering pipeline
 ├── text         # Font loading, shaping, rasterization (font-kit)
 ├── layout       # Taffy integration, incremental layout
 ├── reactive     # Signals, effects, subscriptions
 ├── elements     # Built-in element types (div, text, button, etc.)
 ├── style        # Style types, theme struct
 ├── events       # Event types, dispatch, input handling
 ├── focus        # Focus scopes, FocusHandle
 ├── actions      # Named actions, keybinding registry
 ├── executor     # Async runtime, task spawning
 └── a11y         # Accessibility stubs (role, label fields)
```

### Ownership Model

All state lives in a centralized **`AppContext`** arena. Components do not own their data directly — they hold lightweight **signal handles** that point into the arena. This solves Rust's borrow checker challenges with UI state:

```
AppContext (arena)
 ├── Signal<i32> at slot 0  ← SignalHandle { id: 0 }
 ├── Signal<String> at slot 1  ← SignalHandle { id: 1 }
 ├── Signal<Vec<Item>> at slot 2  ← SignalHandle { id: 2 }
 └── ...
```

Signal handles are `Copy + Clone + Send` (just an ID). Reading or writing a signal goes through the `Context`, which manages borrow checking at runtime (similar to `RefCell`, but scoped to the arena).

---

## 3. Crate Structure

mozui is organized as a Cargo workspace with multiple crates for modularity and compile time optimization:

```
mozui/
 ├── Cargo.toml              # Workspace root
 ├── crates/
 │   ├── mozui/              # Main facade crate (re-exports everything)
 │   │   └── src/lib.rs
 │   ├── mozui-app/          # AppContext, lifecycle, arena
 │   │   └── src/lib.rs
 │   ├── mozui-platform/     # Platform shell trait + implementations
 │   │   └── src/
 │   │       ├── lib.rs
 │   │       ├── traits.rs   # Platform, Window, EventLoop traits
 │   │       ├── macos/
 │   │       │   ├── mod.rs
 │   │       │   ├── app.rs
 │   │       │   ├── window.rs
 │   │       │   └── event.rs
 │   │       ├── windows/
 │   │       │   └── ...
 │   │       └── linux/
 │   │           └── ...
 │   ├── mozui-renderer/     # wgpu rendering pipeline
 │   │   └── src/lib.rs
 │   ├── mozui-text/         # Text shaping, layout, rasterization
 │   │   └── src/lib.rs
 │   ├── mozui-layout/       # Taffy integration
 │   │   └── src/lib.rs
 │   ├── mozui-reactive/     # Signals, effects, subscriptions
 │   │   └── src/lib.rs
 │   ├── mozui-elements/     # Built-in elements
 │   │   └── src/lib.rs
 │   ├── mozui-style/        # Style types, theme
 │   │   └── src/lib.rs
 │   ├── mozui-events/       # Event types and dispatch
 │   │   └── src/lib.rs
 │   └── mozui-executor/     # Async runtime
 │       └── src/lib.rs
 └── examples/
     ├── hello.rs
     ├── counter.rs
     ├── todo_app.rs
     └── custom_theme.rs
```

### Why a Workspace?

- **Parallel compilation**: Independent crates compile concurrently
- **Incremental builds**: Changing `mozui-style` doesn't recompile `mozui-platform`
- **Optional features**: Users can depend on subcrates directly if they want fine-grained control
- **Binary size**: Dead code elimination works better across crate boundaries

The top-level `mozui` crate re-exports everything with a flat namespace:

```rust
// Users write:
use mozui::{App, div, text, button, Context};

// Not:
use mozui_app::App;
use mozui_elements::{div, text, button};
use mozui_reactive::Context;
```

---

## 4. Rendering Engine

### Overview

mozui uses `wgpu` to render all UI elements on the GPU. The renderer is a custom 2D rendering pipeline optimized for UI workloads: rectangles, rounded rectangles, borders, shadows, text glyphs, and images.

### Render Pipeline Architecture

```
Element Tree
     |
     v
Paint Phase (elements emit DrawCommands)
     |
     v
DrawCommand List (sorted by z-order)
     |
     v
Batching (group by texture/shader/blend mode)
     |
     v
GPU Upload (vertex buffers, uniform buffers, texture atlases)
     |
     v
wgpu Render Pass (draw calls)
     |
     v
Surface Present
```

### Draw Command Types

```rust
enum DrawCommand {
    /// Filled rectangle, optionally with rounded corners
    Rect {
        bounds: Rect,
        background: Fill,       // Solid color, linear gradient, radial gradient
        corner_radii: Corners,  // Per-corner radius
        border: Option<Border>,
        shadow: Option<Shadow>,
    },

    /// Text run (pre-shaped glyphs)
    Text {
        glyphs: Vec<PositionedGlyph>,
        color: Color,
        clip: Rect,
    },

    /// Raster image
    Image {
        texture_id: TextureId,
        bounds: Rect,
        corner_radii: Corners,
        opacity: f32,
    },

    /// Clip region push/pop
    PushClip { bounds: Rect, corner_radii: Corners },
    PopClip,

    /// Opacity layer
    PushOpacity { opacity: f32 },
    PopOpacity,
}
```

### Shader Strategy

Two primary shaders handle the majority of rendering:

1. **Rect Shader**: Renders filled/stroked rounded rectangles with optional gradients, borders, and shadows. Uses signed distance fields (SDF) for anti-aliased edges — a single quad per rectangle, all corner rounding and border rendering happens in the fragment shader.

2. **Glyph Shader**: Renders text glyphs from a texture atlas. Supports both grayscale and subpixel (LCD) anti-aliasing. Alpha-tested for crisp rendering at all sizes.

```wgsl
// Simplified rect fragment shader concept
@fragment
fn rect_frag(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = rounded_rect_sdf(in.local_pos, in.size, in.corner_radii);

    // Anti-aliased edge
    let alpha = 1.0 - smoothstep(-0.5, 0.5, dist);

    // Border (if present)
    let border_alpha = smoothstep(-0.5, 0.5, dist + in.border_width)
                     - smoothstep(-0.5, 0.5, dist);

    let fill_color = evaluate_fill(in.fill_type, in.local_pos, in.size);
    let final_color = mix(fill_color, in.border_color, border_alpha);

    return vec4(final_color.rgb, final_color.a * alpha);
}
```

### Texture Atlas

A dynamic texture atlas manages glyph bitmaps and small images:

- **Atlas allocation**: Shelf-packing algorithm (simple, fast, good enough for UI)
- **Eviction**: LRU-based when the atlas is full
- **Size**: Start at 1024x1024, grow to 4096x4096 as needed
- **Format**: RGBA8 for color emoji and images, R8 for grayscale glyphs

```rust
struct TextureAtlas {
    texture: wgpu::Texture,
    allocator: ShelfAllocator,
    entries: HashMap<AtlasKey, AtlasRegion>,
    lru: LruTracker,
}

enum AtlasKey {
    Glyph { font_id: FontId, glyph_id: GlyphId, size_px: u16 },
    Image { image_id: ImageId },
}
```

### Frame Rendering

Each frame follows this sequence:

1. **Collect draw commands**: Walk the element tree, each element appends to a `DrawList`
2. **Sort by z-order**: Stable sort preserving tree order within the same z-level
3. **Batch**: Group consecutive commands with the same shader/texture/blend state
4. **Upload**: Write vertex data and uniforms to GPU buffers
5. **Record render pass**: One `wgpu::RenderPass` with multiple draw calls
6. **Present**: Submit command buffer, present surface

Target: **60fps for typical UIs** (< 16ms per frame). Most frames should be well under this with incremental layout and dirty-region rendering.

### Damage Tracking (Future Optimization)

Initially, the entire window is repainted each frame. As an optimization, damage tracking can be added:

- Track which screen regions are affected by dirty elements
- Only repaint damaged regions
- Use scissor rects to limit GPU work

This is not required for v0.1 but should be designed for.

---

## 5. Text Rendering

### Stack

```
font-kit (font loading, discovery, platform-native access)
     |
     v
HarfBuzz (via font-kit's shaping, or rustybuzz as pure-Rust alternative)
     |
     v
Glyph rasterization (font-kit's native rasterizers)
     |
     v
Texture atlas (glyph bitmaps uploaded to GPU)
     |
     v
Glyph shader (renders quads with atlas UVs)
```

`font-kit` wraps platform-native text APIs:
- **macOS**: Core Text
- **Windows**: DirectWrite
- **Linux**: FreeType + Fontconfig

This gives us high-quality, platform-tuned glyph rasterization while mozui handles the layout and rendering to achieve consistent *positioning* cross-platform.

### Text Layout Pipeline

```rust
struct TextLayout {
    /// Input
    text: String,
    style: TextStyle,
    max_width: Option<f32>,

    /// Output (computed)
    lines: Vec<TextLine>,
    size: Size,
}

struct TextLine {
    glyphs: Vec<PositionedGlyph>,
    baseline_y: f32,
    width: f32,
}

struct PositionedGlyph {
    glyph_id: GlyphId,
    x: f32,
    y: f32,
    font_id: FontId,
}

struct TextStyle {
    font_family: FontFamily,
    font_size: f32,           // In logical pixels
    font_weight: FontWeight,
    font_style: FontStyle,    // Normal, Italic
    line_height: LineHeight,  // Absolute or relative
    letter_spacing: f32,
    color: Color,
}
```

### Text Layout Steps

1. **Font resolution**: Given a `FontFamily` + weight + style, find the best matching font file via `font-kit`'s font matching
2. **Itemization**: Split text into runs of uniform font/script/direction (handles fallback fonts for emoji, CJK, etc.)
3. **Shaping**: Pass each run through HarfBuzz (or the platform shaper) to get positioned glyphs with proper kerning and ligatures
4. **Line breaking**: Given a max width, break shaped runs into lines using Unicode line break algorithm (UAX #14)
5. **Alignment**: Apply text alignment (left, center, right) to each line
6. **Cache**: Cache shaped runs keyed by `(text, style, max_width)` — invalidate when any input changes

### Font Fallback

When a glyph is missing from the primary font, mozui performs automatic fallback:

1. Check the user-specified font family
2. Try system fallback fonts (via `font-kit`'s system font matching)
3. For emoji: use the system emoji font (Apple Color Emoji, Segoe UI Emoji, Noto Color Emoji)
4. Last resort: render a replacement character (U+FFFD)

### Glyph Cache

Rasterized glyphs are cached in the texture atlas:

```rust
struct GlyphCache {
    atlas: TextureAtlas,
    entries: HashMap<GlyphCacheKey, GlyphCacheEntry>,
}

struct GlyphCacheKey {
    font_id: FontId,
    glyph_id: GlyphId,
    size_px: u16,        // Quantized to avoid excessive cache entries
    subpixel_offset: u8, // Quantized to 4 positions (0, 0.25, 0.5, 0.75)
}

struct GlyphCacheEntry {
    atlas_region: AtlasRegion,
    bearing_x: f32,
    bearing_y: f32,
    advance: f32,
}
```

Subpixel positioning is quantized to 4 horizontal positions to balance quality against cache size.

---

## 6. Platform Shells

### Overview

mozui implements custom platform shells rather than using `winit`. Each platform shell provides:

- Window creation and management (borderless, no native decorations)
- Event loop integration
- Raw input events (mouse, keyboard, touch, scroll)
- Clipboard access
- Cursor management
- Display/monitor enumeration
- DPI/scale factor handling
- File dialogs (optional)
- Drag and drop (optional)

### Platform Trait

```rust
trait Platform: 'static {
    fn new() -> Self;
    fn run(&self, app: AppContext);

    fn screens(&self) -> Vec<Screen>;
    fn primary_screen(&self) -> Screen;

    fn open_window(&self, options: WindowOptions) -> WindowHandle;
    fn close_window(&self, handle: WindowHandle);

    fn set_cursor(&self, cursor: CursorStyle);
    fn clipboard_read(&self) -> Option<String>;
    fn clipboard_write(&self, text: &str);

    fn open_url(&self, url: &str);
}

trait PlatformWindow: 'static {
    fn bounds(&self) -> Rect;
    fn set_bounds(&self, bounds: Rect);
    fn scale_factor(&self) -> f32;
    fn is_focused(&self) -> bool;
    fn set_title(&self, title: &str);
    fn minimize(&self);
    fn maximize(&self);
    fn close(&self);
    fn request_redraw(&self);

    fn wgpu_surface(&self) -> &wgpu::Surface;
}

struct WindowOptions {
    title: String,
    size: Size,
    min_size: Option<Size>,
    max_size: Option<Size>,
    position: Option<Point>,
    resizable: bool,
    visible: bool,
    transparent: bool,
}

struct Screen {
    bounds: Rect,
    work_area: Rect,  // Excluding taskbar/dock
    scale_factor: f32,
}
```

### macOS Implementation

Uses Objective-C runtime bindings (`objc2` crate) to interact with AppKit:

- **`NSApplication`**: Application lifecycle, event loop
- **`NSWindow`**: Borderless window (`NSWindowStyleMask::borderless`)
- **`NSView`**: Custom `CAMetalLayer`-backed view for wgpu rendering
- **`NSEvent`**: Mouse, keyboard, scroll events
- **`NSPasteboard`**: Clipboard
- **`NSCursor`**: Cursor management

```rust
// Simplified macOS window creation
fn open_window(options: WindowOptions) -> WindowHandle {
    unsafe {
        let window = NSWindow::alloc()
            .initWithContentRect_styleMask_backing_defer(
                rect_to_ns(options.size),
                NSWindowStyleMask::Borderless
                    | NSWindowStyleMask::Resizable
                    | NSWindowStyleMask::Miniaturizable,
                NSBackingStoreType::Buffered,
                false,
            );

        window.setTitlebarAppearsTransparent(true);
        window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        window.setMovableByWindowBackground(false);

        let view = MozuiView::new(/* ... */);
        window.setContentView(Some(&view));
        window.makeKeyAndOrderFront(None);

        // Create wgpu surface from the view's CAMetalLayer
        let surface = create_wgpu_surface(&view);

        WindowHandle::new(window, surface)
    }
}
```

Key macOS details:
- Use `NSTrackingArea` for mouse move/enter/exit events
- Handle `windowDidResize`, `windowDidMove`, `windowDidBecomeKey`/`windowDidResignKey` via delegate
- For the traffic light buttons: either embed real `NSButton` controls repositioned, or draw custom ones and handle hit-testing manually
- Retina support: `convertPointToBacking` / `backingScaleFactor`

### Windows Implementation

Uses the Win32 API via `windows-rs`:

- **`CreateWindowExW`**: Borderless window (`WS_POPUP | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX`)
- **`WM_NCCALCSIZE`**: Override to remove default chrome
- **`WM_NCHITTEST`**: Handle custom resize borders and title bar drag areas
- **Direct3D/DXGI**: Surface for wgpu
- **`WM_KEYDOWN`/`WM_CHAR`**: Keyboard input
- **`WM_MOUSEMOVE`/`WM_LBUTTONDOWN`**: Mouse input

```rust
// Simplified Windows message handling for custom chrome
fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCCALCSIZE => {
            // Return 0 to remove all non-client area (title bar, borders)
            LRESULT(0)
        }
        WM_NCHITTEST => {
            let point = point_from_lparam(lparam);
            let window_rect = get_window_rect(hwnd);

            // Check resize borders (8px hit zones)
            if let Some(border) = hit_test_borders(point, window_rect, 8) {
                return border; // HTTOPLEFT, HTTOP, etc.
            }

            // Check custom title bar drag area
            if hit_test_title_bar(point) {
                return HTCAPTION;
            }

            HTCLIENT
        }
        // ... other messages
    }
}
```

Key Windows details:
- Handle DPI awareness via `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`
- Snap layouts: respond to `WM_NCHITTEST` with `HTMAXBUTTON` for the maximize hover zone
- High contrast mode detection for accessibility
- IME support for international input

### Linux Implementation

Supports both Wayland (primary) and X11 (fallback):

**Wayland** (via `wayland-client`):
- `wl_surface` + `xdg_toplevel` for window management
- Client-side decorations (CSD) — Wayland expects this by default, which aligns perfectly with mozui's custom chrome approach
- `wl_keyboard`, `wl_pointer` for input
- `wp_fractional_scale_v1` for HiDPI

**X11** (via `x11rb`):
- Borderless via `_MOTIF_WM_HINTS` or override-redirect
- `_NET_WM_STATE` for maximize/minimize
- XKB for keyboard
- `XRandR` for multi-monitor

The platform shell auto-detects Wayland vs X11 at startup:

```rust
fn create_linux_platform() -> Box<dyn Platform> {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        Box::new(WaylandPlatform::new())
    } else {
        Box::new(X11Platform::new())
    }
}
```

### Event Abstraction

Platform-specific events are converted to a unified event type:

```rust
enum PlatformEvent {
    MouseMove { position: Point, modifiers: Modifiers },
    MouseDown { button: MouseButton, position: Point, click_count: u32, modifiers: Modifiers },
    MouseUp { button: MouseButton, position: Point, modifiers: Modifiers },
    ScrollWheel { delta: ScrollDelta, position: Point, modifiers: Modifiers },
    KeyDown { key: Key, modifiers: Modifiers, is_repeat: bool, ime_text: Option<String> },
    KeyUp { key: Key, modifiers: Modifiers },
    WindowResize { size: Size },
    WindowMove { position: Point },
    WindowFocused,
    WindowBlurred,
    WindowCloseRequested,
    ScaleFactorChanged { scale: f32 },
    FileDrop { paths: Vec<PathBuf> },
}

enum ScrollDelta {
    Lines(f32, f32),
    Pixels(f32, f32),
}
```

---

## 7. Custom Window Chrome

### Title Bar

mozui draws its own title bar as part of the element tree. The default title bar component provides:

- Application title text
- Window control buttons (close, minimize, maximize/restore)
- Draggable area (the title bar itself)
- Platform-appropriate button placement (left on macOS, right on Windows/Linux)

```rust
fn default_title_bar(cx: &mut Context) -> impl Element {
    let platform = cx.platform();

    div()
        .h(TITLE_BAR_HEIGHT)                // 38px default (macOS-like)
        .w_full()
        .bg(cx.theme().title_bar_background)
        .drag_region()                       // Makes this area draggable
        .child(
            if platform.is_macos() {
                traffic_lights(cx)           // Close, minimize, maximize — left side
            } else {
                window_controls(cx)          // Minimize, maximize, close — right side
            }
        )
        .child(
            text(cx.window().title())
                .font_size(13.0)
                .color(cx.theme().title_bar_text)
                .center()
        )
}
```

### Drag Regions

Elements can be marked as drag regions. When the user clicks and drags on a drag region, mozui initiates a window move:

```rust
// On the element:
div().drag_region()

// In the event handler (internal):
if event.is_mouse_down() && element.is_drag_region {
    platform_window.begin_move();
}
```

### Resize Handles

Invisible resize handles are placed around the window edges (8px inset). On hover, the cursor changes to the appropriate resize cursor. On drag, mozui initiates a window resize via the platform API.

```rust
struct ResizeHandles {
    border_width: f32,  // 8.0 default
    corner_size: f32,   // 16.0 default — larger hit zone at corners
}
```

### Window Control Buttons

The built-in window control buttons are regular mozui elements with hover/active states:

```rust
fn close_button(cx: &mut Context) -> impl Element {
    div()
        .size(12.0)
        .rounded_full()
        .bg(cx.theme().close_button)
        .hover(|s| s.bg(cx.theme().close_button_hover))
        .active(|s| s.bg(cx.theme().close_button_active))
        .on_click(move |_, cx| cx.window().close())
}
```

Users can fully replace the title bar by providing their own component — mozui's default title bar is just a convenience.

---

## 8. Layout System

### Taffy Integration

mozui uses [Taffy](https://github.com/DioxusLabs/taffy) for CSS Flexbox layout. Taffy is a pure-Rust layout engine that implements the Flexbox specification.

### Layout Flow

```
Element Tree (mozui elements)
        |
        v
Style Extraction (convert mozui styles to Taffy styles)
        |
        v
Taffy Tree (mirrors the element tree structure)
        |
        v
Taffy::compute_layout()
        |
        v
Layout Results (position + size for each node)
        |
        v
Paint (elements use layout results for rendering)
```

### Incremental Layout

Layout is only recomputed for dirty subtrees:

1. When a signal changes, the dependent view is marked dirty
2. The dirty flag propagates up to the root (ancestors need relayout too)
3. On the next frame, `Taffy::compute_layout()` is called on the root, but Taffy internally skips clean subtrees
4. After layout, dirty flags are cleared

```rust
struct LayoutTree {
    taffy: taffy::TaffyTree,
    nodes: HashMap<ElementId, taffy::NodeId>,
    dirty: HashSet<ElementId>,
}

impl LayoutTree {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
        if let Some(taffy_node) = self.nodes.get(&element_id) {
            self.taffy.mark_dirty(*taffy_node);
        }
    }

    fn compute(&mut self, root: ElementId, available_space: Size) {
        if self.dirty.is_empty() {
            return; // Nothing changed, skip layout entirely
        }

        let root_node = self.nodes[&root];
        self.taffy.compute_layout(root_node, available_space.into());
        self.dirty.clear();
    }
}
```

### Style Mapping

mozui's style builder methods map to Taffy's style properties:

```rust
// mozui                          // Taffy equivalent
.w(200.0)                         // size.width = length(200.0)
.h(100.0)                         // size.height = length(100.0)
.w_full()                         // size.width = percent(1.0)
.padding(16.0)                    // padding = length(16.0) on all sides
.padding_x(8.0)                   // padding.left = padding.right = length(8.0)
.gap(12.0)                        // gap = length(12.0)
.flex()                           // display = Display::Flex
.flex_row()                       // flex_direction = FlexDirection::Row
.flex_col()                       // flex_direction = FlexDirection::Column
.flex_grow(1.0)                   // flex_grow = 1.0
.items_center()                   // align_items = AlignItems::Center
.justify_between()                // justify_content = JustifyContent::SpaceBetween
.absolute()                       // position = Position::Absolute
.top(10.0)                        // inset.top = length(10.0)
.overflow_hidden()                // overflow = Overflow::Hidden
```

### Scrolling

Scroll containers are elements with `overflow_scroll()`. mozui handles:

- Computing scrollable content size vs visible area
- Scroll offset tracking
- Scroll bar rendering (overlay style, auto-hide)
- Scroll physics (momentum scrolling on macOS, smooth scrolling)

```rust
fn scrollable_list(cx: &mut Context, items: &[String]) -> impl Element {
    div()
        .h(400.0)
        .overflow_y_scroll()
        .children(items.iter().map(|item| {
            div().padding(8.0).child(text(item))
        }))
}
```

---

## 9. Reactivity & State Management

### Signals

Signals are the core reactive primitive. A signal holds a value and tracks which views depend on it.

```rust
fn counter(cx: &mut Context) -> impl Element {
    let (count, set_count) = cx.use_signal(0);

    div()
        .child(text(&format!("Count: {}", count.get(cx))))
        .child(
            button("Increment")
                .on_click(move |_, cx| {
                    set_count.set(cx, count.get(cx) + 1);
                })
        )
}
```

### Signal Implementation

```rust
/// Read handle — can read the value and registers dependencies
#[derive(Copy, Clone)]
struct Signal<T> {
    id: SignalId,
    _marker: PhantomData<T>,
}

/// Write handle — can update the value and trigger re-renders
#[derive(Copy, Clone)]
struct SetSignal<T> {
    id: SignalId,
    _marker: PhantomData<T>,
}

impl<T: 'static> Signal<T> {
    /// Read the current value. Automatically registers the calling view
    /// as a subscriber (dependency tracking).
    fn get(&self, cx: &Context) -> &T {
        cx.track_dependency(self.id);
        cx.arena.get(self.id)
    }
}

impl<T: 'static> SetSignal<T> {
    /// Set a new value. Marks all subscribed views as dirty.
    fn set(&self, cx: &mut Context, value: T) {
        cx.arena.set(self.id, value);
        cx.notify_subscribers(self.id);
    }

    /// Update the value using a closure.
    fn update(&self, cx: &mut Context, f: impl FnOnce(&mut T)) {
        cx.arena.update(self.id, f);
        cx.notify_subscribers(self.id);
    }
}
```

### Dependency Tracking

When a view renders, it enters a "tracking scope." Any signal read during this scope is recorded as a dependency:

```rust
struct TrackingScope {
    view_id: ViewId,
    dependencies: Vec<SignalId>,
}

impl Context {
    fn track_dependency(&self, signal_id: SignalId) {
        if let Some(scope) = &self.current_tracking_scope {
            scope.dependencies.push(signal_id);
        }
    }

    fn render_view(&mut self, view_id: ViewId, component: &dyn Fn(&mut Context) -> Element) {
        // Clear old dependencies
        self.clear_subscriptions(view_id);

        // Start tracking
        self.current_tracking_scope = Some(TrackingScope {
            view_id,
            dependencies: vec![],
        });

        // Call the component function — signal reads are tracked
        let element = component(self);

        // Register new dependencies
        let scope = self.current_tracking_scope.take().unwrap();
        for signal_id in scope.dependencies {
            self.subscribe(signal_id, view_id);
        }

        element
    }
}
```

### Derived/Computed Values

For values derived from other signals, use `cx.use_memo()`:

```rust
fn item_list(cx: &mut Context) -> impl Element {
    let (items, _) = cx.use_signal(vec!["apple", "banana", "cherry"]);
    let (filter, set_filter) = cx.use_signal(String::new());

    // Recomputed only when `items` or `filter` changes
    let filtered = cx.use_memo(move |cx| {
        let filter = filter.get(cx);
        items.get(cx)
            .iter()
            .filter(|item| item.contains(filter.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    });

    div()
        .child(text_input().value(filter).on_change(move |val, cx| set_filter.set(cx, val)))
        .child(
            div().children(
                filtered.get(cx).iter().map(|item| text(item))
            )
        )
}
```

### Effects

Side effects that run when dependencies change:

```rust
fn document_editor(cx: &mut Context) -> impl Element {
    let (content, set_content) = cx.use_signal(String::new());
    let (is_saved, set_is_saved) = cx.use_signal(true);

    // Run whenever `content` changes
    cx.use_effect(move |cx| {
        let _content = content.get(cx);
        set_is_saved.set(cx, false);
    });

    // ...
}
```

### Signal Lifecycle

Signals are scoped to the view that created them. When a view is removed from the tree:

1. Its signals are dropped from the arena
2. Its subscriptions are cleaned up
3. Any effects are cancelled

This prevents memory leaks and stale subscriptions.

---

## 10. Component Model

### Function Components

Components are plain Rust functions that take a `&mut Context` and return an element:

```rust
fn greeting(cx: &mut Context) -> impl Element {
    text("Hello, world!")
        .font_size(24.0)
        .color(cx.theme().text_primary)
}
```

### Components with Props

For components that accept configuration, use a struct:

```rust
struct ButtonProps {
    label: String,
    variant: ButtonVariant,
    on_click: Option<Box<dyn Fn(&mut Context)>>,
    disabled: bool,
}

impl Default for ButtonProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            variant: ButtonVariant::Primary,
            on_click: None,
            disabled: false,
        }
    }
}

fn styled_button(cx: &mut Context, props: ButtonProps) -> impl Element {
    let (hovered, set_hovered) = cx.use_signal(false);

    let bg = match props.variant {
        ButtonVariant::Primary => cx.theme().primary,
        ButtonVariant::Secondary => cx.theme().secondary,
    };

    div()
        .padding_x(16.0)
        .padding_y(8.0)
        .rounded(6.0)
        .bg(if *hovered.get(cx) { bg.lighter(0.1) } else { bg })
        .on_mouse_enter(move |_, cx| set_hovered.set(cx, true))
        .on_mouse_leave(move |_, cx| set_hovered.set(cx, false))
        .on_click(move |_, cx| {
            if !props.disabled {
                if let Some(handler) = &props.on_click {
                    handler(cx);
                }
            }
        })
        .child(text(&props.label).color(cx.theme().on_primary))
}
```

### Component Composition

Components compose by nesting:

```rust
fn app(cx: &mut Context) -> impl Element {
    div()
        .flex_col()
        .gap(16.0)
        .padding(24.0)
        .child(header(cx))
        .child(main_content(cx))
        .child(footer(cx))
}

fn header(cx: &mut Context) -> impl Element {
    div()
        .flex_row()
        .items_center()
        .justify_between()
        .child(text("My App").font_size(20.0).bold())
        .child(navigation(cx))
}
```

### Children

Components can accept children for slot-based composition:

```rust
fn card(cx: &mut Context, children: impl IntoElements) -> impl Element {
    div()
        .padding(16.0)
        .rounded(8.0)
        .bg(cx.theme().surface)
        .shadow(cx.theme().shadow_md)
        .children(children)
}

// Usage
fn app(cx: &mut Context) -> impl Element {
    card(cx, (
        text("Card Title").bold(),
        text("Some content here."),
        styled_button(cx, ButtonProps { label: "Action".into(), ..Default::default() }),
    ))
}
```

### View Identity

Each component instance needs a stable identity so the framework can:

- Preserve signal state across re-renders
- Match old and new elements for incremental updates
- Clean up signals when a component is removed

Identity is determined by **position in the tree** (like React) plus an optional **key**:

```rust
// Without key — identity is by position
div().children(items.iter().map(|item| item_row(cx, item)))

// With key — identity is stable even if order changes
div().children(items.iter().map(|item| {
    keyed(item.id, || item_row(cx, item))
}))
```

---

## 11. Element Tree & Builder Pattern

### Element Trait

All UI nodes implement the `Element` trait:

```rust
trait Element: 'static {
    /// Return the Taffy style for layout
    fn layout_style(&self) -> taffy::Style;

    /// Paint this element given its computed layout
    fn paint(&self, bounds: Rect, cx: &mut PaintContext);

    /// Handle an event targeted at this element
    fn handle_event(&mut self, event: &Event, cx: &mut EventContext) -> EventResult;

    /// Children of this element
    fn children(&self) -> &[Box<dyn Element>];

    /// Optional accessibility info
    fn accessibility(&self) -> Option<AccessibilityInfo> {
        None
    }
}
```

### Built-in Elements

```rust
// Container — the fundamental building block
div()

// Text
text("Hello")
text(&format!("Count: {}", count))

// Interactive
button("Click me")
text_input()
checkbox()

// Media
image(image_source)
svg(svg_source)

// Utility
spacer()           // Fills available space (flex_grow: 1)
separator()        // Horizontal or vertical line
scroll_view()      // Scrollable container
```

### Builder Methods

Every element returns a builder that supports the full set of styling/behavior methods. The builder consumes `self` and returns `Self` for chaining:

```rust
struct Div {
    style: Style,
    children: Vec<Box<dyn Element>>,
    event_handlers: EventHandlers,
    focus_handle: Option<FocusHandle>,
    drag_region: bool,
    accessibility: Option<AccessibilityInfo>,
}

impl Div {
    // --- Layout ---
    fn w(mut self, width: f32) -> Self { ... }
    fn h(mut self, height: f32) -> Self { ... }
    fn w_full(mut self) -> Self { ... }
    fn h_full(mut self) -> Self { ... }
    fn size(mut self, size: f32) -> Self { ... }
    fn min_w(mut self, width: f32) -> Self { ... }
    fn max_w(mut self, width: f32) -> Self { ... }
    fn padding(mut self, padding: f32) -> Self { ... }
    fn padding_x(mut self, padding: f32) -> Self { ... }
    fn padding_y(mut self, padding: f32) -> Self { ... }
    fn margin(mut self, margin: f32) -> Self { ... }
    fn gap(mut self, gap: f32) -> Self { ... }

    // --- Flex ---
    fn flex(mut self) -> Self { ... }
    fn flex_row(mut self) -> Self { ... }
    fn flex_col(mut self) -> Self { ... }
    fn flex_grow(mut self, grow: f32) -> Self { ... }
    fn flex_shrink(mut self, shrink: f32) -> Self { ... }
    fn flex_wrap(mut self) -> Self { ... }
    fn items_start(mut self) -> Self { ... }
    fn items_center(mut self) -> Self { ... }
    fn items_end(mut self) -> Self { ... }
    fn justify_start(mut self) -> Self { ... }
    fn justify_center(mut self) -> Self { ... }
    fn justify_between(mut self) -> Self { ... }
    fn justify_end(mut self) -> Self { ... }

    // --- Visual ---
    fn bg(mut self, color: impl Into<Fill>) -> Self { ... }
    fn rounded(mut self, radius: f32) -> Self { ... }
    fn rounded_t(mut self, radius: f32) -> Self { ... }
    fn rounded_b(mut self, radius: f32) -> Self { ... }
    fn rounded_full(mut self) -> Self { ... }
    fn border(mut self, width: f32, color: Color) -> Self { ... }
    fn shadow(mut self, shadow: Shadow) -> Self { ... }
    fn opacity(mut self, opacity: f32) -> Self { ... }

    // --- Overflow ---
    fn overflow_hidden(mut self) -> Self { ... }
    fn overflow_x_scroll(mut self) -> Self { ... }
    fn overflow_y_scroll(mut self) -> Self { ... }

    // --- Positioning ---
    fn absolute(mut self) -> Self { ... }
    fn relative(mut self) -> Self { ... }
    fn top(mut self, v: f32) -> Self { ... }
    fn right(mut self, v: f32) -> Self { ... }
    fn bottom(mut self, v: f32) -> Self { ... }
    fn left(mut self, v: f32) -> Self { ... }
    fn z(mut self, z_index: i32) -> Self { ... }

    // --- Interaction ---
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Context) + 'static) -> Self { ... }
    fn on_mouse_down(mut self, handler: impl Fn(&MouseEvent, &mut Context) + 'static) -> Self { ... }
    fn on_mouse_up(mut self, handler: impl Fn(&MouseEvent, &mut Context) + 'static) -> Self { ... }
    fn on_mouse_enter(mut self, handler: impl Fn(&MouseEvent, &mut Context) + 'static) -> Self { ... }
    fn on_mouse_leave(mut self, handler: impl Fn(&MouseEvent, &mut Context) + 'static) -> Self { ... }
    fn on_scroll(mut self, handler: impl Fn(&ScrollEvent, &mut Context) + 'static) -> Self { ... }
    fn on_key_down(mut self, handler: impl Fn(&KeyEvent, &mut Context) + 'static) -> Self { ... }

    // --- Interactive states ---
    fn hover(mut self, style: impl Fn(Style) -> Style) -> Self { ... }
    fn active(mut self, style: impl Fn(Style) -> Style) -> Self { ... }
    fn focused(mut self, style: impl Fn(Style) -> Style) -> Self { ... }

    // --- Focus ---
    fn focusable(mut self, handle: &FocusHandle) -> Self { ... }
    fn focus_scope(mut self) -> Self { ... }

    // --- Children ---
    fn child(mut self, child: impl Element) -> Self { ... }
    fn children(mut self, children: impl IntoIterator<Item = impl Element>) -> Self { ... }

    // --- Accessibility ---
    fn role(mut self, role: Role) -> Self { ... }
    fn label(mut self, label: impl Into<String>) -> Self { ... }

    // --- Window chrome ---
    fn drag_region(mut self) -> Self { ... }

    // --- Cursor ---
    fn cursor(mut self, cursor: CursorStyle) -> Self { ... }
}
```

### Conditional Rendering

Standard Rust control flow works naturally with the builder pattern:

```rust
fn user_status(cx: &mut Context) -> impl Element {
    let (logged_in, _) = cx.use_signal(false);

    div().child(
        if *logged_in.get(cx) {
            text("Welcome back!")
        } else {
            text("Please log in")
        }
    )
}
```

### Lists

```rust
fn todo_list(cx: &mut Context) -> impl Element {
    let (items, _) = cx.use_signal(vec!["Buy milk", "Walk dog", "Write code"]);

    div()
        .flex_col()
        .gap(4.0)
        .children(
            items.get(cx).iter().enumerate().map(|(i, item)| {
                keyed(i, || {
                    div()
                        .padding(8.0)
                        .bg(cx.theme().surface)
                        .rounded(4.0)
                        .child(text(item))
                })
            })
        )
}
```

---

## 12. Styling & Theming

### Style Properties

All visual properties are set inline via builder methods. There is no separate style object to manage — the element *is* its style.

### Color System

```rust
struct Color {
    r: f32, g: f32, b: f32, a: f32,
}

impl Color {
    fn hex(hex: &str) -> Color { ... }
    fn rgb(r: u8, g: u8, b: u8) -> Color { ... }
    fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color { ... }
    fn hsl(h: f32, s: f32, l: f32) -> Color { ... }

    fn lighter(&self, amount: f32) -> Color { ... }
    fn darker(&self, amount: f32) -> Color { ... }
    fn with_alpha(&self, alpha: f32) -> Color { ... }
}

enum Fill {
    Solid(Color),
    LinearGradient { angle: f32, stops: Vec<(f32, Color)> },
    RadialGradient { center: Point, radius: f32, stops: Vec<(f32, Color)> },
}

// Ergonomic conversion
impl From<Color> for Fill { ... }
impl From<&str> for Color { ... } // Hex strings: "#ff0000"
```

### Theme Struct

The theme provides a consistent design token system:

```rust
struct Theme {
    // --- Surface colors ---
    pub background: Color,
    pub surface: Color,
    pub surface_variant: Color,
    pub overlay: Color,

    // --- Brand colors ---
    pub primary: Color,
    pub primary_hover: Color,
    pub primary_active: Color,
    pub secondary: Color,
    pub accent: Color,

    // --- Semantic colors ---
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // --- Text colors ---
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub text_disabled: Color,
    pub text_on_primary: Color,
    pub text_on_secondary: Color,

    // --- Border colors ---
    pub border: Color,
    pub border_hover: Color,
    pub border_focus: Color,

    // --- Window chrome ---
    pub title_bar_background: Color,
    pub title_bar_text: Color,
    pub close_button: Color,
    pub close_button_hover: Color,
    pub close_button_active: Color,
    pub minimize_button: Color,
    pub maximize_button: Color,

    // --- Shadows ---
    pub shadow_sm: Shadow,
    pub shadow_md: Shadow,
    pub shadow_lg: Shadow,

    // --- Spacing ---
    pub spacing: Spacing,

    // --- Typography ---
    pub font_family: FontFamily,
    pub font_mono: FontFamily,
    pub font_size_xs: f32,     // 11
    pub font_size_sm: f32,     // 13
    pub font_size_md: f32,     // 15
    pub font_size_lg: f32,     // 18
    pub font_size_xl: f32,     // 24
    pub font_size_2xl: f32,    // 32

    // --- Radii ---
    pub radius_sm: f32,        // 4
    pub radius_md: f32,        // 6
    pub radius_lg: f32,        // 8
    pub radius_xl: f32,        // 12
    pub radius_full: f32,      // 9999

    // --- Animation ---
    pub transition_fast: Duration,   // 100ms
    pub transition_normal: Duration, // 200ms
    pub transition_slow: Duration,   // 300ms
}

struct Spacing {
    pub xs: f32,   // 4
    pub sm: f32,   // 8
    pub md: f32,   // 16
    pub lg: f32,   // 24
    pub xl: f32,   // 32
    pub xxl: f32,  // 48
}

struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}
```

### Built-in Themes

```rust
impl Theme {
    fn dark() -> Self { ... }   // Default dark theme
    fn light() -> Self { ... }  // Default light theme
}
```

### Custom Themes

Users create custom themes by modifying the struct:

```rust
fn my_theme() -> Theme {
    Theme {
        primary: Color::hex("#6366f1"),
        background: Color::hex("#0a0a0a"),
        ..Theme::dark()
    }
}

// Applied at app startup
App::new()
    .theme(my_theme())
    .run(app);
```

### Theme Access in Components

```rust
fn card(cx: &mut Context) -> impl Element {
    let theme = cx.theme();

    div()
        .padding(theme.spacing.md)
        .rounded(theme.radius_lg)
        .bg(theme.surface)
        .shadow(theme.shadow_md)
        .child(
            text("Card content")
                .font_size(theme.font_size_md)
                .color(theme.text_primary)
        )
}
```

---

## 13. Event Handling

### Flat Dispatch Model

Events are dispatched **directly to the target element** — there is no bubbling or capture phase. This is simpler to reason about and debug.

When an event occurs:

1. **Hit test**: Determine which element is under the cursor (for mouse events) or has focus (for keyboard events)
2. **Dispatch**: Call the element's event handler
3. **Done**: No propagation

```rust
impl EventDispatcher {
    fn dispatch_mouse_event(&mut self, event: &MouseEvent, cx: &mut Context) {
        // Hit test: walk the tree front-to-back, find topmost element containing the point
        let target = self.hit_test(event.position);

        if let Some(target) = target {
            target.handle_event(event, cx);
        }
    }
}
```

### Hit Testing

Hit testing walks the element tree in reverse paint order (front to back):

```rust
fn hit_test(&self, point: Point) -> Option<ElementId> {
    // Walk children in reverse order (last painted = on top)
    for child in self.children.iter().rev() {
        if let Some(hit) = child.hit_test(point) {
            return Some(hit);
        }
    }

    // Check self
    if self.bounds.contains(point) && self.is_interactive() {
        return Some(self.id);
    }

    None
}
```

### Event Types

```rust
struct ClickEvent {
    pub position: Point,
    pub button: MouseButton,
    pub click_count: u32, // 1 = single, 2 = double, 3 = triple
    pub modifiers: Modifiers,
}

struct MouseEvent {
    pub position: Point,
    pub button: MouseButton,
    pub modifiers: Modifiers,
}

struct ScrollEvent {
    pub delta: ScrollDelta,
    pub position: Point,
    pub modifiers: Modifiers,
}

struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub is_repeat: bool,
    pub text: Option<String>, // For text input (IME-resolved)
}

struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Cmd on macOS, Win on Windows
}

enum MouseButton { Left, Right, Middle, Back, Forward }

enum Key {
    Character(char),
    Enter, Escape, Tab, Backspace, Delete,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Home, End, PageUp, PageDown,
    F1, F2, /* ... */ F12,
    Space,
    // ...
}
```

### Click Detection

Clicks are synthesized from mouse down + mouse up pairs:

```rust
struct ClickDetector {
    pending: Option<PendingClick>,
    double_click_timeout: Duration,   // 500ms
    double_click_distance: f32,       // 4px
}

struct PendingClick {
    position: Point,
    button: MouseButton,
    timestamp: Instant,
    count: u32,
}
```

- Single click: mouse down + mouse up within same element
- Double click: two single clicks within 500ms and 4px
- Triple click: three single clicks within the same window

### Hover State

Hover is tracked by the framework. When the cursor enters/exits an element's bounds, `on_mouse_enter` and `on_mouse_leave` are called. Elements with `.hover()` style modifiers are automatically updated.

---

## 14. Focus System

### FocusHandle

A `FocusHandle` represents a focusable element. It is created from the context and attached to an element:

```rust
fn text_field(cx: &mut Context) -> impl Element {
    let focus = cx.use_focus_handle();
    let (value, set_value) = cx.use_signal(String::new());

    div()
        .focusable(&focus)
        .border(1.0, if focus.is_focused(cx) {
            cx.theme().border_focus
        } else {
            cx.theme().border
        })
        .on_key_down(move |event, cx| {
            if let Some(text) = &event.text {
                set_value.update(cx, |v| v.push_str(text));
            }
        })
        .child(text(&value.get(cx)))
}
```

### Focus Scopes

A focus scope groups focusable elements. Tab navigation cycles within the scope. This is essential for:

- **Modals**: Focus stays within the modal
- **Menus**: Arrow keys navigate within the menu
- **Panels**: Tab cycles through panel elements

```rust
fn modal(cx: &mut Context, children: impl IntoElements) -> impl Element {
    div()
        .focus_scope()  // Traps focus within
        .absolute()
        .inset(0.0)
        .bg(Color::BLACK.with_alpha(0.5))
        .items_center()
        .justify_center()
        .child(
            div()
                .w(400.0)
                .bg(cx.theme().surface)
                .rounded(12.0)
                .padding(24.0)
                .shadow(cx.theme().shadow_lg)
                .children(children)
        )
}
```

### Tab Order

Within a focus scope, tab order follows DOM (tree) order by default:

1. Tab moves to the next focusable element in tree order
2. Shift+Tab moves to the previous
3. At the end of a scope, wrap to the beginning
4. Elements can opt out with `.tab_index(-1)` (focusable programmatically but not via tab)

### Focus Management API

```rust
impl FocusHandle {
    fn focus(&self, cx: &mut Context);         // Programmatically focus this element
    fn blur(&self, cx: &mut Context);          // Remove focus
    fn is_focused(&self, cx: &Context) -> bool;
}

impl Context {
    fn focused_element(&self) -> Option<&FocusHandle>;
    fn focus_next(&mut self);      // Move focus forward
    fn focus_previous(&mut self);  // Move focus backward
}
```

---

## 15. Action & Keybinding System

### Named Actions

Actions are named operations that can be triggered by keyboard shortcuts, menu items, or programmatically. They decouple "what to do" from "how it's triggered":

```rust
// Define actions
actions!(
    app,
    [Quit, NewWindow, CloseWindow, ToggleTheme]
);

actions!(
    editor,
    [Copy, Paste, Cut, Undo, Redo, SelectAll, Find, Replace]
);

actions!(
    list,
    [MoveUp, MoveDown, Select, Delete]
);
```

Each action is a zero-sized type that implements the `Action` trait:

```rust
trait Action: std::any::Any + std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn namespace(&self) -> &'static str;
}
```

### Keybinding Registry

Keybindings map key combinations to actions, scoped by context:

```rust
impl App {
    fn keybindings(&mut self) -> &mut KeybindingRegistry {
        // ...
    }
}

// Global keybindings
app.keybindings()
    .bind("cmd-q", Quit)
    .bind("cmd-n", NewWindow)
    .bind("cmd-w", CloseWindow);

// Context-scoped keybindings (active when an editor element is focused)
app.keybindings()
    .context("Editor")
    .bind("cmd-c", Copy)
    .bind("cmd-v", Paste)
    .bind("cmd-x", Cut)
    .bind("cmd-z", Undo)
    .bind("cmd-shift-z", Redo)
    .bind("cmd-a", SelectAll);

// Context-scoped keybindings (active when a list element is focused)
app.keybindings()
    .context("List")
    .bind("up", MoveUp)
    .bind("down", MoveDown)
    .bind("enter", Select)
    .bind("backspace", Delete);
```

### Key Notation

Key combinations use a human-readable string notation:

```
"cmd-s"           → Cmd + S (macOS) / Ctrl + S (Windows/Linux)
"ctrl-shift-p"    → Ctrl + Shift + P
"alt-enter"       → Alt + Enter
"cmd-shift-["     → Cmd + Shift + [
"escape"          → Escape
"f5"              → F5
```

`cmd` is automatically mapped to `Ctrl` on Windows/Linux and `Cmd` on macOS.

### Action Dispatch

When a key event occurs:

1. Check the focused element's context for matching keybindings
2. Walk up the focus scope tree checking parent contexts
3. Check global keybindings
4. If a match is found, dispatch the action

```rust
fn dispatch_key_event(&mut self, event: &KeyEvent, cx: &mut Context) {
    let key_combo = KeyCombo::from_event(event);

    // Walk up from focused element's context
    let mut current_context = cx.focused_context();

    while let Some(context) = current_context {
        if let Some(action) = self.keybindings.match_in_context(context, &key_combo) {
            self.dispatch_action(action, cx);
            return;
        }
        current_context = context.parent();
    }

    // Check global bindings
    if let Some(action) = self.keybindings.match_global(&key_combo) {
        self.dispatch_action(action, cx);
    }
}
```

### Action Handlers

Components register action handlers:

```rust
fn editor(cx: &mut Context) -> impl Element {
    let (content, set_content) = cx.use_signal(String::new());

    div()
        .context("Editor")  // This element provides the "Editor" context
        .on_action::<Copy>(move |_, cx| {
            let text = get_selected_text(cx);
            cx.clipboard_write(&text);
        })
        .on_action::<Paste>(move |_, cx| {
            if let Some(text) = cx.clipboard_read() {
                insert_text(cx, &text);
            }
        })
        .on_action::<Undo>(move |_, cx| {
            undo(cx);
        })
        .child(/* editor content */)
}
```

---

## 16. Async Runtime

### Custom Executor

mozui embeds a lightweight async executor directly in the event loop. No external runtime (Tokio, smol) is needed.

### Architecture

```
Main Thread Event Loop
 ├── Platform events (mouse, keyboard, window)
 ├── Timer events (scheduled wakeups)
 ├── Main-thread task polling (cooperative)
 └── Redraw (when dirty)

Background Thread Pool
 ├── CPU-bound tasks
 └── Results sent back to main thread via channel
```

### Main Thread Tasks

`cx.spawn()` creates a task that runs on the main thread. It is polled cooperatively between event processing and rendering. Because it runs on the main thread, closures don't need to be `Send`:

```rust
fn fetch_data(cx: &mut Context) -> impl Element {
    let (data, set_data) = cx.use_signal(None::<String>);
    let (loading, set_loading) = cx.use_signal(false);

    let load = move |cx: &mut Context| {
        set_loading.set(cx, true);

        cx.spawn(async move |cx| {
            let response = cx.background(async {
                // This runs on the thread pool (Send required here)
                reqwest::get("https://api.example.com/data")
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap()
            }).await;

            // Back on main thread — can update signals directly
            set_data.set(&cx, Some(response));
            set_loading.set(&cx, false);
        });
    };

    div()
        .child(match data.get(cx) {
            Some(text) => text(text),
            None if *loading.get(cx) => text("Loading..."),
            None => text("No data"),
        })
        .child(
            button("Load").on_click(move |_, cx| load(cx))
        )
}
```

### Background Executor

`cx.background()` runs a `Send + 'static` future on a thread pool. The result is delivered back to the main thread:

```rust
impl Context {
    /// Spawn a task on the main thread (not Send)
    fn spawn<F>(&self, future: F) -> TaskHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static;

    /// Run a future on the background thread pool (must be Send)
    fn background<F>(&self, future: F) -> TaskHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;
}

struct TaskHandle<T> {
    // Can be awaited from a main-thread task, or polled
}

impl<T> Future for TaskHandle<T> {
    type Output = T;
    // ...
}
```

### Timer / Delayed Execution

```rust
impl Context {
    fn set_timeout(&self, duration: Duration, callback: impl FnOnce(&mut Context) + 'static);
    fn set_interval(&self, duration: Duration, callback: impl Fn(&mut Context) + 'static) -> IntervalHandle;
}

struct IntervalHandle { /* ... */ }

impl IntervalHandle {
    fn cancel(self);
}
```

### Executor Implementation

The executor is a simple single-threaded polling loop integrated with the platform event loop:

```rust
struct Executor {
    /// Main-thread tasks
    ready_queue: VecDeque<Task>,

    /// Background thread pool
    thread_pool: ThreadPool,

    /// Channel for background task results
    result_rx: Receiver<CompletedTask>,
}

impl Executor {
    fn poll_ready_tasks(&mut self, cx: &mut Context) {
        // Poll up to N tasks per frame to avoid starving rendering
        let max_polls = 64;
        for _ in 0..max_polls {
            if let Some(mut task) = self.ready_queue.pop_front() {
                match task.poll(cx) {
                    Poll::Pending => { /* task will re-enqueue itself when woken */ }
                    Poll::Ready(_) => { /* task complete, drop it */ }
                }
            } else {
                break;
            }
        }

        // Collect results from background tasks
        while let Ok(completed) = self.result_rx.try_recv() {
            completed.deliver(cx);
        }
    }
}
```

The background thread pool uses `std::thread` with a simple work-stealing queue. The number of threads defaults to `num_cpus - 1` (leaving one core for the main/UI thread).

---

## 17. Accessibility

### Design-Aware Deferral

Full accessibility support is deferred, but the element API includes accessibility fields from day one to avoid a painful retrofit:

```rust
// Every element can carry optional a11y info
div()
    .role(Role::Button)
    .label("Submit form")

text_input()
    .role(Role::TextField)
    .label("Email address")
```

### Accessibility Info

```rust
struct AccessibilityInfo {
    role: Option<Role>,
    label: Option<String>,
    description: Option<String>,
    value: Option<String>,
    state: AccessibilityState,
}

struct AccessibilityState {
    disabled: bool,
    selected: bool,
    expanded: Option<bool>,
    checked: Option<bool>,
}

enum Role {
    Button,
    TextField,
    Label,
    Checkbox,
    RadioButton,
    Slider,
    List,
    ListItem,
    Menu,
    MenuItem,
    Dialog,
    Alert,
    Tab,
    TabPanel,
    Heading,
    Image,
    Link,
    Separator,
    Toolbar,
    ScrollView,
    Group,
    // ...
}
```

### Future Integration Path

When accessibility is implemented:

1. **Integrate `accesskit`**: Map mozui's element tree to an `accesskit` tree
2. **Platform bridges**: `accesskit` handles NSAccessibility (macOS), UI Automation (Windows), AT-SPI (Linux)
3. **Announce changes**: Signal mutations that change visible text trigger accessibility announcements
4. **Focus tracking**: The focus system already maps cleanly to a11y focus

The key insight is that `role`, `label`, `FocusHandle`, and the named action system are already the building blocks accessibility needs. The deferred work is primarily the platform bridge code.

---

## 18. Application Lifecycle

### App Startup

```rust
fn main() {
    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "My App".into(),
            size: Size::new(1200.0, 800.0),
            ..Default::default()
        })
        .keybindings(|kb| {
            kb.bind("cmd-q", Quit);
            kb.bind("cmd-w", CloseWindow);
        })
        .run(app_root);
}

fn app_root(cx: &mut Context) -> impl Element {
    div()
        .w_full()
        .h_full()
        .bg(cx.theme().background)
        .flex_col()
        .child(title_bar(cx))
        .child(main_content(cx))
}
```

### App Struct

```rust
struct App {
    platform: Box<dyn Platform>,
    context: AppContext,
    renderer: Renderer,
    executor: Executor,
    keybindings: KeybindingRegistry,
    theme: Theme,
    windows: Vec<Window>,
}

impl App {
    fn new() -> AppBuilder { ... }

    fn run(self, root: fn(&mut Context) -> impl Element) -> ! {
        // 1. Initialize platform
        // 2. Create window
        // 3. Initialize wgpu
        // 4. Enter event loop (never returns)
        self.platform.run(move |event| {
            match event {
                PlatformEvent::RedrawRequested => {
                    self.executor.poll_ready_tasks(&mut self.context);
                    self.layout();
                    self.render();
                    self.present();
                }
                event => {
                    self.dispatch_event(event);
                    if self.context.is_dirty() {
                        self.platform.request_redraw();
                    }
                }
            }
        });
    }
}
```

### Frame Lifecycle

Each frame proceeds through these phases:

```
1. Event Processing
   └── Platform events → dispatch to elements → signal mutations → dirty marking

2. Task Polling
   └── Poll main-thread async tasks → possible signal mutations → dirty marking

3. Layout
   └── Recompute layout for dirty subtrees via Taffy

4. Render
   └── Walk element tree → produce draw commands

5. Paint
   └── Batch draw commands → upload to GPU → execute render pass

6. Present
   └── Swap buffers, display frame
```

### Multi-Window Support

mozui supports multiple windows. Each window has its own:

- Platform window handle
- wgpu surface
- Element tree
- Layout tree

But they share:
- The `AppContext` arena (signals can be shared across windows)
- The async executor
- The keybinding registry
- The theme

```rust
impl Context {
    fn open_window(&mut self, options: WindowOptions, root: fn(&mut Context) -> impl Element) -> WindowHandle;
    fn close_window(&mut self, handle: WindowHandle);
    fn window(&self) -> &Window; // Current window
}
```

---

## 19. Error Handling Strategy

### Philosophy

mozui uses Rust's type system to prevent errors at compile time where possible. Runtime errors use `Result<T, E>` at API boundaries and panic for internal invariant violations.

### User-Facing API

- **Builder methods**: Infallible. Invalid style values are clamped (e.g., negative padding becomes 0).
- **Signal access**: Infallible within a component's render function. Panics if accessed outside a context (programming error).
- **Window operations**: Return `Result` for operations that can fail (e.g., `open_window` may fail if the platform can't create a window).
- **Async tasks**: Results delivered via `TaskHandle<Result<T, E>>` — users handle errors in their async code.

### Internal Errors

- **Renderer errors** (wgpu device lost, surface timeout): Attempt recovery. If recovery fails, log and close the affected window.
- **Layout errors**: Should never happen with valid Taffy input. Panic in debug, fallback to zero-size in release.
- **Platform errors**: Logged via `tracing`. Non-fatal errors are handled gracefully.

### Logging

mozui uses the `tracing` crate for structured logging:

```rust
// Internal
tracing::warn!("Glyph cache full, evicting {} entries", count);
tracing::error!("wgpu surface lost, attempting recovery");
tracing::debug!("Layout computed in {:?} for {} nodes", duration, count);
```

Users can subscribe to mozui's tracing spans for debugging:

```rust
// In user's main.rs
tracing_subscriber::fmt()
    .with_env_filter("mozui=debug,my_app=info")
    .init();
```

---

## 20. Testing Strategy

### Unit Tests

Each subsystem has unit tests:

- **Layout**: Verify Taffy integration produces correct positions/sizes for known element trees
- **Signals**: Test dependency tracking, dirty marking, cleanup on drop
- **Events**: Test hit testing, click detection, focus navigation
- **Actions**: Test keybinding matching, context scoping
- **Style**: Test builder method correctness, theme defaults

### Visual Regression Tests

Render known element trees to an off-screen texture and compare against reference images:

```rust
#[test]
fn button_renders_correctly() {
    let app = TestApp::new();
    let image = app.render_to_image(Size::new(200.0, 50.0), |cx| {
        button("Click me")
            .bg(Color::hex("#3b82f6"))
            .rounded(6.0)
            .padding_x(16.0)
            .padding_y(8.0)
    });

    assert_image_snapshot!(image, "button_default");
}
```

### Integration Tests

Test full application scenarios:

```rust
#[test]
fn counter_increments() {
    let app = TestApp::new();
    app.mount(counter);

    assert_eq!(app.text_content(), "Count: 0");

    app.click_button("Increment");
    assert_eq!(app.text_content(), "Count: 1");

    app.click_button("Increment");
    assert_eq!(app.text_content(), "Count: 2");
}
```

### Test Utilities

mozui provides a `TestApp` that runs without a real window or GPU:

```rust
struct TestApp {
    context: AppContext,
    element_tree: ElementTree,
}

impl TestApp {
    fn new() -> Self;
    fn mount(&mut self, component: fn(&mut Context) -> impl Element);
    fn render_to_image(&self, size: Size, component: fn(&mut Context) -> impl Element) -> Image;
    fn click_at(&mut self, position: Point);
    fn click_button(&mut self, label: &str);
    fn type_text(&mut self, text: &str);
    fn press_key(&mut self, key: Key, modifiers: Modifiers);
    fn text_content(&self) -> String;
    fn find_by_label(&self, label: &str) -> Option<&dyn Element>;
}
```

---

## 21. Performance Considerations

### Priority Order

Per the design philosophy, performance is the **lowest priority** after DX, binary size, and correctness. However, a GUI framework has a baseline performance requirement: **60fps for typical UIs**. The following optimizations ensure this:

### Rendering

- **Batching**: Consecutive draw commands with the same shader/texture are merged into a single draw call
- **Texture atlas**: Glyphs and small images share a single texture to minimize bind calls
- **SDF rectangles**: Rounded rects, borders, and shadows are a single quad each — no tessellation

### Layout

- **Incremental**: Only dirty subtrees are relaid out
- **Skip identical**: If a component re-renders but produces identical layout styles, skip Taffy update

### Reactivity

- **Fine-grained tracking**: Only views that read a changed signal re-render
- **No virtual DOM diffing**: Direct signal-to-view mapping, no tree diff needed

### Memory

- **Arena allocation**: Signal data is arena-allocated, reducing allocator pressure
- **Element recycling**: Element structs are reused across frames where possible

### What We Explicitly Don't Optimize (Yet)

- Multi-threaded rendering (single render thread is fine for UI)
- Damage tracking / partial repaint
- GPU compute for layout
- SIMD for hit testing

These can be added later if profiling shows they're needed.

---

## 22. Implementation Phases

### Phase 1: Foundation (Weeks 1-4)

**Goal**: Open a window, render a colored rectangle.

- [ ] Project structure (Cargo workspace)
- [ ] macOS platform shell (borderless NSWindow)
- [ ] wgpu initialization and surface creation
- [ ] Basic rect shader (solid color, rounded corners)
- [ ] Event loop (process events, request redraw)
- [ ] Basic `div()` element with `.w()`, `.h()`, `.bg()`, `.rounded()`

**Milestone**: `App::new().run(|cx| div().w(200.0).h(100.0).bg(Color::RED).rounded(8.0))`

### Phase 2: Layout & Text (Weeks 5-8)

**Goal**: Lay out multiple elements with Flexbox, render text.

- [ ] Taffy integration
- [ ] Flexbox builder methods (`.flex_row()`, `.items_center()`, `.gap()`, etc.)
- [ ] `font-kit` integration for font loading
- [ ] Text shaping and glyph rasterization
- [ ] Glyph texture atlas
- [ ] Glyph shader
- [ ] `text()` element
- [ ] Theme struct with defaults

**Milestone**: A vertical stack of styled text labels with padding and spacing.

### Phase 3: Reactivity & Interaction (Weeks 9-12)

**Goal**: Interactive components with state.

- [ ] Signal implementation (arena, handles, dependency tracking)
- [ ] `cx.use_signal()` API
- [ ] Dirty marking and incremental re-render
- [ ] Mouse event dispatch (hit testing, click detection)
- [ ] Keyboard event dispatch
- [ ] `.on_click()`, `.on_mouse_enter()`, `.on_mouse_leave()`, `.on_key_down()`
- [ ] Hover and active states (`.hover()`, `.active()`)
- [ ] `button()` element

**Milestone**: A working counter app (click button, number increments).

### Phase 4: Focus & Actions (Weeks 13-16)

**Goal**: Keyboard navigation and shortcuts.

- [ ] `FocusHandle` implementation
- [ ] Focus scopes
- [ ] Tab navigation
- [ ] Action system (`actions!` macro, `Action` trait)
- [ ] Keybinding registry
- [ ] Contextual keybinding dispatch
- [ ] `text_input()` element (basic)

**Milestone**: A form with multiple text inputs, tab navigation, and keyboard shortcuts.

### Phase 5: Async & Window Chrome (Weeks 17-20)

**Goal**: Async operations and custom window chrome.

- [ ] Custom async executor (main thread)
- [ ] Background thread pool
- [ ] `cx.spawn()` and `cx.background()`
- [ ] Timer support (`set_timeout`, `set_interval`)
- [ ] Custom title bar component
- [ ] Window control buttons (close, minimize, maximize)
- [ ] Drag regions and resize handles
- [ ] Clipboard support
- [ ] Cursor management

**Milestone**: A window with custom chrome, a text input, and a button that fetches data asynchronously.

### Phase 6: Polish & Cross-Platform (Weeks 21-28)

**Goal**: Windows and Linux support, visual polish.

- [ ] Windows platform shell (Win32)
- [ ] Linux platform shell (Wayland + X11)
- [ ] Shadows and gradients in the renderer
- [ ] Scroll containers
- [ ] Image element
- [ ] Light theme
- [ ] `use_memo`, `use_effect`
- [ ] Multi-window support
- [ ] Comprehensive examples

**Milestone**: The same app runs on macOS, Windows, and Linux with identical appearance.

### Phase 7: Ecosystem & DX (Weeks 29+)

**Goal**: Make it a joy to use.

- [ ] Documentation and guides
- [ ] Devtools (element inspector, signal debugger)
- [ ] Hot reload exploration
- [ ] Component library (common patterns)
- [ ] Accessibility (`accesskit` integration)
- [ ] WASM target exploration
- [ ] Performance profiling and optimization

---

## 23. Dependencies

### Core Dependencies

| Crate | Purpose | Why |
|-------|---------|-----|
| `wgpu` | GPU abstraction | Cross-platform GPU access without native API code |
| `taffy` | Flexbox layout | Mature, pure-Rust, CSS-spec-compliant |
| `font-kit` | Font loading & discovery | Platform-native font access, high-quality glyph rasterization |
| `tracing` | Structured logging | Standard Rust logging, zero-cost when disabled |

### Platform Dependencies

| Crate | Platform | Purpose |
|-------|----------|---------|
| `objc2` | macOS | Objective-C runtime bindings for AppKit |
| `block2` | macOS | Objective-C block support |
| `core-foundation` | macOS | Core Foundation types |
| `windows-rs` | Windows | Win32 API bindings |
| `wayland-client` | Linux | Wayland protocol client |
| `x11rb` | Linux | X11 protocol bindings |
| `xkbcommon` | Linux | Keyboard handling |

### Utility Dependencies

| Crate | Purpose |
|-------|---------|
| `raw-window-handle` | Window handle abstraction for wgpu surface creation |
| `parking_lot` | Faster Mutex/RwLock for internal synchronization |
| `smallvec` | Stack-allocated small vectors for common hot paths |
| `rustc-hash` | Fast HashMap for internal ID lookups |

### Dev Dependencies

| Crate | Purpose |
|-------|---------|
| `insta` | Snapshot testing (for visual regression) |
| `criterion` | Benchmarking |

### Explicitly Avoided

| Crate | Why Not |
|-------|---------|
| `winit` | Custom platform shells provide full control |
| `tokio` / `smol` | Custom executor is lighter, no external runtime dependency |
| `skia-safe` | C++ dependency, violates lightweight philosophy |
| `serde` | Not needed in core (users can add it for their types) |

---

## 24. API Reference Sketches

### Complete Counter Example

```rust
use mozui::*;

fn main() {
    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Counter".into(),
            size: Size::new(400.0, 300.0),
            ..Default::default()
        })
        .run(app);
}

fn app(cx: &mut Context) -> impl Element {
    div()
        .w_full()
        .h_full()
        .bg(cx.theme().background)
        .flex_col()
        .child(title_bar(cx))
        .child(counter(cx))
}

fn title_bar(cx: &mut Context) -> impl Element {
    div()
        .h(38.0)
        .w_full()
        .bg(cx.theme().title_bar_background)
        .drag_region()
        .flex_row()
        .items_center()
        .padding_x(16.0)
        .child(text("Counter").font_size(13.0).color(cx.theme().title_bar_text))
}

fn counter(cx: &mut Context) -> impl Element {
    let (count, set_count) = cx.use_signal(0i32);

    div()
        .flex_grow(1.0)
        .flex_col()
        .items_center()
        .justify_center()
        .gap(16.0)
        .child(
            text(&format!("{}", count.get(cx)))
                .font_size(48.0)
                .bold()
                .color(cx.theme().text_primary)
        )
        .child(
            div()
                .flex_row()
                .gap(8.0)
                .child(
                    button("-")
                        .on_click(move |_, cx| set_count.update(cx, |n| *n -= 1))
                )
                .child(
                    button("+")
                        .on_click(move |_, cx| set_count.update(cx, |n| *n += 1))
                )
        )
}
```

### Todo App Example

```rust
use mozui::*;

fn main() {
    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Todos".into(),
            size: Size::new(500.0, 600.0),
            ..Default::default()
        })
        .keybindings(|kb| {
            kb.bind("cmd-q", Quit);
        })
        .run(app);
}

actions!(app, [Quit, AddTodo, DeleteTodo]);

#[derive(Clone)]
struct Todo {
    id: u64,
    text: String,
    done: bool,
}

fn app(cx: &mut Context) -> impl Element {
    let (todos, set_todos) = cx.use_signal(Vec::<Todo>::new());
    let (input, set_input) = cx.use_signal(String::new());
    let (next_id, set_next_id) = cx.use_signal(0u64);

    let remaining = cx.use_memo(move |cx| {
        todos.get(cx).iter().filter(|t| !t.done).count()
    });

    let add_todo = move |cx: &mut Context| {
        let text = input.get(cx).clone();
        if !text.is_empty() {
            let id = *next_id.get(cx);
            set_todos.update(cx, |todos| {
                todos.push(Todo { id, text, done: false });
            });
            set_next_id.update(cx, |id| *id += 1);
            set_input.set(cx, String::new());
        }
    };

    let toggle_todo = move |id: u64, cx: &mut Context| {
        set_todos.update(cx, |todos| {
            if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
                todo.done = !todo.done;
            }
        });
    };

    let delete_todo = move |id: u64, cx: &mut Context| {
        set_todos.update(cx, |todos| {
            todos.retain(|t| t.id != id);
        });
    };

    div()
        .w_full()
        .h_full()
        .bg(cx.theme().background)
        .flex_col()
        .child(title_bar(cx))
        .child(
            div()
                .flex_grow(1.0)
                .flex_col()
                .padding(24.0)
                .gap(16.0)
                // Header
                .child(
                    text("Todos")
                        .font_size(cx.theme().font_size_xl)
                        .bold()
                        .color(cx.theme().text_primary)
                )
                // Input row
                .child(
                    div()
                        .flex_row()
                        .gap(8.0)
                        .child(
                            text_input()
                                .flex_grow(1.0)
                                .placeholder("What needs to be done?")
                                .value(input)
                                .on_change(move |val, cx| set_input.set(cx, val))
                                .on_submit(move |_, cx| add_todo(cx))
                        )
                        .child(
                            button("Add").on_click(move |_, cx| add_todo(cx))
                        )
                )
                // Todo list
                .child(
                    div()
                        .flex_col()
                        .gap(4.0)
                        .flex_grow(1.0)
                        .overflow_y_scroll()
                        .children(
                            todos.get(cx).iter().map(|todo| {
                                let id = todo.id;
                                keyed(id, || {
                                    todo_row(cx, todo, move |cx| toggle_todo(id, cx), move |cx| delete_todo(id, cx))
                                })
                            })
                        )
                )
                // Footer
                .child(
                    text(&format!("{} items remaining", remaining.get(cx)))
                        .font_size(cx.theme().font_size_sm)
                        .color(cx.theme().text_secondary)
                )
        )
}

fn todo_row(
    cx: &mut Context,
    todo: &Todo,
    on_toggle: impl Fn(&mut Context) + 'static,
    on_delete: impl Fn(&mut Context) + 'static,
) -> impl Element {
    div()
        .flex_row()
        .items_center()
        .padding(12.0)
        .rounded(cx.theme().radius_md)
        .bg(cx.theme().surface)
        .hover(|s| s.bg(cx.theme().surface_variant))
        .gap(12.0)
        .child(
            checkbox()
                .checked(todo.done)
                .on_change(move |_, cx| on_toggle(cx))
        )
        .child(
            text(&todo.text)
                .flex_grow(1.0)
                .color(if todo.done {
                    cx.theme().text_disabled
                } else {
                    cx.theme().text_primary
                })
        )
        .child(
            button("x")
                .on_click(move |_, cx| on_delete(cx))
                .opacity(0.5)
                .hover(|s| s.opacity(1.0))
        )
}
```

### Custom Theme Example

```rust
fn catppuccin_mocha() -> Theme {
    Theme {
        background: Color::hex("#1e1e2e"),
        surface: Color::hex("#313244"),
        surface_variant: Color::hex("#45475a"),
        primary: Color::hex("#cba6f7"),
        primary_hover: Color::hex("#b4befe"),
        text_primary: Color::hex("#cdd6f4"),
        text_secondary: Color::hex("#a6adc8"),
        text_disabled: Color::hex("#585b70"),
        border: Color::hex("#45475a"),
        error: Color::hex("#f38ba8"),
        success: Color::hex("#a6e3a1"),
        warning: Color::hex("#f9e2af"),
        ..Theme::dark()
    }
}
```

---

## Appendix A: Glossary

| Term | Definition |
|------|-----------|
| **Element** | A node in the UI tree. Has style, children, and optional event handlers. |
| **Component** | A function that returns an element. May use signals for state. |
| **Signal** | A reactive value. Reading it registers a dependency; writing it triggers re-renders. |
| **Arena** | The centralized store that owns all signal data. |
| **FocusHandle** | A handle to a focusable element. Used for programmatic focus management. |
| **Focus Scope** | A group of focusable elements. Tab navigation cycles within the scope. |
| **Action** | A named operation (e.g., `Copy`, `Undo`) that can be triggered by keybindings. |
| **Platform Shell** | OS-specific code for window creation, event loops, and system integration. |
| **Draw Command** | An instruction to the renderer (draw rect, draw text, push clip, etc.). |
| **Dirty** | A view whose output may have changed due to a signal mutation. Needs re-render. |

## Appendix B: File Naming Conventions

```
snake_case.rs         — All Rust source files
mod.rs                — Module roots
lib.rs                — Crate roots
traits.rs             — Trait definitions
types.rs              — Shared type definitions
```

## Appendix C: Coordinate System

- **Origin**: Top-left corner of the window
- **X axis**: Positive to the right
- **Y axis**: Positive downward
- **Units**: Logical pixels (scaled by the display's scale factor)
- **Physical pixels**: Logical pixels * scale_factor (used only for GPU operations)

```rust
struct Point {
    x: f32,
    y: f32,
}

struct Size {
    width: f32,
    height: f32,
}

struct Rect {
    origin: Point,
    size: Size,
}

impl Rect {
    fn contains(&self, point: Point) -> bool { ... }
    fn intersects(&self, other: &Rect) -> bool { ... }
    fn union(&self, other: &Rect) -> Rect { ... }
    fn inset(&self, amount: f32) -> Rect { ... }
}
```
