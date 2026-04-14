# NATIVE2

## Objective

Replace the current `mozui-native` architecture with a Glass-style native-controls architecture where:

- `mozui` owns the native control substrate and all AppKit/UIKit bridging code.
- Platform backends expose native controls and native window/chrome primitives as first-class framework capabilities.
- `mozui-components` becomes the semantic API layer that can render either custom or native-backed controls, without containing any platform bridging code.
- `mozui-native` is fully retired after migration.

This plan does **not** use SwiftUI as the primary mechanism. Glass's implementation is closer to AppKit/UIKit-native controls integrated directly into the GPUI platform layer. We follow that same architectural direction.

---

## Why We Are Changing Direction

The current `mozui-native` approach is capable of embedding isolated native controls, but it does not make the app feel natively integrated. The primary weaknesses are architectural:

- Native controls are attached ad hoc to raw parent views rather than being owned by the framework as a coherent subsystem.
- Native lifecycle, cleanup, callback routing, and focus semantics are wrapper-local rather than framework-global.
- Toolbar/search/sidebar/popover/panel behavior is not modeled as first-class `Window` capabilities.
- Higher-level UI still reads as custom-rendered UI with some native controls grafted on.
- `mozui-native` uses `objc2` 0.6 with type-safe `define_class!` and `Retained<T>`, but `mozui`'s macOS backend (`macos/native_controls.rs`) still uses the legacy `cocoa` 0.26 / `objc` 0.2 crates with manual `ClassDecl` and raw `msg_send!` calls. This inconsistency creates two separate bridging layers with different safety guarantees.

Glass achieves a stronger native feel because GPUI itself understands:

- Native control state and its lifecycle within the render tree.
- Native toolbar items with full `NSToolbar` integration.
- Native search fields and native focus routing through `NSSearchToolbarItem`.
- Native popup menus, suggestion menus, popovers, and panels.
- Hosted-content composition for sidebars, inspectors, and chrome-adjacent surfaces via `GPUISurfaceView`.
- Callback scheduling back into the framework event loop, never from ObjC delegates directly.

That is the target.

---

## End State

At the end of this migration:

- `crates/mozui-native` no longer exists.
- `mozui` exposes a `PlatformNativeControls` subsystem and window-native APIs.
- `mozui`'s macOS and iOS backends implement those APIs using `objc2` exclusively.
- `mozui-components` owns semantic components and chooses native-backed rendering where appropriate, with zero platform bridging code.
- App code does not manually install native controls by reaching around the framework.
- Native and custom rendering are swappable at the semantic component layer.

---

## Current Status

### Completed

- **Phase 1**: `NativeControlState` and `PlatformNativeControls` trait in `crates/mozui/src/platform/native_controls.rs`. Config structs for button, switch, slider, progress, text field. `MacNativeControls` stub in `crates/mozui/src/platform/macos/native_controls.rs`.
- **Phase 2**: Leaf native element wrappers in `crates/mozui/src/elements/` — `native_button.rs`, `native_switch.rs`, `native_slider.rs`, `native_progress.rs`, `native_text_field.rs`.
- **Phase 3**: Native window chrome contracts in `crates/mozui/src/platform/native_window.rs` — `NativeToolbar`, `NativeToolbarItem`, `NativePopoverHandle`, `NativeSheetHandle`, `schedule_no_args`/`schedule_value` for callback routing.
- **Phase 5** (partial): `mozui-native::toolbar`, toolbar-search integration, sidebar hosting, inspector hosting, popover and sheet presentation now route through core `mozui` window-native APIs.
- **Phase 6** (partial): Semantic `.native()` rendering paths in `mozui-components` for `Button`, single-line `Input`, `Switch`, horizontal `Slider`, and `Progress`.

### In Progress / Incomplete

- `macos/native_controls.rs` still uses `cocoa`/`objc` (legacy). Must migrate to `objc2`.
- Phase 4 (hosted-surface lifecycle) is provisional. Scaffolding exists; runtime validation of resize, z-ordering, deallocation, and event routing is incomplete.
- `mozui-native/src/symbol.rs` has no port target in `mozui/src/elements/`.
- Eleven `mozui-native` files have no migration plan: `tab_view.rs`, `alert.rs`, `breadcrumb.rs`, `color_picker.rs`, `menu.rs`, `picker.rs`, `share.rs`, `date_picker.rs`, `drag_drop.rs`, `stepper.rs`, `file_dialog.rs`. Decisions required for each.
- iOS backend has no `native_controls.rs` implementation.
- `Select`/`ComboBox`, `Table`, sidebar affordances not yet covered in Phase 6.

---

## Design Principles

1. Native infrastructure lives in `mozui`, not in component wrappers.
2. `mozui-components` owns semantic controls, not platform bridging code.
3. Native controls must route events through the `mozui` event loop instead of directly mutating app state from ObjC delegates.
4. Window chrome is part of the platform abstraction.
5. Native composition must support mixed custom/native surfaces.
6. macOS and iOS share the same Rust-side contracts even when platform implementations diverge.
7. Native rendering remains opt-in at the semantic layer.
8. All new macOS and iOS bridging uses `objc2` exclusively. No new `cocoa`/`objc` 0.2 code.

---

## objc2 Integration Patterns

