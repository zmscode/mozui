# NATIVE2

## Objective

Replace the current `mozui-native` architecture with a Glass-style native-controls architecture where:

- `mozui` owns the native control substrate.
- platform backends expose native controls and native window/chrome primitives as first-class framework capabilities.
- `mozui-components` becomes the semantic API layer that can render either custom or native-backed controls.
- `mozui-native` is fully retired after migration.

This plan intentionally does **not** use SwiftUI as the primary mechanism. Glass's implementation is closer to AppKit/UIKit-native controls integrated into the GPUI platform layer than to SwiftUI hosting. We should follow that architectural direction.

## Why We Are Changing Direction

The current `mozui-native` approach is good at embedding isolated native controls, but it does not make the app feel natively integrated. The main weaknesses are architectural:

- native controls are attached ad hoc to raw parent views rather than owned by the framework as a coherent subsystem.
- native lifecycle, cleanup, callback routing, and focus semantics are wrapper-local instead of framework-global.
- toolbar/search/sidebar/popover/panel behavior is not modeled as first-class `Window` capabilities.
- higher-level UI still reads as custom-rendered UI with some native controls attached.

Glass gets the stronger native feel because GPUI itself understands:

- native control state
- native toolbar items
- native search fields and native focus routing
- native popup menus / suggestion menus / popovers / panels
- hosted-content composition for sidebars, inspectors, and chrome-adjacent surfaces
- callback scheduling back into the framework event loop

That is the target.

## End State

At the end of this migration:

- `crates/mozui-native` no longer exists.
- `mozui` exposes a `PlatformNativeControls` subsystem and window-native APIs.
- `mozui` macOS and iOS backends implement those APIs.
- `mozui-components` owns the semantic components and can choose native-backed rendering where appropriate.
- app code does not manually install native controls by reaching around the framework.
- native and custom rendering are swappable at the semantic component layer.

## Design Principles

1. Native infrastructure lives in `mozui`, not in component wrappers.
2. `mozui-components` owns semantic controls, not platform bridging code.
3. Native controls must route events through the `mozui` event loop instead of directly mutating app state from ObjC delegates.
4. Window chrome is part of the platform abstraction.
5. Native composition must support mixed custom/native surfaces.
6. macOS and iOS share the same Rust-side contracts even when platform implementations diverge.
7. Native rendering remains opt-in at the semantic layer.

## Proposed Architecture

### 1. Native Control Substrate In `mozui`

Add a new core module family, likely:

- `crates/mozui/src/platform/native_controls.rs`
- `crates/mozui/src/elements/native_button.rs`
- `crates/mozui/src/elements/native_search_field.rs`
- `crates/mozui/src/elements/native_image_view.rs`
- `crates/mozui/src/elements/native_tracking_view.rs`
- `crates/mozui/src/elements/native_switch.rs`
- `crates/mozui/src/elements/native_slider.rs`
- `crates/mozui/src/elements/native_progress.rs`
- `crates/mozui/src/elements/native_text_field.rs`
- `crates/mozui/src/elements/native_sidebar.rs`
- `crates/mozui/src/elements/native_combo_box.rs`
- `crates/mozui/src/elements/native_toggle_group.rs`
- `crates/mozui/src/elements/native_table.rs`
- `crates/mozui/src/elements/native_glass_effect.rs`
- `crates/mozui/src/elements/native_visual_effect.rs`

This layer should define:

- `NativeControlState`
  - opaque platform-owned state
  - native view pointer
  - callback target pointer
  - cleanup function
- per-control config structs
  - `ButtonConfig`
  - `SearchFieldConfig`
  - `TextFieldConfig`
  - `SwitchConfig`
  - `SliderConfig`
  - `ProgressConfig`
  - `ImageViewConfig`
  - `TrackingViewConfig`
  - `SidebarViewConfig`
  - `TableViewConfig`
  - etc.
- `PlatformNativeControls` trait
  - `update_button`
  - `update_search_field`
  - `update_text_field`
  - `update_switch`
  - `update_slider`
  - `update_progress`
  - `update_image_view`
  - `update_tracking_view`
  - `update_sidebar`
  - `update_combo_box`
  - `update_toggle_group`
  - `update_table`
  - `update_outline`
  - `update_visual_effect`
  - `update_glass_effect`
  - attach / remove helpers where needed

This should mirror the role Glass pushed into GPUI: the framework owns the abstraction and the backends fill it in.

