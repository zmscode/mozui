# iOS Native Integration Plan (`mozui` + `mozui-native`)

## Goal

Build and ship a true native iOS backend for `mozui` (UIKit lifecycle + Metal rendering + touch/text input), then layer iOS-native controls in `mozui-native`.

This plan explicitly excludes WebView hosting as an implementation path.

## Non-Goals (MVP)

- No `WKWebView` fallback path.
- No iOS support target for `mozui-builder` during MVP.
- No requirement to make `mozui-webview` run on iOS during MVP.
- No immediate parity for every macOS-only native control in `mozui-native`.

## Architecture Overview

```text
┌─────────────────────────────────────────────────────────────────────┐
│                              iOS App Host                           │
│       (Xcode target, UIApplication/UIScene lifecycle bridge)        │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                              mozui                                  │
│                                                                     │
│  platform::ios (new)                                                 │
│  ┌──────────────┬──────────────┬──────────────┬───────────────────┐ │
│  │ lifecycle    │ dispatcher   │ window/input │ display/text      │ │
│  │ + app run    │ + executors  │ + events     │ + metrics/IME     │ │
│  └──────────────┴──────────────┴──────────────┴───────────────────┘ │
│                           │                                          │
│                           ▼                                          │
│                    Metal renderer (iOS)                              │
└───────────────────────────┬──────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          mozui-native                               │
│                src/macos/*       src/ios/*      src/apple/*         │
│                 (existing)        (new)         (optional shared)   │
└─────────────────────────────────────────────────────────────────────┘
```

## Audit Summary (All `mozui-*` Crates)

This plan is based on a full workspace sweep of:

- `mozui`
- `mozui-native`
- `mozui-components`
- `mozui-webview`
- `mozui-builder`
- `mozui-macros`
- `mozui-components-macros`

### Crate-by-Crate Lessons Learned

#### `mozui` (core blocker and primary scope)

- `platform.rs` currently selects macOS/Windows/Linux/wasm only; there is no iOS branch.
- Platform contracts are broad (`Platform` trait includes windowing, menus, clipboard, URL handling, credentials, prompts, etc.); iOS needs explicit behavior decisions, not just rendering.
- `keymap::KeyContext::new_with_defaults` sets `os` to `macos/linux/windows/unknown`; iOS currently resolves to `unknown`.
- `svg_renderer.rs` emoji family selection has macOS/Windows/Linux branches but not iOS.
- Several APIs are desktop-oriented (quit/reopen/menu semantics), so iOS behavior must be mapped explicitly rather than inheriting desktop defaults.

#### `mozui-native` (second major scope)

- Code is fully macOS-gated (`#[cfg(target_os = "macos")]`) despite package description saying macOS/iOS.
- Current implementations are AppKit-first (`NSView`, `NSTextField`, `NSWindow`, `cocoa`), not UIKit-compatible.
- `NativeViewState` pattern is reusable conceptually, but current types and coordinate assumptions are AppKit-specific.

#### `mozui-components` (compatibility layer scope)

- Multiple `#[cfg(target_os = "macos")]` vs `#[cfg(not(target_os = "macos"))]` branches currently force iOS into “non-macOS” behavior.
- This is wrong for hardware-keyboard UX on iOS/iPadOS in several places (for example cmd-vs-ctrl shortcuts and key legends in `kbd.rs`, input and text keybindings).
- Desktop-only UI pieces (title bar/window controls) should remain non-iOS and be explicitly gated.

#### `mozui-webview`

- Desktop `wry` integration crate; not part of native iOS path.
- Keep out of iOS MVP scope.

#### `mozui-builder`

- Desktop-oriented builder app (`open_window`, title-bar assumptions, etc.).
- Exclude from iOS build goals and CI gates for MVP.

#### `mozui-macros` and `mozui-components-macros`

- Proc-macro crates are platform-agnostic and mostly unaffected.
- Only ensure downstream crates keep compiling with new iOS cfg branches.

## Decision: Platform Features in `mozui/src/platform/*`