This section documents the exact `objc2` patterns that all new and migrated native control code must follow. `mozui-native` uses these patterns correctly. `mozui`'s macOS backend must be migrated to match.

### Crate versions

```toml
# Workspace Cargo.toml
objc2 = "0.6"
objc2-foundation = "0.3.2"
objc2-app-kit = "0.3"        # macOS
objc2-ui-kit = "0.3.2"       # iOS
objc2-core-foundation = "0.3.2"  # iOS
```

Do not add new dependencies on `cocoa`, `objc`, or `objc_id`.

### Defining ObjC delegate classes

Use `define_class!` with typed `#[ivars]` rather than `ClassDecl::new` + `add_ivar`:

```rust
use objc2::{define_class, msg_send, AnyThread, MainThreadMarker};
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2_foundation::NSObjectProtocol;
use std::cell::RefCell;
use std::rc::Rc;

type ActionCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

struct ActionTargetIvars {
    callback: ActionCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = ActionTargetIvars]
    #[name = "MozuiActionTarget"]
    struct ActionTarget;

    impl ActionTarget {
        #[unsafe(method(performAction:))]
        fn __perform_action(&self, _sender: &AnyObject) {
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                f();
            }
        }
    }

    unsafe impl NSObjectProtocol for ActionTarget {}
);

impl ActionTarget {
    fn new(callback: ActionCallback, _mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ActionTargetIvars { callback });
        unsafe { msg_send![super(this), init] }
    }
}
```

Key rules:
- `#[unsafe(super(NSObject))]` declares the superclass.
- `#[ivars = StructName]` replaces `add_ivar` calls.
- `#[name = "..."]` sets the ObjC class name. Use `Mozui`-prefixed names to avoid conflicts.
- `#[unsafe(method(selector:))]` replaces `add_method` with an `extern "C"` fn.
- `unsafe impl NSObjectProtocol for T {}` is required.
- Allocation uses `Self::alloc().set_ivars(ivars)`, not `msg_send![cls, alloc]` + `set_ivar`.

### Memory management

Use `Retained<T>` for all ObjC reference-counted objects. Do not manually call `retain`/`release`.

```rust
// Allocate and initialize
let button = NSButton::buttonWithTitle_target_action(&ns_title, None, None, mtm);
// button: Retained<NSButton>

// Cast when needed for generic storage
let view: Retained<NSView> = unsafe { Retained::cast_unchecked(button) };

// Recover typed pointer for updates
let platform_button: &NSButton = unsafe { &*(view.as_ptr() as *const NSButton) };
```

Do not use explicit `autoreleasepool` unless a specific Apple framework API requires it. `Retained<T>` handles reference counting automatically.

### Thread safety via MainThreadMarker

All ObjC allocations and UI mutations must occur on the main thread. Enforce this with `MainThreadMarker`:

```rust
// Obtain from framework context (preferred)
fn update_button(&self, ..., window: &Window) {
    let mtm = window.main_thread_marker();
    let button = NSButton::buttonWithTitle_target_action(&title, None, None, mtm);
    // ...
}

// When outside framework context (e.g. element prepaint on main thread)
let mtm = unsafe { MainThreadMarker::new_unchecked() };
```

`MainThreadMarker::new_unchecked()` is safe only when called from the main thread. Do not pass it across threads. Do not store it.

### Target/action callback pattern

All interactive controls use the same target/action pattern. Store closures in `Rc<RefCell<Option<Box<dyn Fn(...)>>>>`:

- `Rc` allows shared ownership between Rust and the ObjC delegate.
- `RefCell` allows interior mutation for per-frame callback replacement.
- `Option` allows clearing the callback without dropping the delegate.

The callback is moved from the element config into the delegate on each prepaint:

```rust
// In element prepaint
let callback: ActionCallback = Rc::new(RefCell::new(config.on_click.take()));
let target = ActionTarget::new(callback, mtm);

unsafe {
    NSControl::setTarget(&button, Some(&target));
    NSControl::setAction(&button, Some(sel!(performAction:)));
}
```

Callbacks invoked from ObjC must never directly mutate `mozui` state. They must schedule through `NativeCallbackDispatcher` (see Callback Scheduling below).

### Coordinate system

AppKit uses a flipped coordinate system (origin bottom-left). mozui uses top-left origin. Convert in `bounds_to_ns_rect`:

```rust
fn bounds_to_ns_rect(bounds: Bounds<Pixels>, parent_height: f64) -> NSRect {
    let flipped_y = parent_height - bounds.origin.y.0 as f64 - bounds.size.height.0 as f64;
    NSRect::new(
        NSPoint::new(bounds.origin.x.0 as f64, flipped_y),
        NSSize::new(
            (bounds.size.width.0 as f64).max(1.0),
            (bounds.size.height.0 as f64).max(1.0),
        ),
    )
}
```

Use `NSFlippedView` subclasses where you need the parent view to provide a top-left coordinate space.

### Migrating `macos/native_controls.rs` from legacy to objc2

The current `MacNativeControls` implementation in `crates/mozui/src/platform/macos/native_controls.rs` uses `cocoa`/`objc` 0.2 with `ClassDecl`, raw `id` pointers, and `msg_send!`. This must be replaced with `objc2` following the patterns above.

Migration steps for each control:

1. Replace `ClassDecl::new(...)` + `decl.add_ivar(...)` + `decl.register()` with `define_class!` with `#[ivars]`.
2. Replace `extern "C" fn perform(this: &Object, ...) { (*this).get_ivar(...) }` with a typed `#[unsafe(method(...))]` impl block.
3. Replace `let target: id = msg_send![cls, alloc]; msg_send![target, init]` with `Target::alloc().set_ivars(...); msg_send![super(this), init]`.
4. Replace manual `drop(Box::from_raw(...))` in `dealloc` with `Retained<T>` drop semantics. objc2 handles `dealloc` automatically for `define_class!` types.
5. Replace `let view: id = msg_send![class!(NSButton), alloc]` with `NSButton::buttonWithTitle_target_action(...)`.
6. Replace `msg_send![view, setTarget: target]` with `NSControl::setTarget(&view, Some(&target))`.
7. Replace `msg_send![view, setState: 1isize]` with `NSControl::setState(&view, NSControlStateValue::On)`.

The `NativeControlState` struct stores `*mut c_void` pointers and a cleanup fn. After migration, the cleanup fn calls `drop(Retained::from_raw(...))` instead of manual `release`.

---

## Callback Scheduling

Native ObjC delegates must never directly mutate `mozui` state. They enqueue work onto the framework's next-frame queue via `NativeCallbackDispatcher`.

The infrastructure is already in `crates/mozui/src/platform/native_window.rs`:

```rust
fn schedule_no_args(
    handler: Rc<dyn Fn(&mut Window, &mut App)>,
    dispatcher: NativeCallbackDispatcher,
) -> Box<dyn Fn()> {
    Box::new(move || {
        let handler = handler.clone();
        let callback: FrameCallback = Box::new(move |window, cx| {
            handler(window, cx);
        });
        dispatcher.dispatch(callback);
    })
}

fn schedule_value<P: 'static>(
    handler: Rc<dyn Fn(P, &mut Window, &mut App)>,
    dispatcher: NativeCallbackDispatcher,
) -> Box<dyn Fn(P)> {
    Box::new(move |value| {
        let handler = handler.clone();
        let callback: FrameCallback = Box::new(move |window, cx| {
            handler(value, window, cx);
        });
        dispatcher.dispatch(callback);
    })
}
```

Element helpers in `native_element_helpers.rs` wrap these for use in element prepaint. All new controls must use this pattern. Direct state mutation from ObjC callbacks is never permitted.

This is the key architectural difference from `mozui-native` wrappers, which hold wrapper-local `Rc<RefCell<Option<>>>` closures and invoke them synchronously during AppKit events.

---

## Proposed Architecture

### 1. Native Control Substrate in `mozui`

Core module structure:

```
crates/mozui/src/platform/native_controls.rs     — NativeControlState, config structs, PlatformNativeControls trait
crates/mozui/src/platform/native_window.rs       — window chrome contracts, callback scheduling
crates/mozui/src/platform/macos/native_controls.rs  — MacNativeControls impl (migrate to objc2)
crates/mozui/src/platform/ios/native_controls.rs    — iOSNativeControls impl (not yet created)
crates/mozui/src/elements/native_button.rs
crates/mozui/src/elements/native_search_field.rs    (not yet created)
crates/mozui/src/elements/native_image_view.rs      (not yet created)
crates/mozui/src/elements/native_tracking_view.rs   (not yet created)
crates/mozui/src/elements/native_switch.rs
crates/mozui/src/elements/native_slider.rs
crates/mozui/src/elements/native_progress.rs
crates/mozui/src/elements/native_text_field.rs
crates/mozui/src/elements/native_visual_effect.rs   (not yet created)
crates/mozui/src/elements/native_glass_effect.rs    (not yet created)
crates/mozui/src/elements/native_combo_box.rs       (not yet created)
crates/mozui/src/elements/native_toggle_group.rs    (not yet created)
crates/mozui/src/elements/native_table.rs           (not yet created)
crates/mozui/src/elements/native_sidebar.rs         (not yet created)
```

`NativeControlState` (already implemented) is the opaque per-control state stored across frames:

```rust
pub struct NativeControlState {
    view: *mut c_void,     // Retained<NSView> / Retained<UIView> stored as raw
    target: *mut c_void,   // Retained<delegate> stored as raw
    cleanup: unsafe fn(*mut c_void, *mut c_void),
}
```

After migrating to `objc2`, cleanup fns use `Retained::from_raw` rather than manual `release`:

```rust
unsafe fn cleanup_view_and_target(view: *mut c_void, target: *mut c_void) {
    if !target.is_null() {
        drop(Retained::<NSObject>::from_raw(target as *mut NSObject));
    }
    if !view.is_null() {
        let view_obj = Retained::<NSView>::from_raw(view as *mut NSView).unwrap();
        view_obj.removeFromSuperview();
        drop(view_obj);
    }
}
```

`PlatformNativeControls` (already implemented with no-op defaults) grows with additional controls as they are ported:

```rust
pub trait PlatformNativeControls {
    fn update_button(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: ButtonConfig<'_>) {}
    fn update_switch(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: SwitchConfig) {}
    fn update_slider(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: SliderConfig) {}
    fn update_progress(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: ProgressConfig) {}
    fn update_text_field(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: TextFieldConfig<'_>) {}
    fn update_search_field(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: SearchFieldConfig<'_>) {}
    fn update_image_view(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: ImageViewConfig<'_>) {}
    fn update_tracking_view(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: TrackingViewConfig) {}
    fn update_visual_effect(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: VisualEffectConfig) {}
    fn update_glass_effect(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: GlassEffectConfig) {}
    fn update_combo_box(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: ComboBoxConfig<'_>) {}
    fn update_toggle_group(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: ToggleGroupConfig<'_>) {}
    fn update_table(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: TableConfig<'_>) {}
    fn update_outline(&self, state: &mut NativeControlState, parent: *mut c_void, bounds: Bounds<Pixels>, scale: f32, config: OutlineConfig<'_>) {}
}
```

### 2. Window-Native APIs in `mozui`

`native_window.rs` (already partially implemented) must cover:

```rust
// Toolbar
fn set_native_toolbar(toolbar: NativeToolbar) -> Result<()>

// Search
fn focus_native_search_field()
fn blur_native_field_editor()

// Popup / suggestion menus
fn show_native_popup_menu(items: Vec<NativeMenuItem>, anchor: Bounds<Pixels>, on_select: impl Fn(usize))
fn show_native_search_suggestion_menu(items: Vec<NativeSearchSuggestion>, anchor: Bounds<Pixels>)
fn update_native_search_suggestion_menu(items: Vec<NativeSearchSuggestion>)
fn dismiss_native_search_suggestion_menu()

// Popover
fn show_native_popover(config: NativePopoverConfig) -> NativePopoverHandle
fn dismiss_native_popover(handle: NativePopoverHandle)

// Panel / sheet
fn show_native_panel(config: NativePanelConfig) -> NativePanelHandle
fn dismiss_native_panel(handle: NativePanelHandle)
fn show_native_sheet(config: NativeSheetConfig) -> NativeSheetHandle
fn dismiss_native_sheet(handle: NativeSheetHandle)

// Hosted content
fn configure_hosted_content(config: HostedContentConfig)
fn attach_hosted_surface(target: HostedContentTarget, surface: Box<dyn PlatformSurface>)
```

These belong on `PlatformWindow` (or a `PlatformNativeWindow` extension trait) and are exposed through `Window`.

### 3. Glass-Style NSGlassEffectView Support

Glass uses `NSGlassEffectView` available on macOS 26+. The existing `mozui-native::glass_effect.rs` handles this with a runtime class lookup and fallback to `NSVisualEffectView`:

```rust
// Runtime availability check
let cls = AnyClass::get(c"NSGlassEffectView");
if let Some(cls) = cls {
    // macOS 26+: use NSGlassEffectView
    let view: *mut AnyObject = unsafe { msg_send![cls, alloc] };
    // ...
} else {
    // Fallback: NSVisualEffectView with appropriate material
}
```

Port this pattern directly into `crates/mozui/src/elements/native_glass_effect.rs` and `crates/mozui/src/platform/macos/native_controls.rs::update_glass_effect`.

### 4. Hosted Content / Surface Composition

This is the mechanism that allows native platform containers (sidebar, inspector, toolbar, popover) to host `mozui` rendering surfaces while keeping native chrome behavior.

Glass implements this via `GPUISurfaceView` — a `NSView` subclass with a `CAMetalLayer` backing that acts as a rendering target for a secondary `mozui` surface. The key elements:

- `PlatformSurface` trait: exposes `native_view() -> *mut c_void` so a `GPUISurfaceView` can be embedded in a native container.
- `HostedContentConfig`: targets a specific pane (`Sidebar`, `SidebarHeader`, `Inspector`, `Toolbar`, `Popover`) and associates a `PlatformSurface`.
- Window's `attach_hosted_surface(target, surface)` manages view hierarchy insertion, size synchronization, and teardown.
- Scale factor and bounds are synchronized between the native container and the `mozui` surface on every resize.

`mozui`'s equivalent must:

1. Define `PlatformSurface` with `native_view() -> *mut c_void`.
2. Define `HostedContentTarget` enum (Sidebar, Inspector, ToolbarItem, Popover).
3. On macOS: create an `NSView` subclass with `CAMetalLayer` that can be embedded in `NSSplitViewController`, `NSPanel`, `NSPopover`, or `NSToolbar` item views.
4. Route all events from the native container through the `mozui` event loop rather than relying on `NSResponder` chain directly.
5. Validate that hosted surfaces survive: window resize, show/hide cycles, repeated open/close, and deallocation without leaking delegates, retained views, or Metal layer state.

Current status: scaffolding exists in `mozui` core. Runtime validation is incomplete. Do not declare Phase 4 done without live demo proof.

### 5. Unaddressed `mozui-native` Files

These eleven files have no migration plan:

| File | Recommendation |
|---|---|
| `tab_view.rs` | Port `NSTabView` config to `update_tab_view` in `PlatformNativeControls` |
| `alert.rs` | Move to `Window::show_native_alert(...)`. Uses legacy `cocoa`; migrate to `objc2`. |
| `breadcrumb.rs` | macOS 12+ `NSBreadcrumbBarItem`. Port to element or drop if unused. |
| `color_picker.rs` | Port `NSColorWell` config to `update_color_well` in `PlatformNativeControls` |
| `menu.rs` | Move to `Window::show_native_popup_menu(...)`. |
| `picker.rs` | Port `NSPopUpButton` config to `update_combo_box` or `update_picker`. |
| `share.rs` | Move to `Window::show_native_share_sheet(...)`. |
| `date_picker.rs` | Port `NSDatePicker` config to `update_date_picker` in `PlatformNativeControls`. |
| `drag_drop.rs` | Integrate with `mozui` drag/drop event routing rather than as a standalone control. |
| `stepper.rs` | Port `NSStepper` config to `update_stepper` in `PlatformNativeControls`. |
| `file_dialog.rs` | Move to `App::open_file_dialog(...)` / `App::save_file_dialog(...)`. |