### 2. Window-Native APIs In `mozui`

Extend `Window` / `PlatformWindow` so native window/chrome behavior is framework-native:

- `set_native_toolbar(...)`
- `focus_native_search_field(...)`
- `show_native_popup_menu(...)`
- `show_native_search_suggestion_menu(...)`
- `update_native_search_suggestion_menu(...)`
- `dismiss_native_search_suggestion_menu()`
- `show_native_popover(...)`
- `dismiss_native_popover()`
- `show_native_panel(...)`
- `dismiss_native_panel()`
- `blur_native_field_editor()`
- `configure_hosted_content(...)`
- `attach_hosted_surface(...)`

These APIs are mandatory if we want browser/toolbar/sidebar/inspector behavior to feel native instead of improvised.

### 3. Native Callback Scheduling

Add a helper module in `mozui` for routing native callbacks into the main frame/update loop.

Required behavior:

- native delegates/targets never directly mutate UI tree state
- they enqueue framework callbacks onto a next-frame or platform-safe queue
- callbacks execute with `&mut Window` and `&mut App`
- dirty invalidation is triggered centrally

This is one of the key differences from the current `mozui-native` wrappers, which hold wrapper-local closures and invoke them directly.

### 4. Hosted Content / Hosted Surface Composition

We need a first-class mechanism for mixing native containers with custom-rendered surfaces.

Use cases:

- native sidebar shell with custom sidebar content surface
- native inspector shell with custom inspector content surface
- native toolbar item hosting a custom view
- native search suggestion menu anchored to toolbar or content search field
- native popover/panel content that may host a `mozui` surface

This is the mechanism that allows window chrome and platform containers to feel correct while still preserving `mozui` rendering where native controls do not exist or are not wanted.

### 5. Semantic Rendering In `mozui-components`

`mozui-components` should become the place where semantic controls decide whether to render:

- custom `mozui` appearance
- native-backed control appearance

This can be done with an explicit API such as:

- `.native()`
- `.appearance(ControlAppearance::Native)`
- `.platform_style(PlatformStyle::Native)`

The important rule is:

- the semantic decision may live in `mozui-components`
- the native implementation must live in `mozui`

Do **not** move AppKit/UIKit/ObjC bridging into `mozui-components`.

## Recommended Public API Direction

### Buttons

Example direction:

```rust
Button::new("save")
    .label("Save")
    .appearance(ButtonAppearance::Native)
```

or

```rust
Button::new("save")
    .label("Save")
    .native()
```

Under the hood:

- `mozui-components::button::Button` remains semantic
- rendering branches to either custom view or `mozui::elements::native_button`

### Search Fields

Search fields need more than just a native skin. They need:

- native focus routing
- submit/change semantics
- toolbar embedding
- suggestion menu anchoring
- arrow/cancel keyboard behavior

This means:

- content search field can expose `.native()`
- toolbar search field should likely be a dedicated window-toolbar API, not just a component modifier

### Complex Window Chrome

The following should not be modeled as simple component modifiers:

- toolbar
- omnibox
- search suggestion menu
- sidebar chrome
- inspector chrome
- native tab strip
- native popover/panel/sheet

These belong at the `Window` or higher framework shell layer.

## Migration Strategy

### Phase 0: Freeze The Architectural Direction

Deliverables:

- agree that `mozui-native` will be retired
- agree that native substrate moves into `mozui`
- agree that `mozui-components` stays semantic
- agree that AppKit/UIKit integration is the primary path, not SwiftUI

Exit criteria:

- this document is accepted

### Phase 1: Introduce Core Native Contracts In `mozui`

Work:

- add `platform/native_controls.rs`
- add `PlatformWindow::native_controls()`
- add core `Window` wrappers for platform-native APIs
- add `NativeControlState`
- add base config structs and callback helper machinery

File impact:

- `crates/mozui/src/platform.rs`
- `crates/mozui/src/window.rs`
- `crates/mozui/src/platform/native_controls.rs`
- platform-specific backend modules

Exit criteria:

- no semantic components use this yet
- macOS backend compiles with stub or partial implementations
- iOS backend compiles with stub or partial implementations

### Phase 2: Move Existing `mozui-native` Leaf Controls Into `mozui`

Start with low-risk controls:

- button
- switch
- slider
- progress
- text field
- search field
- image/symbol view

Approach:

- port implementation ideas, not file structure
- convert wrapper-local callbacks into framework-scheduled callbacks
- unify state ownership under `NativeControlState`
- expose new `mozui::elements::native_*` primitives

Exit criteria:

- leaf native controls live in `mozui`
- `mozui-native` wrappers either delegate to them or are removed behind temporary shims

### Phase 3: Introduce Native Window Chrome APIs

Add:

- native toolbar model
- native toolbar buttons / groups / search item model
- native search focus APIs
- native popup menu API
- native suggestion menu API
- native popover/panel API

This phase is where the "native feel" starts improving materially.

Exit criteria:

- sample app can create native toolbar items via `Window`
- native search field focus is routed through the platform window
- popup/suggestion menus can anchor to toolbar or content targets

### Phase 4: Hosted Content / Surface Infrastructure

Add composition primitives to allow native shells hosting `mozui` content surfaces.

Target use cases:

- native sidebar with custom-rendered content
- native inspector host
- toolbar item host views
- popover/panel hosted content

Exit criteria:

- custom `mozui` content can be embedded into native platform containers with controlled lifecycle

### Phase 5: Rebuild Sidebar / Inspector / Toolbar Around Core Native APIs

Replace current ad hoc install helpers with framework-native versions.

Likely destinations:

- `mozui` window-native APIs for toolbar/sidebar/inspector shell
- semantic configuration helpers in `mozui-components` or app code

This phase should explicitly replace:

- `mozui-native::toolbar`
- `mozui-native::search`
- `mozui-native::sidebar`
- `mozui-native::inspector`
- `mozui-native::popover`
- `mozui-native::sheet`

Exit criteria:

- browser/finder-style demo can be rebuilt without `mozui-native`

### Phase 6: Add Semantic Native Rendering To `mozui-components`

Introduce semantic switching for targeted components.

Recommended order:

1. `Button`
2. `TextInput` / `Search`
3. `Switch`
4. `Slider`
5. `Progress`
6. `Select` / `ComboBox`
7. `Table`
8. `Sidebar` affordances where appropriate

API constraints:

- no platform-specific code in component crate
- only semantic render decisions
- keep fallback custom rendering available

Exit criteria:

- at least the common form controls support native-backed rendering

### Phase 7: Retire `mozui-native`

Steps:

- remove re-exports and direct dependencies
- migrate demos/examples
- update docs
- delete crate

Exit criteria:

- workspace builds without `crates/mozui-native`
- all surviving native functionality lives in `mozui`

## File-Level Migration Map

### Delete / Replace

- `crates/mozui-native/src/native_view.rs`
  - replaced by `mozui` native state + backend attach logic
- `crates/mozui-native/src/button.rs`
  - port to `crates/mozui/src/elements/native_button.rs`
- `crates/mozui-native/src/slider.rs`
  - port to `crates/mozui/src/elements/native_slider.rs`
- `crates/mozui-native/src/progress.rs`
  - port to `crates/mozui/src/elements/native_progress.rs`
- `crates/mozui-native/src/switch.rs`
  - port to `crates/mozui/src/elements/native_switch.rs`
- `crates/mozui-native/src/text_field.rs`
  - port to `crates/mozui/src/elements/native_text_field.rs`
- `crates/mozui-native/src/search.rs`
  - replace with core window-native search APIs
- `crates/mozui-native/src/toolbar.rs`
  - replace with core `Window::set_native_toolbar(...)`
- `crates/mozui-native/src/sidebar.rs`
  - replace with hosted-content / native sidebar framework APIs
- `crates/mozui-native/src/popover.rs`
  - replace with core native popover API
- `crates/mozui-native/src/sheet.rs`
  - replace with core native sheet/panel API
- `crates/mozui-native/src/inspector.rs`
  - replace with framework-native inspector hosting

### Add / Expand

- `crates/mozui/src/platform.rs`
- `crates/mozui/src/window.rs`
- `crates/mozui/src/platform/native_controls.rs`
- `crates/mozui/src/platform/macos/...`
- `crates/mozui/src/platform/ios/...`
- `crates/mozui/src/elements/native_*.rs`
- `crates/mozui-components/src/button/...`
- `crates/mozui-components/src/input/...`
- `crates/mozui-components/src/select.rs`
- `crates/mozui-components/src/table/...`

## Platform Backend Work

### macOS

Backend responsibilities:

- implement `PlatformNativeControls`
- implement native toolbar items via `NSToolbar`
- implement `NSSearchToolbarItem` and content `NSSearchField`
- implement native popup menus / suggestion menus / popovers / panels
- implement hosted-content rearrangement for sidebar / inspector / toolbar hosting
- implement glass / visual effect views
- support native tracking regions where custom hover correctness matters

Priority:

- highest, because Glass's best native feel is most visible on macOS

### iOS

Backend responsibilities:

- implement `PlatformNativeControls` for UIKit controls
- keep event routing inside `mozui`
- move away from wrapper-local ownership patterns
- add parity for text/search controls
- add hosted-content composition only where needed and justified

Important caveat:

- iOS may intentionally lag macOS in sidebar/inspector/chrome richness
- leaf-control parity should still be built on the same architecture

## API Shape For `mozui-components`

Recommended model:

- semantic components remain cross-platform
- native rendering is opt-in and explicit
- custom rendering remains available

Examples:

```rust
Button::new("save").native()
SearchInput::new("query").native()
Select::new("country").native()
```

Internally, each semantic component chooses between:

- custom component renderer
- framework-native element renderer

Do not create a second parallel component library.

## Testing Plan

### Unit / Compile Tests

- config structs compile on all targets
- native elements compile on macOS and iOS
- no `mozui-components` platform leakage

### Runtime Verification

macOS demos:

- toolbar search field behavior
- suggestion menu anchoring
- native button / switch / slider / text field interaction
- sidebar + inspector hosted-content composition
- popover / panel / sheet behavior

iOS demos:

- button / switch / slider / progress / text/search parity
- focus + text input routing
- scroll + gesture coexistence with hosted native controls

### Regression Areas

- focus handling
- keyboard shortcuts
- hit testing
- z-ordering between native and custom surfaces
- resizing / live layout
- cleanup and deallocation
- async callback safety

## Risk Register

### 1. Native / Custom Surface Z-Ordering

Risk:

- native views may not clip, overlap, or animate like custom `mozui` surfaces

Mitigation:

- use hosted-surface architecture explicitly
- document which combinations are supported

### 2. Event Re-Entrancy

Risk:

- platform delegates invoke state changes during paint/layout

Mitigation:

- schedule into next-frame callbacks only
- do not mutate `mozui` state directly from ObjC target/delegate callbacks

### 3. API Sprawl In `Window`

Risk:

- too many special-case native APIs

Mitigation:

- keep the surface area grouped around reusable primitives
- avoid app-specific APIs

### 4. `mozui-components` Bloat

Risk:

- semantic components become cluttered with rendering variants

Mitigation:

- add native appearance only to components that materially benefit
- centralize appearance selection patterns

### 5. Incomplete iOS Parity

Risk:

- architecture becomes macOS-first and iOS remains partial

Mitigation:

- require shared Rust-side contracts first
- allow backend-specific implementation depth second

## What We Should Not Do

- do not keep `mozui-native` as a second-class implementation layer with most logic still there
- do not move AppKit/UIKit/ObjC glue into `mozui-components`
- do not treat toolbar/sidebar/search/inspector as mere leaf widgets
- do not rely on SwiftUI hosting as the primary strategy
- do not directly invoke application state changes from native delegates

## Recommended Execution Order

1. Add core native-control contracts to `mozui`
2. Add callback scheduling + `NativeControlState`
3. Implement macOS backend for leaf controls
4. Implement native toolbar/search/popup APIs on macOS
5. Add hosted-content/surface APIs
6. Rebuild sidebar/inspector/window-chrome integration
7. Port iOS leaf controls to the new substrate
8. Add semantic `.native()` rendering paths in `mozui-components`
9. Migrate demos/examples
10. Delete `mozui-native`

## Definition Of Done

The migration is done when all of the following are true:

- `mozui-native` is removed from the workspace
- native control abstractions live in `mozui`
- `mozui-components` can opt into native rendering for common semantic controls
- native toolbar/search/sidebar/inspector behavior is available through `Window`
- native callbacks are scheduled through the framework event loop
- macOS demos show a materially more native toolbar/search/sidebar experience
- iOS demos use the same architecture even if the feature set is smaller

## Immediate Next Step After This Plan

Start with a non-user-facing substrate PR that adds:

- `NativeControlState`
- `PlatformNativeControls`
- callback scheduling helpers
- `Window::native_controls()`
- one ported control: native button

That is the smallest credible slice that proves the architecture before we touch toolbar/search/sidebar.