Yes, this is the right direction.

Use **crate features + module-level cfg in `mozui/src/platform/*`** to stage iOS incrementally without destabilizing desktop backends.

### Recommended Feature Model

In `crates/mozui/Cargo.toml`, add staged iOS features:

- `ios` (enables iOS platform module)
- `ios-metal` (renderer + surface path)
- `ios-text` (IME/text input integration)
- `ios-clipboard` (UIPasteboard bridge)
- `ios-url-open` (`open_url` bridge)
- `ios-credentials` (keychain bridge, can be deferred)

Then gate iOS module internals in `crates/mozui/src/platform/ios/*` by these features so compile milestones are explicit.

### Why this matters

- You avoid “all-or-nothing” iOS implementation risk.
- CI can enforce staged compile targets.
- You can land non-rendering platform work before Metal is complete.

## Planned Module/File Changes (High Level)

### `mozui`

- Update: `crates/mozui/src/platform.rs`
- Add: `crates/mozui/src/platform/ios/mod.rs`
- Add: `crates/mozui/src/platform/ios/platform.rs`
- Add: `crates/mozui/src/platform/ios/window.rs`
- Add: `crates/mozui/src/platform/ios/dispatcher.rs`
- Add: `crates/mozui/src/platform/ios/display.rs`
- Add: `crates/mozui/src/platform/ios/keyboard.rs`
- Add: `crates/mozui/src/platform/ios/text_system.rs`
- Add: `crates/mozui/src/platform/ios/metal_renderer.rs`
- Update: `crates/mozui/Cargo.toml` (ios feature staging + target deps)
- Update: `crates/mozui/src/keymap/context.rs` (`os` context for iOS)
- Update: `crates/mozui/src/svg_renderer.rs` (emoji fallback family for iOS)

### `mozui-native`

- Update: `crates/mozui-native/src/lib.rs` (split by platform)
- Add: `crates/mozui-native/src/ios/*` (UIKit implementations)
- Optional add: `crates/mozui-native/src/apple/*` (shared helpers)
- Update: `crates/mozui-native/Cargo.toml` (iOS cfg deps)

### `mozui-components`

- Update target-specific keyboard and UI behavior:
  - `crates/mozui-components/src/input/state.rs`
  - `crates/mozui-components/src/text/state.rs`
  - `crates/mozui-components/src/kbd.rs`
  - `crates/mozui-components/src/title_bar.rs`
  - other `not(macos)` call sites as discovered

## Architecture Constraints and Invariants

- Keep one `Platform` trait; avoid introducing a parallel iOS trait system.
- Prefer Apple-shared abstractions only where proven (`macos` + `ios`), not speculative generalization.
- Do not regress existing macOS behavior while adding iOS.
- Treat iOS as first-class in cfg branches, not as `not(macos)`.

## Phase 0 Decisions

### Target Matrix

- `aarch64-apple-ios`: physical iPhone build target.
- `aarch64-apple-ios-sim`: Apple Silicon simulator target.

### Minimum Platform Assumptions

- Minimum iOS version: `iOS 17`.
- Development baseline: current Xcode toolchain with iOS 17+ SDK.
- Initial hardware assumption: Apple Silicon host machine for simulator work.

### Why `iOS 17`

- It is a clean floor for a new backend with no need to optimize for older UIKit/scene lifecycle edge cases yet.
- It keeps API choices simpler for Metal, scene management, text input, and safe-area handling.
- This project is early enough that lowering the minimum version now would add cost without helping the native backend land faster.

### `Platform` Policy Table (Phase 0)