For each: decide during Phase 2 extension whether it is a leaf control (add to `PlatformNativeControls`), a window-level API (add to `native_window.rs`), or out of scope (delete and document).

`symbol.rs` is also unported. Port `NSImageView` + SF Symbols to `crates/mozui/src/elements/native_image_view.rs`.

---

## Recommended Public API

### Semantic controls in `mozui-components`

No platform code in component crate. Only render-path selection:

```rust
Button::new("save")
    .label("Save")
    .native()

SearchInput::new("query")
    .native()

Switch::new("notify")
    .checked(true)
    .native()

Slider::new("volume")
    .range(0.0..=1.0)
    .native()

Select::new("country")
    .options(countries)
    .native()
```

Each component branches internally between the custom renderer and `mozui::elements::native_*` primitives. Do not create a parallel component library.

### Window chrome — not component modifiers

These belong at the `Window` layer, not in `mozui-components`:

```rust
window.set_native_toolbar(
    NativeToolbar::new()
        .item(NativeToolbarItem::button("new", "New"))
        .item(NativeToolbarItem::search("query"))
        .item(NativeToolbarItem::flexible_space())
)

window.focus_native_search_field()

window.show_native_popup_menu(items, anchor, on_select)

window.show_native_popover(
    NativePopoverConfig::new()
        .anchor(anchor_bounds)
        .content(|window, cx| { /* mozui content */ })
)
```

Toolbar, omnibox, search suggestion menu, sidebar chrome, inspector chrome, native tab strip, and native popover/panel/sheet are all window-layer concerns.

### Search fields

Content search fields can expose `.native()` as a component modifier. Toolbar search fields should be modeled as a `NativeToolbarItem::search(...)` rather than a component modifier, because the toolbar owns focus routing, suggestion menu anchoring, and keyboard dismiss behavior.

---

## Migration Phases

### Phase 0: Freeze Architecture ✓

- `mozui-native` will be retired.
- Native substrate moves into `mozui`.
- `mozui-components` stays semantic.
- AppKit/UIKit integration (not SwiftUI) is the primary path.

### Phase 1: Core Native Contracts ✓

Delivered:
- `NativeControlState` in `crates/mozui/src/platform/native_controls.rs`
- `PlatformNativeControls` trait
- `MacNativeControls` stub
- Config structs for button, switch, slider, progress, text field
- `crates/mozui/src/platform/native_window.rs` with callback scheduling primitives

### Phase 2: Leaf Controls in `mozui`

#### 2a: Migrate `macos/native_controls.rs` to objc2

Replace all `cocoa`/`objc` 0.2 code in `MacNativeControls` with `objc2` patterns. This is prerequisite for Phase 7 — the codebase cannot have two bridging layers at retirement.

Work:
- Replace `ClassDecl`-based targets with `define_class!` targets.
- Replace raw `id` pointer APIs with `objc2-app-kit` typed methods.
- Replace manual `retain`/`release` in cleanup fns with `Retained::from_raw` drop.
- Use `NSControl::setTarget()`, `NSControl::setAction()` typed methods.
- Verify `cargo check -p mozui` and `cargo check -p mozui --target aarch64-apple-ios-sim` remain clean.

Exit criteria:
- `macos/native_controls.rs` has zero `cocoa`/`objc` imports.
- All five implemented controls (button, switch, slider, progress, text field) compile and function correctly.

#### 2b: Add remaining leaf controls

Port from `mozui-native`:
- `native_image_view.rs` — `NSImageView` + SF Symbols (from `symbol.rs`)
- `native_visual_effect.rs` — `NSVisualEffectView` (from `visual_effect.rs`)
- `native_glass_effect.rs` — `NSGlassEffectView` + fallback (from `glass_effect.rs`)
- `native_tracking_view.rs` — `NSTrackingArea` for hover correctness
- `native_search_field.rs` — standalone content `NSSearchField` (separate from toolbar search item)
- `native_combo_box.rs` — `NSPopUpButton` / `NSComboBox`
- `native_stepper.rs` — `NSStepper`
- `native_date_picker.rs` — `NSDatePicker`
- `native_color_well.rs` — `NSColorWell`
- `native_tab_view.rs` — `NSTabView`

Port implementation ideas, not file structure. Convert wrapper-local callbacks to `NativeCallbackDispatcher`-scheduled callbacks. Unify state under `NativeControlState`.

Exit criteria:
- All listed elements in `crates/mozui/src/elements/`.
- `mozui-native` wrappers either delegate to them or are removed behind temporary shims.

### Phase 3: Native Window Chrome APIs ✓ (partial)

Delivered:
- `NativeToolbar`, `NativeToolbarItem` model
- `NativePopoverHandle`, `NativeSheetHandle`
- `schedule_no_args` / `schedule_value` callback helpers

