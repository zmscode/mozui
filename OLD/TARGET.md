# Multi-Platform Targeting

mozui currently targets macOS only. This document outlines the infrastructure
work required to compile to WASM, iOS, and Android.

## Current Architecture

**Platform-agnostic crates** (no changes needed):
mozui-style, mozui-events, mozui-layout, mozui-reactive, mozui-executor,
mozui-elements, mozui-icons, mozui-devtools, mozui-text, mozui-renderer (core)

**Platform-locked crates** (macOS only):
- `mozui-platform` — AppKit event loop, NSWindow, clipboard, file dialogs, cursors
- `mozui-renderer/src/gpu.rs` — wgpu surface creation via raw window handle

### Platform Trait Surface

The entire platform boundary is two traits + a handful of free functions:

```rust
// traits.rs
trait Platform {
    fn run(&mut self, callback: EventCallback) -> !;
    fn open_window(&mut self, options: WindowOptions) -> (WindowId, Box<dyn PlatformWindow>);
    fn screens(&self) -> Vec<Screen>;
    fn set_cursor(&self, cursor: CursorStyle);
    fn clipboard_read(&self) -> Option<String>;
    fn clipboard_write(&self, text: &str);
}

trait PlatformWindow: HasWindowHandle + HasDisplayHandle {
    fn bounds(&self) -> Rect;
    fn set_bounds(&mut self, bounds: Rect);
    fn content_size(&self) -> Size;
    fn scale_factor(&self) -> f32;
    fn is_focused(&self) -> bool;
    fn is_visible(&self) -> bool;
    fn is_maximized(&self) -> bool;
    fn set_title(&mut self, title: &str);
    fn minimize(&mut self);
    fn maximize(&mut self);
    fn close(&mut self);
    fn request_redraw(&self);
    fn begin_drag_move(&self);
    fn titlebar_height(&self) -> f32;
    fn titlebar_style(&self) -> TitlebarStyle;
}

// Free functions that also need per-platform impls:
fn open_url(url: &str);
fn open_file_dialog(options: FileDialogOptions) -> Vec<PathBuf>;
fn save_file_dialog(options: FileDialogOptions) -> Option<PathBuf>;
```

---

## Phase 0: Refactor Platform Crate Structure

Before adding any new target, restructure `mozui-platform` for multi-backend:

```
crates/mozui-platform/src/
  lib.rs            # cfg-gated module selection + create_platform()
  traits.rs         # Platform + PlatformWindow traits (unchanged)
  macos/            # existing macOS backend (already here)
  web/              # new: WASM backend
  ios/              # new: iOS backend
  android/          # new: Android backend
```

**Steps:**
1. Move free functions (`clipboard_read`, `open_url`, `open_file_dialog`, etc.)
   behind the `Platform` trait so all platform behavior routes through one
   interface. No more standalone `#[cfg]`-gated functions.
2. Make `create_platform()` dispatch based on `cfg(target_arch)` /
   `cfg(target_os)`.
3. Add `#[cfg(target_arch = "wasm32")]`, `#[cfg(target_os = "ios")]`,
   `#[cfg(target_os = "android")]` gates in `lib.rs`.

**Estimated effort:** Small (1-2 days). Pure refactor, no new functionality.

---

## Phase 1: WASM Target

### 1.1 Platform Backend (`mozui-platform/src/web/`)

Implement `Platform` and `PlatformWindow` for the browser.

| Trait method | Browser equivalent |
|---|---|
| `run(callback)` | `requestAnimationFrame` loop via `wasm_bindgen_futures` |
| `open_window(options)` | Create/find a `<canvas>` element in the DOM |
| `screens()` | `window.screen` API |
| `set_cursor(cursor)` | `canvas.style.cursor = "..."` |
| `clipboard_read()` | `navigator.clipboard.readText()` (async) |
| `clipboard_write(text)` | `navigator.clipboard.writeText()` (async) |
| `open_file_dialog()` | Hidden `<input type="file">` click |
| `save_file_dialog()` | `URL.createObjectURL` + download link |
| `open_url(url)` | `window.open(url)` |

`PlatformWindow` maps to a `<canvas>`:
- `content_size()` → `canvas.clientWidth / clientHeight`
- `scale_factor()` → `window.devicePixelRatio`
- `request_redraw()` → schedule next `requestAnimationFrame`
- `bounds()` → `getBoundingClientRect()`
- `is_focused()` → `document.hasFocus()` + focus events
- Window management methods (`minimize`, `maximize`, `close`,
  `begin_drag_move`) → no-op or unsupported

**Key dependencies to add:**
```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Window", "Document", "HtmlCanvasElement", "Navigator",
    "Clipboard", "Screen", "KeyboardEvent", "MouseEvent",
    "WheelEvent", "PointerEvent", "HtmlInputElement",
] }
js-sys = "0.3"
```

### 1.2 Event Translation

Map DOM events to `mozui_events::PlatformEvent`:

| DOM event | mozui event |
|---|---|
| `pointermove` | `MouseMoved { position }` |
| `pointerdown` / `pointerup` | `MouseDown` / `MouseUp` |
| `wheel` | `ScrollWheel { delta_x, delta_y }` |
| `keydown` / `keyup` | `KeyDown { key, modifiers }` / `KeyUp` |
| `resize` (ResizeObserver) | `Resized { size }` |
| `focus` / `blur` | `Focused` / `Unfocused` |
| `pointerenter` / `pointerleave` | `MouseEntered` / `MouseExited` |

Touch events (`touchstart`, `touchmove`, `touchend`) should be mapped to
mouse events for now. Dedicated touch/gesture support is a separate effort.

### 1.3 Renderer (wgpu WebGPU/WebGL2)

wgpu already supports `wasm32` via WebGPU and WebGL2 backends.

Changes needed in `mozui-renderer/src/gpu.rs`:
- Use `wgpu::Instance::create_surface(canvas)` instead of
  `create_surface_unsafe(RawHandle)` on WASM.
- Replace `pollster::block_on()` with `.await` (no blocking on WASM).
  This means `GpuContext::new_with_surface` must become async, or use
  `wasm_bindgen_futures::spawn_local`.

### 1.4 Text Rendering

cosmic-text and swash both compile to WASM. The main consideration:
- No system fonts on WASM. Bundle a default font (e.g. Inter) and load via
  `cosmic_text::FontSystem::new_with_fonts()`.
- Font loading must be async (fetch from server) or embedded in the binary.

### 1.5 Build & Packaging

```bash
# Install
cargo install wasm-pack

# Build
wasm-pack build crates/mozui --target web

# Or with trunk for dev server
trunk serve examples/layout_demo/index.html
```

Each example needs an `index.html` that creates a `<canvas>` and loads the
WASM module. Consider a shared template.

**Estimated effort:** Large (2-3 weeks).
Biggest risks: async surface creation, font loading, clipboard API differences.

---

## Phase 2: iOS Target

### 2.1 Platform Backend (`mozui-platform/src/ios/`)

| Trait method | iOS equivalent |
|---|---|
| `run(callback)` | `UIApplicationMain` + `CADisplayLink` for frame loop |
| `open_window(options)` | Create `UIWindow` + `UIViewController` with `MTKView` |
| `screens()` | `UIScreen.main` |
| `set_cursor(cursor)` | No-op (no cursor on iOS) |
| `clipboard_read()` | `UIPasteboard.general.string` |
| `clipboard_write(text)` | `UIPasteboard.general.string = text` |
| `open_file_dialog()` | `UIDocumentPickerViewController` |
| `save_file_dialog()` | `UIDocumentPickerViewController` (export mode) |
| `open_url(url)` | `UIApplication.shared.open(url)` |

`PlatformWindow` maps to `UIWindow` + `MTKView`:
- `content_size()` → `view.bounds.size` adjusted for safe area
- `scale_factor()` → `UIScreen.main.scale`
- `request_redraw()` → `setNeedsDisplay()` or `CADisplayLink` tick
- `begin_drag_move()` → no-op
- `titlebar_height()` → 0.0 (no titlebar on iOS)
- Window management → mostly no-op (iOS manages window lifecycle)

**Key dependencies:**
```toml
[target.'cfg(target_os = "ios")'.dependencies]
objc2 = "0.6"
objc2-foundation = "0.3"
objc2-ui-kit = { version = "0.3", features = [
    "UIApplication", "UIWindow", "UIViewController", "UIView",
    "UIScreen", "UIPasteboard", "UIGestureRecognizer",
    "UIDocumentPickerViewController",
] }
```

### 2.2 Event Translation

iOS is touch-first. Map touch events to the existing mouse event model:

| iOS event | mozui event |
|---|---|
| `touchesBegan` | `MouseDown` (first touch position) |
| `touchesMoved` | `MouseMoved` |
| `touchesEnded` | `MouseUp` |
| `UIPinchGestureRecognizer` | `ScrollWheel` (zoom) |
| Keyboard (external) | `KeyDown` / `KeyUp` |
| `viewDidLayoutSubviews` | `Resized` |
| `becomeFirstResponder` | `Focused` |

Long-press → `RightClick` (context menu) is a reasonable mapping.

No hover support on iOS — `MouseMoved` only fires during active touches.

### 2.3 Renderer

wgpu supports Metal on iOS natively. Surface creation:
- Get `CAMetalLayer` from `MTKView.layer`
- Use `wgpu::Instance::create_surface(metal_layer)` (safe on iOS)

### 2.4 Text Rendering

cosmic-text works on iOS. System font access:
- Use Core Text via `core-text` crate to enumerate system fonts
- Or bundle fonts like WASM target

### 2.5 Build & Packaging