| `Platform` area | iOS MVP policy | Implementation note |
| --- | --- | --- |
| `run` | Supported | Bridge to `UIApplication` / `UIScene` lifecycle. |
| `quit` | No-op / unsupported semantic | iOS apps should not programmatically quit in normal flow. |
| `restart` | Unsupported for MVP | Return an explicit error or documented no-op; do not fake desktop restart semantics. |
| `activate` | Supported best-effort | Map to foreground activation where platform APIs permit. |
| `hide` / app hide semantics | Unsupported semantic | Do not emulate macOS hide behavior. |
| `open_window` | Supported | Backed by primary UIKit window/surface management; multi-window can be deferred. |
| `window_appearance` | Supported | Derive from trait collection / interface style. |
| `button_layout` | Unsupported / `None` | No desktop titlebar button layout on iOS. |
| `open_url` | Supported | Bridge to `UIApplication::openURL` equivalent. |
| `on_open_urls` | Supported | Required for deep-link handling. |
| `register_url_scheme` | Unsupported at runtime | iOS registration is bundle-config driven, not runtime mutable. |
| `prompt_for_paths` | Deferred | Use document picker later if needed; not required for first native MVP. |
| `prompt_for_new_path` | Deferred | Same as above. |
| `can_select_mixed_files_and_dirs` | Deferred | Depends on document picker integration. |
| `reveal_path` | Unsupported for MVP | No Finder-style reveal behavior. |
| `open_with_system` | Deferred | Only implement if document handoff is required. |
| `on_quit` | Unsupported semantic | No normal user-driven app quit callback on iOS. |
| `on_reopen` | Unsupported semantic | No dock reopen model on iOS. |
| `set_menus` / app menu hooks | Unsupported semantic | Desktop menu system does not exist on iOS. |
| `set_dock_menu` / dock actions | Unsupported semantic | No dock menu on iOS. |
| `add_recent_document` | Deferred | Can be omitted initially. |
| `update_jump_list` | Unsupported semantic | Windows-specific concept; no iOS equivalent. |
| `thermal_state` | Supported | Map from iOS thermal APIs. |
| `on_thermal_state_change` | Supported | Useful for renderer throttling and diagnostics. |
| `app_path` | Supported | Return bundle path where meaningful. |
| `path_for_auxiliary_executable` | Unsupported for MVP | iOS bundle model differs; revisit only if needed. |
| `set_cursor_style` | Mostly no-op | Touch-first platform; limited hardware pointer support can come later. |
| `should_auto_hide_scrollbars` | Supported | Use iOS-native behavior. |
| `read_from_clipboard` / `write_to_clipboard` | Supported in `ios-clipboard` | Backed by `UIPasteboard`. |
| credentials APIs | Deferred to `ios-credentials` | Use Keychain once core backend is stable. |
| keyboard layout / mapper | Supported | Needed for hardware keyboard correctness and shortcuts. |

## Phased Plan

## Phase 0: Contract and Feature Scaffolding

### Objectives

- Lock implementation boundaries and stage gates.

### Tasks

- Add iOS feature flags to `crates/mozui/Cargo.toml`.
- Define iOS target matrix:
  - `aarch64-apple-ios`
  - `aarch64-apple-ios-sim`
- Add iOS policy table for each required `Platform` method:
  - `run/quit/reopen`
  - menus/dock menu
  - prompts/file dialogs
  - URL open/register scheme
  - clipboard/credentials
- Decide minimum iOS version and SDK constraints.

### Exit Criteria

- Every mandatory `Platform` method has an iOS owner and behavior decision.
- Feature gating strategy is agreed and documented.

### Phase 0 Deliverables Landed

- Staged iOS feature flags added to `crates/mozui/Cargo.toml`:
  - `ios`
  - `ios-metal`
  - `ios-text`
  - `ios-clipboard`
  - `ios-url-open`
  - `ios-credentials`
- Target matrix documented.
- Minimum iOS / SDK assumptions documented.
- Initial `Platform` policy table documented for iOS behavior.

## Phase 1: `mozui` iOS Platform Skeleton

### Objectives

- Introduce compile-valid iOS backend structure.

### Tasks

- Wire iOS backend selection in `crates/mozui/src/platform.rs`.
- Add `crates/mozui/src/platform/ios/` module tree:
  - `mod.rs`
  - `platform.rs`
  - `window.rs`
  - `dispatcher.rs`
  - `display.rs`
  - `keyboard.rs`
  - `text_system.rs`
  - `metal_renderer.rs`