Remaining:
- macOS backend implementations for `set_native_toolbar`, `focus_native_search_field`, `show_native_popup_menu`, `show_native_search_suggestion_menu`, `update_native_search_suggestion_menu`.
- macOS `NSSearchToolbarItem` integration (toolbar-embedded search field with native focus routing).
- Verify suggestion menu anchors to both toolbar and content search targets.

Exit criteria:
- Sample app creates native toolbar items via `Window`.
- Native search field focus routes through `PlatformWindow`.
- Popup and suggestion menus anchor to toolbar or content targets.

### Phase 4: Hosted Content / Surface Infrastructure

Define `PlatformSurface` and `HostedContentTarget`. Implement `attach_hosted_surface` on `PlatformWindow` for macOS.

macOS implementation:
- Create a `MozuiSurfaceView: NSView` subclass with `CAMetalLayer` backing.
- Expose `native_view() -> *mut NSView` from the surface.
- Handle resize notifications: synchronize surface bounds with native container bounds.
- Forward events: route `mouseDown`, `keyDown`, etc. from `MozuiSurfaceView` into `mozui` event loop.
- Deallocation: ensure cleanup of `CAMetalLayer`, Metal device references, and any registered observers when surface is dropped.

Runtime verification targets (required before phase exit):
- Hosted surface survives window resize without layout corruption.
- Show/hide cycle does not leak views.
- Repeated popover open/close does not accumulate retained delegates or Metal layers.
- Focus transitions between native container and hosted surface are correct.
- Z-ordering between native chrome and custom surface is stable.

Exit criteria:
- Custom `mozui` content can be embedded in native platform containers with controlled lifecycle.
- Live demo proves composition, sizing, z-ordering, and deallocation.

### Phase 5: Rebuild Sidebar / Inspector / Toolbar Chrome ✓ (partial)

Delivered:
- `mozui-native::toolbar`, toolbar-search integration, sidebar hosting, inspector hosting, popover and sheet presentation now route through `mozui` core window-native APIs.
- Standalone `create_search_field(...)` in `mozui-native::search` is now a compatibility leaf, not the primary architecture.

Remaining:
- Replace remaining ad hoc install helpers with Phase 3/4 framework-native versions.
- Confirm no app code bypasses `Window` to reach AppKit directly.

Exit criteria:
- Browser/finder-style demo can be rebuilt without `mozui-native`.

### Phase 6: Semantic Native Rendering in `mozui-components` (partial)

Delivered: `Button`, single-line `Input`, `Switch`, horizontal `Slider`, `Progress`.

Remaining (in recommended order):
1. `Select` / `ComboBox` — requires `native_combo_box` from Phase 2b.
2. `Table` — requires `native_table` / `native_outline` from Phase 2b.
3. `Sidebar` affordances where appropriate.

API constraints:
- Zero platform-specific code in `mozui-components`.
- Only semantic render-path decisions.
- Custom rendering always available as fallback.

Exit criteria:
- Common form controls support native-backed rendering.
- `Select`/`ComboBox` has a `.native()` path.

### Phase 7: Retire `mozui-native`

Steps:
1. Remove all `mozui-native` dependencies from workspace members.
2. Delete re-exports in crate root.
3. Migrate demos and examples to use `mozui` and `mozui-components` directly.
4. Update docs.
5. Delete `crates/mozui-native` from workspace.

Prerequisites:
- Phase 2 objc2 migration complete.
- All files in the unaddressed list resolved (ported or explicitly dropped).
- Phase 4 hosted surface runtime validation complete.

Exit criteria:
- `cargo build --workspace` succeeds without `crates/mozui-native`.
- All native functionality lives in `mozui`.

---

## Platform Backend Responsibilities

### macOS

- Implement `PlatformNativeControls` in full using `objc2`.
- Implement `NSToolbar` + `NSToolbarDelegate` for native toolbar items.
- Implement `NSSearchToolbarItem` and standalone `NSSearchField` for content search.
- Implement `NSMenu` for native popup menus.
- Implement `NSPopover` for native popovers with hosted-content support.
- Implement `NSPanel` for panels.
- Implement `NSSplitViewController` for sidebar/inspector chrome.
- Implement `NSVisualEffectView` and `NSGlassEffectView` (macOS 26+ with fallback).
- Implement `NSTrackingArea` integration for hover-correct regions.
- Implement `MozuiSurfaceView` for hosted-content composition.

### iOS

- Implement `PlatformNativeControls` for `UIKit` leaf controls.
- Route all events inside `mozui`; no wrapper-local ownership.
- Leaf control parity with macOS: button, switch, slider, progress, text field, search field.
- Hosted-content composition where justified (e.g. `UINavigationController` integration).
- Intentionally lag macOS on sidebar/inspector/chrome richness.

---

## File-Level Migration Map

### Delete / Replace