```bash
# Add iOS targets
rustup target add aarch64-apple-ios aarch64-apple-ios-sim

# Build
cargo build --target aarch64-apple-ios

# Package into .app with Xcode project or cargo-xcode
cargo install cargo-xcode
cargo xcode  # generates Xcode project
```

Requires an Xcode project wrapper with `Info.plist`, launch storyboard,
and provisioning profile for device deployment.

**Estimated effort:** Large (3-4 weeks).
Biggest risks: UIKit lifecycle integration, touch-to-mouse mapping edge
cases, Xcode toolchain setup.

---

## Phase 3: Android Target

### 3.1 Platform Backend (`mozui-platform/src/android/`)

| Trait method | Android equivalent |
|---|---|
| `run(callback)` | `android_activity::AndroidApp` event loop + `Choreographer` |
| `open_window(options)` | `ANativeWindow` from `SurfaceView` |
| `screens()` | `DisplayMetrics` via JNI |
| `set_cursor(cursor)` | `setPointerIcon()` (API 24+, tablets) |
| `clipboard_read()` | `ClipboardManager.getPrimaryClip()` via JNI |
| `clipboard_write(text)` | `ClipboardManager.setPrimaryClip()` via JNI |
| `open_file_dialog()` | `Intent(ACTION_OPEN_DOCUMENT)` via JNI |
| `save_file_dialog()` | `Intent(ACTION_CREATE_DOCUMENT)` via JNI |
| `open_url(url)` | `Intent(ACTION_VIEW, uri)` via JNI |

`PlatformWindow` maps to `ANativeWindow`:
- `content_size()` → `ANativeWindow_getWidth/Height`
- `scale_factor()` → `DisplayMetrics.density`
- `request_redraw()` → `ANativeWindow` invalidation
- Window management → mostly no-op (Android manages activities)

**Key dependencies:**
```toml
[target.'cfg(target_os = "android")'.dependencies]
android-activity = { version = "0.6", features = ["native-activity"] }
ndk = "0.9"
jni = "0.21"
```

### 3.2 Event Translation

Same touch-first model as iOS:

| Android event | mozui event |
|---|---|
| `MotionEvent::Down` | `MouseDown` |
| `MotionEvent::Move` | `MouseMoved` |
| `MotionEvent::Up` | `MouseUp` |
| `KeyEvent` | `KeyDown` / `KeyUp` |
| `onConfigurationChanged` | `Resized` |
| `onWindowFocusChanged` | `Focused` / `Unfocused` |

### 3.3 Renderer

wgpu supports Vulkan on Android. Surface creation:
- Get `ANativeWindow` from `android_activity`
- Use `wgpu::Instance::create_surface(native_window)`
- Fallback: OpenGL ES via `wgpu`'s GLES backend

### 3.4 Text Rendering

cosmic-text works on Android. Font loading:
- System fonts at `/system/fonts/` (read-only)
- Or bundle fonts in the APK `assets/` directory

### 3.5 Build & Packaging

```bash
# Install
cargo install cargo-ndk
rustup target add aarch64-linux-android armv7-linux-androideabi

# Build native library
cargo ndk -t arm64-v8a build --release

# Package into APK with gradle wrapper or xbuild
cargo install xbuild
x build --platform android --arch arm64
```

Requires Android NDK, a minimal `AndroidManifest.xml`, and a Java/Kotlin
activity shim that sets up the `NativeActivity` or `GameActivity`.

**Estimated effort:** Large (3-4 weeks).
Biggest risks: JNI boilerplate for system services, Vulkan driver
compatibility across devices, build toolchain complexity.

---

## Recommended Order

1. **Phase 0** — Refactor platform crate (prerequisite for everything)
2. **Phase 1** — WASM (highest value, best ecosystem support, fastest
   iteration cycle, no app store friction)
3. **Phase 2** — iOS (shares Objective-C tooling knowledge from macOS backend,
   Metal renderer already works)
4. **Phase 3** — Android (most infrastructure overhead due to JNI + NDK +
   Gradle)

## Cross-Cutting Concerns

**Touch input abstraction:** iOS and Android both need touch-to-mouse
mapping. Consider adding a `TouchEvent` variant to `PlatformEvent` and
handling conversion in `mozui-app` rather than duplicating in each platform
backend.

**Font bundling:** WASM, iOS, and Android all have limited or no system font
access compared to desktop. Add a `FontBundle` concept to `mozui-text` that
embeds default fonts at compile time.

**Async clipboard/file dialogs:** Browser clipboard and mobile file pickers
are async. The current sync `clipboard_read() -> Option<String>` API will
need an async variant or callback-based alternative.

**Screen safe areas:** iOS has notch/Dynamic Island safe areas. Android has
navigation bar / cutout insets. `PlatformWindow` may need a
`safe_area_insets() -> Rect` method.

**No hover on mobile:** Components that rely on hover state (tooltips,
hover highlight) need touch-friendly alternatives. This is a UI-layer
concern, not infrastructure.