- Add iOS dependencies (UIKit/Foundation/QuartzCore/Metal via objc2 ecosystem) behind iOS cfg.
- Ensure `keymap::KeyContext` recognizes iOS (`os=ios` or `os=apple-mobile`, but explicit and stable).

### Exit Criteria

- `cargo check -p mozui --target aarch64-apple-ios-sim` passes with stubs.
- iOS backend is selected by cfg on iOS targets.

### Phase 1 Deliverables Landed

- `target_os = "ios"` platform selection wired in `crates/mozui/src/platform.rs`.
- New iOS module tree scaffolded under `crates/mozui/src/platform/ios/`:
  - `mod.rs`
  - `dispatcher.rs`
  - `display.rs`
  - `keyboard.rs`
  - `metal_renderer.rs`
  - `platform.rs`
  - `text_system.rs`
  - `window.rs`
- `keymap::KeyContext` now resolves `os=ios` on iOS targets.
- Current scaffold uses no-op / placeholder implementations intentionally; UIKit, Metal, and text-system bindings remain for the next implementation slices.

## Phase 2: Lifecycle, Window, and Scheduler Semantics

### Objectives

- Make app lifecycle correct on iOS main thread.

### Tasks

- Bridge UIApplication/UIScene lifecycle to `Platform::run` flow.
- Ensure foreground executor dispatches on main thread.
- Map unsupported desktop semantics safely:
  - app quit/reopen hooks
  - dock menu APIs
  - some file prompt behaviors
- Implement display metrics, safe areas, orientation updates.

### Exit Criteria

- App survives background/foreground transitions.
- Rotation updates trigger layout and redraw correctly.

### Phase 2 Deliverables Landed

- iOS dispatcher now uses Apple dispatch queues instead of the temporary thread-only placeholder.
- `dispatch_on_main_thread` is now distinct from background dispatch in the iOS backend scaffold.
- iOS platform state now tracks:
  - launch status
  - lifecycle phase (`initialized`, `launching`, `active`, `background`)
  - current window appearance
  - active window
  - window stack ordering
  - clipboard contents
  - thermal state
- `Platform::run` now completes launch through the foreground executor instead of invoking launch inline from arbitrary caller context.
- iOS platform now maintains a live registry of windows, not just window ids.
- iOS window instances now participate in platform bookkeeping:
  - activation updates active window ordering
  - previous and new active windows receive active-status callbacks
  - dropped windows are removed from platform window state
  - close callbacks are retained and invoked on drop
  - fullscreen state is tracked in the scaffold
- iOS display state now tracks mutable:
  - bounds
  - visible bounds
  - scale factor
- Future UIKit bridge entry points now exist in code for:
  - entering background / foreground
  - pushing display-metric updates
  - attaching and detaching a UIKit surface per window
- Background / foreground lifecycle semantics now suspend and resume the window rendering surface and request a redraw on foreground re-entry.

### Still Deferred Inside Phase 2

- Real `UIApplication` / `UIScene` lifecycle bridge.
- Rotation and safe-area propagation from real UIKit notifications instead of injected bridge calls.
- True app foreground/background notifications from iOS.
- Multi-scene policy and scene activation semantics.

## Phase 3: Rendering Path (`ios-metal`)

### Objectives

- First stable pixels on iPhone and simulator.

### Tasks

- Implement `CAMetalLayer`-backed renderer for iOS.
- Adapt or share renderer logic from macOS where practical.
- Handle drawable lifecycle, resize, content scale, frame pacing.
- Add iOS coverage for any renderer-adjacent platform assumptions in `mozui`.

### Exit Criteria

- Simple scene renders on device and simulator.
- No persistent present/drawable starvation across suspend/resume.

### Phase 3 Deliverables Landed

- `mozui::platform::wgpu` now compiles on iOS targets.
- Shared `WgpuContext` now selects the Metal backend on Apple platforms instead of Vulkan/GL.
- iOS target dependencies now include the renderer/runtime pieces required for the first real surface path:
  - `wgpu`
  - `font-kit`
  - `objc2-ui-kit`
  - `objc2-quartz-core`
