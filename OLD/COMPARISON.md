mozui vs GPUI — Feature Coverage Comparison

### Architecture & Core

| Feature | GPUI | mozui | Notes |
|---------|------|-------|-------|
| Rendering backend | Metal / DX12 / Vulkan | wgpu (Metal) | GPUI has native per-platform backends; mozui uses wgpu abstraction (easier cross-platform later) |
| Layout engine | Custom Flexbox (Taffy-derived) | Taffy | Both Flexbox, no Grid |
| State management | Entity/Model system (ECS-like) | Signal arena (React hooks) | Different paradigms — GPUI is more Elm-like, mozui is more React-like |
| Text rendering | cosmic-text + swash | cosmic-text + swash | **At parity** — both use HarfBuzz-compatible shaping (harfrust), swash rasterization, BiDi, complex scripts, ligatures |
| Async | Custom executor + Tasks | Custom executor + timers | Both have async runtimes; GPUI's is more mature |

### Platform Support

| Platform | GPUI | mozui |
|----------|------|-------|
| macOS | **Full** | **Full** |
| Windows | **Full** | **Full** (Win32, DWM custom titlebar, per-monitor DPI, min/max constraints) |
| Linux | **Full** | **Full** (X11 via x11rb + Wayland via wayland-client, runtime detection) |
| WASM | No | No (planned) |

Both ship all 3 desktop platforms. mozui uses wgpu for cross-platform GPU rendering. Linux backend auto-detects X11 vs Wayland at runtime.

### Rendering Capabilities

| Feature | GPUI | mozui |
|---------|------|-------|
| Rounded rects (SDF) | Yes | Yes |
| Text | Yes | Yes |
| Shadows | Yes (box + text) | Yes (Gaussian blur via erf) |
| Gradients | Yes (linear) | Yes (linear + radial) |
| Images | Yes | Yes (PNG, JPEG, WebP, GIF) |
| SVG | Yes (via resvg) | Yes (via resvg, arbitrary SVG rendering) |
| Animated GIF | No (static only) | **Yes** (frame cycling via AnimatedImage) |
| Custom shaders | Yes (Canvas element) | **No** |
| Clip rects | Yes | Yes (software stack) |
| Paint caching / invalidation | Yes | Yes (layout caching, paint-only redraws) |
| Object-fit modes | Yes | Yes (Cover, Contain, Fill) |

Remaining rendering gap: custom shaders (Canvas element equivalent).

### Component Library (54 components)

| Category | GPUI | mozui | Details |
|----------|------|-------|---------|
| Primitives | div, text, img, svg, canvas | div, text, label, icon, img (5) | GPUI has canvas (custom shaders); mozui has label (styled text) and icon (Phosphor icon atlas) |
| Form controls | Minimal (div + handlers) | **15 components** | button, button_group, icon_button, checkbox, radio, switch, slider, text_input, rating, **select** (+ combobox), **color_picker** (HSV + alpha), **date_picker** (calendar dropdown), **calendar** (standalone grid), **number_input** (±buttons), **toggle_group** (segmented) |
| Data display | Not built-in | **12 components** | badge, tag, kbd, progress, spinner, description_list, pagination, **table** (sortable, selectable), virtual_list, **skeleton** (rect/circle/pill), **avatar** (initials, icon, image, status), **alert** (4 variants, dismissible) |
| Navigation | Not built-in | **6 components** | tab/tab_bar (4 variants), breadcrumb, link, stepper, **tree_view** (hierarchical, expand/collapse, icons), **sidebar** (collapsible, groups, icons, active state) |
| Layout | Not built-in | **9 components** | accordion, collapsible, group_box, divider (3 variants), list, virtual_list, **card** (header/body/footer), **resizable_panel** (drag-to-resize, h/v axis), **sheet** (slide-in overlay, 4 placements, animated) |
| Overlays | Core feature | **6 components** | dialog (animated), menu, tooltip, popover, notification (animated, 5 types, **6 placements**), **command_palette** (searchable action list, animated) |

Key insight: GPUI provides **low-level primitives** (div, text, img, svg, canvas) and leaves component building to the application (Zed builds its own buttons, tabs, etc.). mozui ships a **component library** out of the box — more accessible for users who want ready-made components.

### Event Handling & Interaction

| Feature | GPUI | mozui |
|---------|------|-------|
| Click/hover/active | Yes | Yes |
| Keyboard events | Yes | Yes |
| Focus system | Yes (scopes + trapping) | Yes (scopes + trapping) |
| Drag-and-drop | Yes (files, text, custom) | Yes (DragId-based source/target matching) |
| IME / text input | **Yes** (complex input) | **No** |
| Context menus (right-click) | Yes | **Yes** (`.on_right_click()` + dispatch) |
| Scroll physics | Yes (momentum) | Yes (momentum with deceleration) |
| Custom cursors | Yes | Yes |
| Keybindings/Actions | Yes | Yes |

IME is the main remaining interaction gap.

### Reactivity & Hooks

| Feature | GPUI | mozui |
|---------|------|-------|
| State (signal/entity) | Entity/Model | `cx.use_signal()` |
| Derived state | Entity observers | **`cx.use_memo(deps, compute)`** |
| Side effects | Subscriptions | **`cx.use_effect(deps, effect)`** |
| Animated values | Manual | `cx.use_animated()`, `cx.use_spring()` |
| Scroll state | Manual | `cx.use_scroll()` |

mozui now has a complete React-style hook system: signals, memo, effects, animations, and scroll state.

### Animation

