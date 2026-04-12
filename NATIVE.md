# mozui-native — Native macOS Component Roadmap

Native AppKit components wrapped as mozui elements and window-level APIs, providing seamless macOS integration with Liquid Glass support on macOS 26+.

## Current State

| Module | Component | AppKit Class | SwiftUI Equivalent |
|---|---|---|---|
| `button.rs` | Push button | `NSButton` | `Button` |
| `switch.rs` | Toggle switch | `NSSwitch` | `Toggle` |
| `text_field.rs` | Text input | `NSTextField` | `TextField` |
| `symbol.rs` | SF Symbols | `NSImageView` | `Image(systemName:)` |
| `visual_effect.rs` | Blur/vibrancy | `NSVisualEffectView` | `.background(.material)` |
| `glass_effect.rs` | Liquid Glass | `NSGlassEffectView` | Liquid Glass (macOS 26+) |
| `toolbar.rs` | Window toolbar | `NSToolbar` | `toolbar(content:)` |
| `sidebar.rs` | Source list sidebar | `NSSplitViewController` + `NSOutlineView` | `NavigationSplitView` |
| `breadcrumb.rs` | Path bar | `NSPathControl` | `NavigationPath` |
| `view.rs` | Window properties | `NSWindow` | Window modifiers |

## Implementation Phases

### Phase 1 — Core Interaction (Essential for real apps)

| Module | Component | AppKit Class | SwiftUI Equivalent | Priority |
|---|---|---|---|---|
| `menu.rs` | Menu + context menu | `NSMenu`, `NSMenuItem` | `Menu`, `contextMenu`, `CommandMenu` | P0 |
| `search.rs` | Toolbar search field | `NSSearchField` | `searchable(text:)` | P0 |
| `toolbar.rs` | Toolbar item groups | `NSToolbarItemGroup` | `ToolbarItemGroup` | P0 |
| `alert.rs` | Alerts + confirmation | `NSAlert` | `alert()`, `confirmationDialog()` | P0 |
| `popover.rs` | Anchored popover | `NSPopover` | `popover(isPresented:)` | P1 |

### Phase 2 — Controls & Data Display

| Module | Component | AppKit Class | SwiftUI Equivalent | Priority |
|---|---|---|---|---|
| `picker.rs` | Dropdown / popup | `NSPopUpButton` | `Picker` | P1 |
| `slider.rs` | Range slider | `NSSlider` | `Slider` | P1 |
| `progress.rs` | Progress indicator | `NSProgressIndicator` | `ProgressView` | P1 |
| `color_picker.rs` | Color selection | `NSColorWell` | `ColorPicker` | P2 |
| `table.rs` | Multi-column table | `NSTableView` | `Table` | P2 |

### Phase 3 — System Integration

| Module | Component | AppKit Class | SwiftUI Equivalent | Priority |
|---|---|---|---|---|
| `file_dialog.rs` | Open/Save panels | `NSOpenPanel`, `NSSavePanel` | `fileImporter`, `fileExporter` | P1 |
| `inspector.rs` | Inspector panel | `NSSplitViewItem` (inspector) | `inspector(isPresented:)` | P2 |
| `tab_view.rs` | Tabbed content | `NSTabViewController` | `TabView` | P2 |
| `stepper.rs` | Increment/decrement | `NSStepper` | `Stepper` | P2 |
| `date_picker.rs` | Date/time picker | `NSDatePicker` | `DatePicker` | P2 |

### Phase 4 — Advanced Patterns

| Module | Component | AppKit Class | SwiftUI Equivalent | Priority |
|---|---|---|---|---|
| `sheet.rs` | Modal sheets | `NSWindow.beginSheet` | `sheet(isPresented:)` | P2 |
| `share.rs` | Share menu | `NSSharingServicePicker` | `ShareLink` | P3 |
| `drag_drop.rs` | Drag & drop | `NSDraggingDestination` | `draggable()`, `dropDestination()` | P3 |

## Architecture Notes

- **Element-based components** (button, switch, etc.) implement `IntoElement` and render as `NSView` subviews positioned via the mozui layout engine.
- **Window-level APIs** (toolbar, sidebar, breadcrumb, view) operate on `&Window` and configure `NSWindow` / `NSViewController` hierarchy directly.
- **Delegate patterns** use `objc`/`cocoa` crates with `ClassDecl` + `Once` guard for ObjC class registration. Data stored via leaked `Box` pointers in ivars.
- **objc2 vs objc**: Element components use `objc2`/`objc2-app-kit`. Delegate-heavy components (toolbar, sidebar) use `cocoa`/`objc` due to `define_class!` limitations with `Retained<NSArray>` return types.