- `crates/mozui/src/platform/ios/metal_renderer.rs` is no longer a placeholder:
  - it now wraps the shared `WgpuRenderer`
  - it tracks the live raw UIKit window surface
  - it handles surface replacement, suspension, recovery, resize, transparency, and draw
- iOS windows now expose real raw-window-handle UIKit handles once a host `UIView` is attached:
  - `UiKitWindowHandle`
  - `UiKitDisplayHandle`
- iOS window state now supports:
  - attaching a host UIKit view/controller surface
  - detaching a surface
  - resuming a suspended surface
  - updating renderer drawable size from display metrics
  - switching sprite atlas and GPU reporting over to the live renderer
- `cargo check -p mozui --target aarch64-apple-ios-sim` passes with the renderer path compiled in.
- A real host app scaffold now exists at `crates/mozui-native/ios/TestHost`:
  - XcodeGen project
  - Swift app delegate / view controller
  - `CAMetalLayer` host view
  - C bridge header into Rust
- A Rust `staticlib` demo host crate now exists at `crates/mozui-ios-demo` and exposes the first C ABI bridge for:
  - host creation / destruction
  - surface attach / detach
  - metric updates
  - foreground / background transitions
  - last-error inspection
- `MozuiIOSHost` now builds successfully for the iOS simulator with the Rust demo library linked in.

### Still Deferred Inside Phase 3

- First rendered frame verification on simulator and physical device.
- Any direct `CAMetalLayer` customization beyond what the shared `wgpu` Metal surface already provides.
- Renderer tuning for suspend/resume starvation, frame pacing, and content-scale changes on real device hardware.

### Additional Lessons Learned After First Device Bring-Up

- First pixels and first controls now render on a physical iPhone through the native host path.
- `mozui-components::Switch` and `mozui-native::NativeSwitch` both mount on iOS now.
- This exposed three higher-priority blockers that must be treated as backend work, not polish:
  - surface sizing / presentation is still incorrect at the screen edges
  - `mozui` interaction delivery on iOS is still incomplete for normal component hit-testing
  - UIKit-hosted native controls need intrinsic-size-aware layout instead of naive full-bounds embedding
- Concretely observed on device:
  - a persistent black band remains at the bottom of the screen
  - a black strip remains to the right of the diagnostic orange edge stripe
  - the `mozui-components` switch still does not respond to touch interaction
  - the `mozui-native` switch needed explicit layout correction to avoid top-left placement inside a larger container
- This means the first “real app usable on phone” milestone is not “more controls”, but:
  - correct full-screen surface metrics
  - reliable touch-to-click mapping for normal mozui elements
  - safe native-subview embedding semantics for UIKit-backed controls

## Phase 4: Input + Text + Clipboard (`ios-text`, `ios-clipboard`)

### Objectives

- Make interaction production-usable.

### Tasks

- Map touch/gesture events to `mozui` input model.
- Implement software keyboard and first-responder flow.
- Implement composition/selection/edit behavior for text input.
- Add clipboard bridge via iOS APIs.
- Reconcile keyboard mapping semantics for iOS hardware keyboards.
- Verify normal `mozui-components` interaction works on device, including:
  - hitbox generation
  - touch-down / touch-up to click synthesis
  - pointer/touch compatibility for existing `on_mouse_down` / `on_click` consumers
- Audit `mozui-components` controls that currently assume desktop mouse semantics and define the iOS interaction contract.

### Exit Criteria

- End-to-end text editing works on device.
- Touch and scroll gestures are reliable.
- A simple `mozui-components` control such as `Switch` toggles correctly on an iPhone without native UIKit fallback.
- No component requires “desktop mouse only” interaction assumptions for basic tap activation on iOS.

### Phase 4 Priority Adjustment

Before expanding the iOS control catalog, Phase 4 must close the current interaction gaps seen in device testing:

- fix touch delivery for ordinary `mozui` controls
- verify click synthesis against hitboxes and content masks on iOS
- confirm that stateful components redraw correctly after touch interaction

Without this, additional `mozui-components` work will appear visually correct but remain non-interactive on phone hardware.

## Phase 5: `mozui-native` Apple Split and iOS Controls

### Objectives

- Convert `mozui-native` into real multi-Apple-platform crate.

### Tasks

- Refactor `crates/mozui-native/src/lib.rs` into platform folders:
  - `src/macos/*`
  - `src/ios/*`
  - optional shared `src/apple/*` helpers
- Add iOS dependency gates in `crates/mozui-native/Cargo.toml`.
- Port `NativeViewState` concept to UIKit view hosting.
- Implement first iOS-native controls required for MVP screen:
  - button
  - switch
  - text field
  - picker
- Add intrinsic-size/layout policy for UIKit-hosted controls:
  - avoid defaulting every native control to full parent bounds
  - define whether each control should use intrinsic size, explicit style size, or container-driven size
  - ensure centering/alignment semantics match the surrounding mozui layout
- Ensure native controls tolerate deferred host-view attachment on iOS without panicking.

### Exit Criteria

- iOS control primitives compile, mount, unmount, and receive input safely.
- No ownership leaks in repeated lifecycle tests.
- A `mozui-native::NativeSwitch` renders at the expected size and alignment inside a styled mozui container on iPhone.

### Phase 5 Deliverables Landed So Far

- `mozui-native` is no longer purely macOS-gated for `NativeSwitch`.
- `NativeViewState` now has an iOS/UIKit hosting path in addition to the existing macOS/AppKit path.
- `mozui-native::NativeSwitch` now compiles for iOS and mounts a real UIKit `UISwitch`.
- iOS native-view attachment now tolerates the initial “window handle unavailable” state instead of panicking before the host `UIView` is attached.

### Still Deferred Inside Phase 5

- Split `mozui-native` into explicit `src/ios/*` and `src/apple/*` directories; current implementation is still a minimal cfg-based extension of the existing layout.
- Native intrinsic sizing policy is still incomplete.
- Additional controls beyond `NativeSwitch` remain unported.

## Phase 6: `mozui-components` iOS Behavior Corrections

### Objectives

- Prevent desktop-fallback behavior from degrading iOS UX.

### Tasks

- Replace broad `not(macos)` branches with explicit intent:
  - `apple_desktop` (`macos`)
  - `apple_mobile` (`ios`)
  - `non_apple`
- Update keybinding defaults in:
  - `src/input/state.rs`
  - `src/text/state.rs`
- Update key legend formatting in `src/kbd.rs` for iOS hardware keyboard semantics.
- Gate desktop-only components (title bar/window controls) out of iOS paths.
- Revisit visual constants tied to macOS-only assumptions (cursor width, etc.) where needed.

### Exit Criteria

- iOS no longer inherits generic non-macOS keyboard behavior.
- Desktop-only controls are not accidentally active on iOS.

## Phase 7: Tooling, CI, and Device Deployment

### Objectives

- Make native iOS workflow repeatable for contributors.

### Tasks

- Add scripts/docs for:
  - Rust iOS target build commands
  - simulator run
  - physical device deploy/signing via Xcode
- Add CI jobs for iOS compile targets (`mozui`, then `mozui-native`, then sample app).
- Keep non-goal crates (`mozui-builder`, `mozui-webview`) out of iOS MVP gates.

### Exit Criteria

- Fresh contributor can deploy using only docs and scripts.
- CI enforces iOS compile health on core crates.

### Phase 7 Deliverables Landed

- `crates/mozui-native/ios/TestHost/README.md` now documents:
  - required Rust targets
  - Xcode project generation
  - simulator build verification
  - physical-device bring-up steps
- The host app now uses `UIScene` lifecycle wiring instead of legacy app-window startup:
  - `SceneDelegate.swift`
  - `UIApplicationSceneManifest`
- `crates/mozui-native/ios/TestHost/project.yml` now embeds Rust build automation for:
  - `aarch64-apple-ios-sim`
  - `aarch64-apple-ios`