| File | Destination |
|---|---|
| `mozui-native/src/native_view.rs` | Replaced by `NativeControlState` + `objc2` backend attach logic |
| `mozui-native/src/button.rs` | `mozui/src/elements/native_button.rs` (done) |
| `mozui-native/src/slider.rs` | `mozui/src/elements/native_slider.rs` (done) |
| `mozui-native/src/progress.rs` | `mozui/src/elements/native_progress.rs` (done) |
| `mozui-native/src/switch.rs` | `mozui/src/elements/native_switch.rs` (done) |
| `mozui-native/src/text_field.rs` | `mozui/src/elements/native_text_field.rs` (done) |
| `mozui-native/src/symbol.rs` | `mozui/src/elements/native_image_view.rs` (pending) |
| `mozui-native/src/visual_effect.rs` | `mozui/src/elements/native_visual_effect.rs` (pending) |
| `mozui-native/src/glass_effect.rs` | `mozui/src/elements/native_glass_effect.rs` (pending) |
| `mozui-native/src/search.rs` | Core window-native search APIs + `native_search_field.rs` element |
| `mozui-native/src/toolbar.rs` | `Window::set_native_toolbar(...)` |
| `mozui-native/src/sidebar.rs` | `Window::attach_hosted_surface(Sidebar, ...)` |
| `mozui-native/src/popover.rs` | `Window::show_native_popover(...)` |
| `mozui-native/src/sheet.rs` | `Window::show_native_sheet(...)` |
| `mozui-native/src/inspector.rs` | `Window::attach_hosted_surface(Inspector, ...)` |
| `mozui-native/src/stepper.rs` | `mozui/src/elements/native_stepper.rs` (pending) |
| `mozui-native/src/date_picker.rs` | `mozui/src/elements/native_date_picker.rs` (pending) |
| `mozui-native/src/color_picker.rs` | `mozui/src/elements/native_color_well.rs` (pending) |
| `mozui-native/src/picker.rs` | `mozui/src/elements/native_combo_box.rs` (pending) |
| `mozui-native/src/tab_view.rs` | `mozui/src/elements/native_tab_view.rs` (pending) |
| `mozui-native/src/alert.rs` | `Window::show_native_alert(...)` |
| `mozui-native/src/menu.rs` | `Window::show_native_popup_menu(...)` |
| `mozui-native/src/share.rs` | `Window::show_native_share_sheet(...)` |
| `mozui-native/src/file_dialog.rs` | `App::open_file_dialog(...)` / `App::save_file_dialog(...)` |
| `mozui-native/src/breadcrumb.rs` | Evaluate: port or drop |
| `mozui-native/src/drag_drop.rs` | Integrate with `mozui` drag/drop event system |
| `mozui-native/src/table.rs` | `mozui/src/elements/native_table.rs` (pending) |

### Add / Expand

```
crates/mozui/src/platform/native_controls.rs     — extend PlatformNativeControls, add missing config structs
crates/mozui/src/platform/native_window.rs       — add remaining window-chrome API types
crates/mozui/src/platform/macos/native_controls.rs  — migrate to objc2, add remaining controls
crates/mozui/src/platform/ios/native_controls.rs    — create from scratch
crates/mozui/src/elements/native_image_view.rs
crates/mozui/src/elements/native_search_field.rs
crates/mozui/src/elements/native_tracking_view.rs
crates/mozui/src/elements/native_visual_effect.rs
crates/mozui/src/elements/native_glass_effect.rs
crates/mozui/src/elements/native_combo_box.rs
crates/mozui/src/elements/native_stepper.rs
crates/mozui/src/elements/native_date_picker.rs
crates/mozui/src/elements/native_color_well.rs
crates/mozui/src/elements/native_tab_view.rs
crates/mozui/src/elements/native_table.rs
crates/mozui/src/elements/native_sidebar.rs
crates/mozui-components/src/select/...          — native combo box path
crates/mozui-components/src/table/...           — native table path
```

---

## Testing Plan

### Compile Tests

```sh
cargo check -p mozui
cargo check -p mozui-components
cargo check -p mozui --target aarch64-apple-ios-sim
cargo check -p mozui-ios-demo --target aarch64-apple-ios-sim
```

After Phase 2a migration, add:

```sh
cargo check -p mozui 2>&1 | grep -c "cocoa\|use objc::" | xargs test 0 -eq
```

Verify zero legacy `cocoa`/`objc` 0.2 imports in `crates/mozui/`.

### Runtime Verification — macOS

- Toolbar search field: focus, dismiss, suggestion menu anchoring, keyboard cancel.
- Native button, switch, slider, text field: interaction, value callbacks, focus.
- Sidebar + inspector hosted-content composition: render, resize, show/hide, deallocation.
- Popover: open, close, repeated cycles, focus routing.
- Panel/sheet: present, dismiss, z-ordering.
- Host containers: resize, focus changes, visibility toggles, teardown without leaks.
- `NSGlassEffectView` on macOS 26+, fallback to `NSVisualEffectView` on earlier.

### Runtime Verification — iOS

- Button, switch, slider, progress, text/search: interaction, value callbacks.
- Focus + text input routing through `mozui` event loop.
- Scroll + gesture coexistence with hosted native controls.

### Regression Areas

- Focus handling across native and custom surfaces.
- Keyboard shortcuts with first-responder in native control.
- Hit testing at native/custom surface boundaries.
- Z-ordering between native views and Metal-rendered `mozui` surfaces.
- Live layout / resize.
- Cleanup and deallocation: no retained `NSView`, no leaked `CAMetalLayer`, no zombie delegates.
- Async callback safety: no state mutation after window teardown.

---

## Risk Register

### 1. Native / Custom Surface Z-Ordering

