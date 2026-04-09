mozui vs GPUI — Feature Coverage Comparison

### Architecture & Core

| Feature | GPUI | mozui | Notes |
|---------|------|-------|-------|
| Rendering backend | Metal / DX12 / Vulkan | wgpu (Metal) | GPUI has native per-platform backends; mozui uses wgpu abstraction (easier cross-platform later) |
| Layout engine | Custom Flexbox (Taffy-derived) | Taffy | Both Flexbox, no Grid |
| State management | Entity/Model system (ECS-like) | Signal arena (React hooks) | Different paradigms — GPUI is more Elm-like, mozui is more React-like |
| Text rendering | cosmic-text + HarfBuzz | font-kit + custom shaping | GPUI has proper shaping (BiDi, complex scripts); mozui is LTR-only |
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
| Shadows | Yes (box + text) | **Yes** (Gaussian blur via erf, matches GPUI) |
| Gradients | Yes (linear) | **Yes** (linear + radial) |
| Images | Yes | **No** |
| SVG | Yes (via resvg) | **No** (icons only via rasterized atlas) |
| Custom shaders | Yes (Canvas element) | **No** |
| Clip rects | Yes | Yes (software stack) |
| Paint caching / invalidation | Yes | **Yes** (layout caching, paint-only redraws) |

mozui now has shadows (Gaussian blur via erf, matching GPUI's approach) and gradients (linear + radial). Remaining gaps: images, full SVG, and custom shaders.

### Component Library

| Category | GPUI | mozui | Gap |
|----------|------|-------|-----|
| Primitives (div, text, icon, label) | Yes | **Yes** (4/4) | None |
| Form controls | Minimal (div + handlers) | **Strong** (7/9) | mozui ahead — GPUI doesn't ship buttons/checkboxes/sliders as library components |
| Data display (badges, tags, progress) | Not built-in | **Strong** (6/9) | mozui ahead |
| Navigation (tabs, breadcrumbs) | Not built-in | **Good** (3/4) | mozui ahead |
| Layout (accordion, collapsible) | Not built-in | **Good** (4/7) | mozui ahead |
| Overlays (modals, menus, popovers) | **Yes** (core feature) | **Good** (5/5) | Dialog, Menu, Tooltip, Popover, Notification |
| Lists/Tables | **Virtual list** (core) | List + VirtualList | Roughly even; GPUI's is battle-tested |

Key insight: GPUI provides **low-level primitives** (div, text, img, svg, canvas) and leaves component building to the application (Zed builds its own buttons, tabs, etc.). mozui ships a **component library** out of the box. These are different design philosophies — mozui's approach is more accessible for users who want ready-made components.

### Event Handling & Interaction

| Feature | GPUI | mozui |
|---------|------|-------|
| Click/hover/active | Yes | Yes |
| Keyboard events | Yes | Yes |
| Focus system | Yes (scopes + trapping) | Yes (scopes + trapping) |
| Drag-and-drop | **Yes** (files, text, custom) | **Yes** (DragId-based source/target matching) |
| IME / text input | **Yes** (complex input) | **No** |
| Context menus | **Yes** | **No** |
| Scroll physics | **Yes** (momentum) | **Yes** (momentum with deceleration) |
| Custom cursors | Yes | Yes |
| Keybindings/Actions | Yes | Yes |

IME is the main interaction gap. Drag-and-drop is now supported with DragId-based source/target matching and 5px activation threshold.

### Animation

| Feature | GPUI | mozui |
|---------|------|-------|
| Spring physics | Yes | Yes |
| Tween/transition | Yes | Yes |
| Easing functions | Yes (cubic bezier) | Yes (cubic bezier + presets) |
| Shared animation flag | N/A (entity-driven) | Yes (Rc<Cell<bool>>) |
| Animation hooks | Entity observers | `cx.use_animated()`, `cx.use_spring()` |

Roughly at parity here. mozui's hook-based approach is arguably more ergonomic for simple cases.

### Window & Overlay System

| Feature | GPUI | mozui |
|---------|------|-------|
| Custom window chrome | Yes | Yes (macOS) |
| Multi-window | **Yes** | **Yes** (WindowId routing, per-window state, dynamic open/close) |
| Modal dialogs | **Yes** (with backdrop) | **Yes** (focus trap, backdrop dismiss) |
| Popovers | **Yes** (anchored positioning) | Partial (infrastructure exists) |
| Menus (context/dropdown) | **Yes** | **Yes** (icons, shortcuts, separators) |
| Tooltips | **Yes** | **Yes** (placement, shortcuts, hover trigger) |
| Notification/toast | Not built-in | **Yes** (typed: Info/Success/Warning/Error, stacking, dismiss) |

mozui now has Dialog (with focus trap), Menu (with icons/shortcuts/separators), Tooltip (with placement/hover), and Notification (typed toasts with stacking, dismiss, and accent stripe). Multi-window is supported with WindowId-based event routing, per-window render state, and dynamic open/close via `cx.open_window()`. Context menus (right-click triggered) are the main remaining gap.

### Advanced Features

| Feature | GPUI | mozui |
|---------|------|-------|
| Accessibility (screen readers) | **Emerging** (via platform APIs) | **No** |
| Clipboard | Yes | Yes |
| File dialogs | **Yes** | **No** |
| use_memo / derived state | Entity observers | **No** (planned Phase 6) |
| use_effect / side effects | Subscriptions | **No** (planned Phase 6) |
| Paint caching | **Yes** | **Yes** (layout caching) |
| Hot reload | **No** | **No** |

---

## Summary: Where mozui is Ahead

1. **Component library** — 32 ready-to-use components vs GPUI's "bring your own". This is a genuine differentiator for DX.
2. **Hook-based API** — `cx.use_signal()`, `cx.use_animated()` feel more familiar to React developers than GPUI's entity system.
3. **Theme system** — 50+ design tokens with dark/light presets. GPUI leaves theming to the application.

## Summary: Where GPUI is Ahead

1. **Cross-platform** — Ships on macOS, Windows, Linux today. This is the #1 gap.
2. **Rendering features** — Images, full SVG rendering, custom shaders (Canvas element).
3. **Text rendering** — cosmic-text with HarfBuzz gives proper international text support.
4. **IME** — Essential for international users.
5. **Multi-window maturity** — GPUI's multi-window is battle-tested in Zed; mozui's is new infrastructure.
6. **Maturity** — Powers a real product (Zed) used by thousands daily.

## Recommended Priorities

Based on this comparison, the highest-impact remaining work for mozui (in order):

1. ~~**Overlay system** (Dialog, Menu, Tooltip, Notification)~~ — **Done**
2. ~~**Shadows + gradients** in renderer~~ — **Done** (Gaussian blur + linear/radial gradients)
3. ~~**Multi-window support**~~ — **Done** (WindowId routing, per-window state, dynamic open/close)
4. ~~**Scroll physics**~~ — **Done** (momentum with deceleration)
5. **Windows platform shell** — blocks adoption
6. **Image loading** — common need
7. **Select/Dropdown** — most-needed missing component
8. **IME support** — blocks international users