- `project.yml` now also carries scheme-level run settings intended to reduce Xcode Metal debug wrapping during first-frame bring-up.
- The simulator path is now reproducible from the repo with:
  - `xcodegen generate`
  - `xcodebuild ... -sdk iphonesimulator ... build`

### Still Deferred Inside Phase 7

- A reproducible “device runtime checklist” for:
  - confirming full-screen surface sizing
  - confirming tap interaction on standard mozui controls
  - confirming UIKit native-subview alignment

## Immediate Next Slice

The next implementation slice should not add more controls first.

It should be:

1. Fix iOS full-surface sizing so the render covers the entire host view with no right/bottom black bands.
2. Fix iOS touch interaction so a plain `mozui-components::Switch` toggles on device.
3. Finish native UIKit control layout semantics so `mozui-native::NativeSwitch` uses correct intrinsic size/alignment inside mozui layout.
4. Only after those pass on a physical iPhone, continue expanding `mozui-native` iOS controls.

- CI jobs for iOS compile/build health.
- A one-command repo-level wrapper script for host generation/build/deploy.
- Verified physical-device build and first launch after signing configuration.
- If `wgpu` continues to crash under an Xcode-provided Metal capture wrapper on runtime launch, patching or upgrading the `wgpu-hal` Metal capability probe may still be required.

## Suggested Milestones

1. M1: iOS backend compiles (`Phase 0-1`).
2. M2: app launches and survives lifecycle (`Phase 2`).
3. M3: first render on device (`Phase 3`).
4. M4: usable text/touch input (`Phase 4`).
5. M5: first `mozui-native` iOS controls (`Phase 5`).
6. M6: components behavior corrected for iOS (`Phase 6`).
7. M7: reproducible tooling + CI (`Phase 7`).

## Phase Gates (Acceptance Checklist)

1. M1 gate:
   - `mozui` iOS target compiles with feature flags.
   - iOS backend selected at runtime on iOS targets.
2. M2 gate:
   - launch/background/foreground flow stable in simulator.
3. M3 gate:
   - first frame rendered on simulator and physical device.
4. M4 gate:
   - text input (insert, delete, selection, composition) works on device.
5. M5 gate:
   - at least four `mozui-native` iOS controls are usable in one test screen.
6. M6 gate:
   - iOS keyboard shortcuts/labels no longer follow generic non-macOS defaults.
7. M7 gate:
   - documented deploy workflow is reproducible by a fresh contributor.

## Key Risks and Mitigations

- Risk: `Platform` trait breadth slows first launch.
  - Mitigation: use staged feature gates and explicit “unsupported on iOS” mappings where appropriate.
- Risk: iOS incorrectly treated as generic non-macOS in components.
  - Mitigation: replace `not(macos)` branches with explicit OS families.
- Risk: macOS regressions while refactoring Apple shared code.
  - Mitigation: keep macOS codepaths intact first; extract shared code only after iOS path is green.
- Risk: lifecycle differences cause latent crashes.
  - Mitigation: add dedicated suspend/resume/orientation smoke tests early.

## Open Questions

1. Should iOS use the existing Metal renderer abstractions directly or add an Apple-shared renderer layer first?
2. For unsupported desktop APIs in `Platform` (for example dock menus), should iOS return no-op behavior or explicit errors?
3. Should iOS keybinding defaults in `mozui-components` mirror macOS command-key conventions whenever hardware keyboard is attached?
4. Where should the minimal iOS host app live long-term: inside this workspace as a sample app or in a sibling repo?

## Definition of Done (Native iOS MVP)

- `mozui` has a dedicated iOS backend selected on iOS targets.
- Metal renderer draws reliably on physical iPhone hardware.
- Touch + text input workflows function end-to-end.
- `mozui-native` provides a minimal iOS control set used by a sample screen.
- `mozui-components` keyboard/desktop behavior is iOS-correct.
- Build/test/deploy workflow is documented and reproducible.