Risk: native `NSView`s do not clip, overlap, or animate consistently with Metal-backed `mozui` surfaces.

Mitigation:
- Use hosted-surface architecture explicitly rather than raw subview placement.
- Document which overlap combinations are supported.
- Replace raw host-view escape hatches with managed `attach_hosted_surface` before declaring Phase 4 complete.
- Validate every combination in runtime demo.

### 2. Event Re-entrancy

Risk: platform delegates invoke state changes during paint or layout.

Mitigation:
- Schedule via `NativeCallbackDispatcher` only.
- Never mutate `mozui` state directly from ObjC target/action callbacks.
- Treat compile success as insufficient — require runtime validation of toolbar/search/popover/sidebar flows.

### 3. objc2 Migration Breakage

Risk: migrating `macos/native_controls.rs` from legacy `cocoa`/`objc` introduces runtime crashes due to incorrect memory management.

Mitigation:
- Migrate one control at a time with runtime verification after each.
- Pay attention to `Retained::from_raw` ownership semantics — only call on pointers that have +1 retain count (i.e. owned pointers, not borrowed views).
- Use `autoreleasepool` around blocks that create many short-lived `NSString` objects if profiling shows pressure.

### 4. API Sprawl in `Window`

Risk: too many one-off native APIs accumulate on `Window`.

Mitigation:
- Group APIs around reusable primitives (`set_native_toolbar`, `show_native_popover`, `attach_hosted_surface`).
- No app-specific APIs.
- Review for overlap before adding each new API.

### 5. `mozui-components` Bloat

Risk: semantic components become cluttered with rendering variants.

Mitigation:
- Add native appearance only to controls that materially benefit.
- Centralize appearance selection in a shared `ControlAppearance` enum.
- Keep fallback custom rendering always available.

### 6. Incomplete iOS Parity

Risk: architecture becomes macOS-first; iOS remains permanently partial.

Mitigation:
- Require shared Rust-side contracts first (done in Phase 1).
- Allow backend implementation depth to differ.
- Create `platform/ios/native_controls.rs` stub in Phase 2a so iOS compiles with no-op impls.

---

## What We Should Not Do

- Do not keep `mozui-native` as a second-class layer with most logic still there.
- Do not move AppKit/UIKit/ObjC bridging into `mozui-components`.
- Do not treat toolbar/sidebar/search/inspector as leaf widgets or simple component modifiers.
- Do not rely on SwiftUI hosting as the primary strategy.
- Do not directly invoke application state changes from native delegates.
- Do not write new `cocoa`/`objc` 0.2 code anywhere in `mozui`.
- Do not call `MainThreadMarker::new_unchecked()` outside the main thread or store it past the current call frame.
- Do not use `Retained::cast_unchecked` across unrelated class hierarchies without a runtime `isKindOfClass:` check.

---

## Recommended Execution Order

1. Migrate `macos/native_controls.rs` from `cocoa`/`objc` to `objc2` (Phase 2a blocker for Phase 7).
2. Create `platform/ios/native_controls.rs` stub so iOS compiles clean.
3. Port remaining leaf controls from `mozui-native` into `mozui/src/elements/` (Phase 2b).
4. Add missing macOS backend for Phase 3 APIs (toolbar item creation, search focus, popup menus).
5. Implement `PlatformSurface` + hosted-content infrastructure; prove with live demo (Phase 4).
6. Migrate remaining `mozui-native` window-chrome to Phase 3/4 APIs (Phase 5 completion).
7. Add `Select`/`ComboBox` and `Table` semantic native paths in `mozui-components` (Phase 6 completion).
8. Resolve all unaddressed `mozui-native` files per migration table.
9. Migrate demos and examples.
10. Delete `crates/mozui-native`.

---

## Definition of Done

The migration is done when all of the following are true:

- `mozui-native` is removed from the workspace.
- `crates/mozui/src/platform/macos/native_controls.rs` uses `objc2` exclusively.
- `crates/mozui/src/platform/ios/native_controls.rs` implements `PlatformNativeControls` for UIKit.
- Native control abstractions for all ported controls live in `mozui/src/elements/`.
- `mozui-components` can opt into native rendering for common semantic controls via `.native()`.
- Native toolbar/search/sidebar/inspector behavior is available through `Window`.
- Native callbacks schedule through the `mozui` event loop, never mutating state from ObjC delegates.
- Hosted-surface composition is validated at runtime for resize, show/hide, deallocation, and event routing.
- macOS demos show a materially more native toolbar/search/sidebar experience.
- iOS demos use the same Rust-side architecture, even with a smaller feature set.

---

## Immediate Next Step

Migrate `crates/mozui/src/platform/macos/native_controls.rs` from `cocoa`/`objc` to `objc2`:

1. Replace the four `ClassDecl`-based target classes (`VoidTarget`, `BoolTarget`, `F64Target`, `TextTarget`) with `define_class!` equivalents following the objc2 patterns in this document.
2. Replace raw `id` allocations with typed `objc2-app-kit` methods.
3. Replace manual `retain`/`release` in cleanup fns with `Retained::from_raw` drop.
4. Verify `cargo check -p mozui` is clean.
5. Do a runtime smoke test of all five controls.

That clears the last legacy bridging from `crates/mozui/` and unblocks clean Phase 7 execution.