| Feature | GPUI | mozui |
|---------|------|-------|
| Spring physics | Yes | Yes |
| Tween/transition | Yes | Yes |
| Easing functions | Yes (cubic bezier) | Yes (cubic bezier + presets) |
| Baked-in component animations | No (manual) | Yes (dialog, notification with .anim() / .no_anim()) |
| Shared animation flag | N/A (entity-driven) | Yes (Rc<Cell<bool>>) |
| Animation hooks | Entity observers | `cx.use_animated()`, `cx.use_spring()` |

### Window & Overlay System

| Feature | GPUI | mozui |
|---------|------|-------|
| Custom window chrome | Yes | Yes (macOS, Windows DWM) |
| Multi-window | Yes | Yes (WindowId routing, per-window state, dynamic open/close) |
| Modal dialogs | Yes (with backdrop) | Yes (focus trap, backdrop dismiss, animated) |
| Popovers | Yes (anchored positioning) | Yes (anchor-based, fit modes) |
| Menus (context/dropdown) | Yes | Yes (icons, shortcuts, separators, right-click) |
| Tooltips | Yes | Yes (placement, shortcuts, hover trigger) |
| Notification/toast | Not built-in | Yes (5 types, stacking, dismiss, animated) |

### Advanced Features

| Feature | GPUI | mozui |
|---------|------|-------|
| Accessibility (screen readers) | Emerging (via platform APIs) | **No** |
| Clipboard | Yes | Yes |
| File dialogs (open/save) | Yes | **Yes** (NSOpenPanel / NSSavePanel) |
| Paint caching | Yes | Yes (layout caching) |
| Hot reload | No | No |

---

## Summary: Where mozui is Ahead

1. **Component library** — 42 ready-to-use components vs GPUI's "bring your own". Genuine differentiator for DX.
2. **Hook-based API** — `cx.use_signal()`, `cx.use_memo()`, `cx.use_effect()`, `cx.use_animated()` feel familiar to React developers.
3. **Theme system** — 50+ design tokens with dark/light presets. GPUI leaves theming to the application.
4. **Cross-platform via wgpu** — Single GPU abstraction (wgpu) vs GPUI's per-platform native backends (Metal/DX12/Vulkan). Easier to extend to new targets (WASM, mobile).
5. **Baked-in animations** — Dialog and Notification animate automatically; GPUI requires manual animation.
6. **Animated GIF** — Native frame-cycling support; GPUI only supports static images.
7. **Gradients** — Linear + radial; GPUI only has linear.

## Summary: Where GPUI is Ahead

1. **IME** — Essential for international users and CJK input.
2. **Custom shaders** — Canvas element allows arbitrary GPU rendering.
3. **Accessibility** — Emerging screen reader support via platform APIs.
4. **Maturity** — Powers a real product (Zed) used by thousands daily.

## Recommended Priorities

### Completed
- ~~Overlay system (Dialog, Menu, Tooltip, Notification)~~
- ~~Shadows + gradients in renderer~~
- ~~Multi-window support~~
- ~~Scroll physics~~
- ~~Image/SVG/GIF rendering~~
- ~~Baked-in component animations (Dialog, Notification)~~
- ~~Tab bar variants (Underline, Pill, Outline, Segmented)~~
- ~~Divider variants (Solid, Dashed, Dotted)~~
- ~~Text rendering upgrade (cosmic-text + swash)~~
- ~~Select/Dropdown (+ combobox)~~
- ~~Context menus (right-click)~~
- ~~Table/DataGrid (sortable, selectable)~~
- ~~use_memo / use_effect hooks~~
- ~~File dialogs (open/save)~~
- ~~Color picker (HSV + alpha)~~
- ~~Skeleton/Loading (rect, circle, pill, presets)~~
- ~~Avatar (initials, icon, image, status indicator)~~
- ~~Toast positioning (6 placements)~~
- ~~Spinner (animated icon + label)~~
- ~~Date picker / Calendar (single, range, disabled dates, month navigation)~~
- ~~Tree View (hierarchical, expand/collapse, icons)~~
- ~~Number Input (±buttons, min/max/step/precision)~~
- ~~Toggle Group (segmented control, exclusive selection)~~
- ~~Card (header/body/footer container)~~
- ~~Alert/Banner (info/success/warning/danger, dismissible)~~
- ~~Command Palette (searchable action list, keyboard highlight, animated)~~
- ~~Resizable Panel (drag-to-resize, horizontal/vertical axis, min/max constraints)~~
- ~~Sheet (slide-in overlay panel, 4 placements, animated entrance)~~
- ~~Sidebar (collapsible, groups, icons, active state, left/right)~~

### Completed — Platform & Infrastructure
- ~~Windows platform shell~~ (Win32 event loop, input, HiDPI, DWM custom titlebar, min/max constraints, clipboard)
- ~~Linux platform shell~~ (X11 via x11rb + Wayland via wayland-client, runtime detection, cursor support, size hints)

### High Priority — Components
1. **Scroll-to / programmatic scroll** — `cx.scroll_to(offset)` or element-based scroll targeting.

### High Priority — Platform & Infrastructure
1. **IME support** — Blocks international users. Requires platform-level integration (TSM on macOS, IMM32 on Windows).
2. **Accessibility** — Screen reader support via platform accessibility APIs (NSAccessibility, UIA). Growing requirement.
3. **Custom shaders** — Canvas element equivalent for arbitrary GPU rendering.
4. **WASM target** — Browser deployment via wgpu WebGPU backend.
