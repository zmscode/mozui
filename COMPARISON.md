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
| Windows | **Full** | Not started |
| Linux | **Full** | Not started |
| WASM | No | No (planned) |

GPUI ships production software on all 3 desktop platforms. mozui is macOS-only. Windows/Linux support is the highest-impact work remaining.

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

### Component Library (42 components)

| Category | GPUI | mozui | Details |
|----------|------|-------|---------|
| Primitives | div, text, img, svg, canvas | div, text, label, icon, img (5) | GPUI has canvas (custom shaders); mozui has label (styled text) and icon (Phosphor icon atlas) |
| Form controls | Minimal (div + handlers) | **11 components** | button, button_group, icon_button, checkbox, radio, switch, slider, text_input, rating, **select** (+ combobox), **color_picker** (HSV + alpha) |
| Data display | Not built-in | **10 components** | badge, tag, kbd, progress, description_list, pagination, **table** (sortable, selectable), virtual_list, **skeleton** (rect/circle/pill), **avatar** (initials, icon, image, status) |
| Navigation | Not built-in | **4 components** | tab/tab_bar (4 variants), breadcrumb, link, stepper |
| Layout | Not built-in | **6 components** | accordion, collapsible, group_box, divider (3 variants), list, virtual_list |
| Overlays | Core feature | **5 components** | dialog (animated), menu, tooltip, popover, notification (animated, 5 types, **6 placements**) |

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
| Custom window chrome | Yes | Yes (macOS) |
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
4. **Baked-in animations** — Dialog and Notification animate automatically; GPUI requires manual animation.
5. **Animated GIF** — Native frame-cycling support; GPUI only supports static images.
6. **Gradients** — Linear + radial; GPUI only has linear.

## Summary: Where GPUI is Ahead

1. **Cross-platform** — Ships on macOS, Windows, Linux today. This is the #1 gap.
2. **IME** — Essential for international users and CJK input.
3. **Custom shaders** — Canvas element allows arbitrary GPU rendering.
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

### High Priority — Components
1. **Color picker** — Form control for color selection. Useful for creative/design apps.
2. **Date picker** — Calendar-based date selection. Common form need.
3. **Skeleton/Loading** — Placeholder components for async content loading.
4. **Avatar** — User avatar display with image, initials fallback, status indicator.
5. **Toast positioning** — Notifications with configurable placement (top/bottom/left/right/center).
6. **Scroll-to / programmatic scroll** — `cx.scroll_to(offset)` or element-based scroll targeting.

### High Priority — Platform & Infrastructure
7. **Windows platform shell** — Blocks adoption for the largest desktop platform. HWND, DX12/Vulkan backend via wgpu.
8. **Linux platform shell** — X11/Wayland support. Smaller audience but important for developer tools.
9. **IME support** — Blocks international users. Requires platform-level integration (TSM on macOS, IMM32 on Windows).
10. **Accessibility** — Screen reader support via platform accessibility APIs (NSAccessibility, UIA). Growing requirement.
11. **Custom shaders** — Canvas element equivalent for arbitrary GPU rendering.
