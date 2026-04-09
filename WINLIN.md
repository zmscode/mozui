# Windows & Linux Platform Backends

Native platform backends for Windows and Linux, mirroring the GPUI approach: separate per-platform implementations using native APIs, no winit.

## Architecture

```
crates/mozui-platform/src/
  traits.rs            # Platform + PlatformWindow traits (existing)
  macos/               # Existing — objc2/AppKit
  web/                 # Existing — wasm-bindgen/WebGPU
  windows/             # New — `windows` crate / Win32
    mod.rs
    platform.rs        # WinPlatform: Platform impl
    window.rs          # WinWindow: PlatformWindow impl
  linux/               # New — x11rb + wayland-client
    mod.rs
    platform.rs        # LinuxPlatform: Platform impl (runtime X11/Wayland detection)
    window.rs          # LinuxWindow: PlatformWindow impl
    x11/
      mod.rs
      client.rs        # X11 connection, event loop, window management
      clipboard.rs     # X11 selections (CLIPBOARD, PRIMARY)
    wayland/
      mod.rs
      client.rs        # Wayland registry, globals, event queue
      clipboard.rs     # wl_data_device clipboard
```

## Reference: macOS Backend Surface Area

Each new backend must implement the same traits. Here's what exists:

**Platform trait** (9 methods):
| Method | macOS impl | Notes |
|---|---|---|
| `run(callback) -> !` | NSApplication manual event pump @ 60fps | Blocking loop |
| `open_window(options)` | NSWindow + NSView creation | Returns (WindowId, Box\<dyn PlatformWindow\>) |
| `create_window(options)` | Same as open but no ID registration | For dynamic windows |
| `screens()` | NSScreen enumeration | Bounds, work area, scale factor |
| `set_cursor(style)` | NSCursor::set() | 9 cursor variants |
| `clipboard_read()` | NSPasteboard generalPasteboard | String only |
| `clipboard_write(text)` | NSPasteboard setString | String only |
| `open_url(url)` | NSWorkspace openURL | Default browser |
| `open_file_dialog(opts)` | NSOpenPanel runModal | Blocking |
| `save_file_dialog(opts)` | NSSavePanel runModal | Blocking |

**PlatformWindow trait** (15 methods):
| Method | macOS impl | Notes |
|---|---|---|
| `bounds()` | NSWindow frame | Origin + size |
| `set_bounds(rect)` | setFrame_display | |
| `content_size()` | NSView frame size | Logical pixels |
| `scale_factor()` | backingScaleFactor | DPI |
| `is_focused()` | isKeyWindow | |
| `is_visible()` | isVisible | |
| `is_maximized()` | isZoomed | |
| `set_title(str)` | setTitle | |
| `minimize()` | miniaturize | |
| `maximize()` | zoom | |
| `close()` | close | |
| `request_redraw()` | setNeedsDisplay | |
| `begin_drag_move()` | performWindowDragWithEvent | Custom titlebars |
| `titlebar_height()` | Stored from options | |
| `titlebar_style()` | Stored from options | |

Plus `HasWindowHandle` and `HasDisplayHandle` from `raw-window-handle`.

**PlatformEvent enum** (12 variants):
MouseMove, MouseDown, MouseUp, ScrollWheel, KeyDown, KeyUp, WindowResize, WindowMove, WindowFocused, WindowBlurred, WindowCloseRequested, ScaleFactorChanged, RedrawRequested

---

## Phase 0: Windows Backend

### Crates
Following GPUI: use the `windows` crate (Microsoft's official Rust bindings).

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.61", features = [
    "Win32_UI_WindowsAndMessaging",
    "Win32_Graphics_Gdi",
    "Win32_System_LibraryLoader",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_HiDpi",
    "Win32_System_Ole",           # Clipboard, drag-drop
    "Win32_UI_Shell",             # File dialogs (IFileOpenDialog)
    "Win32_System_Com",           # COM initialization for dialogs
    "Win32_UI_Shell_Common",      # COMDLG_FILTERSPEC
] }
arboard = "3"  # Clipboard fallback (simpler than raw Win32 clipboard)
```

### 0a: Window creation (`windows/window.rs`)
- Register WNDCLASSEXW with custom WndProc
- CreateWindowExW with WS_OVERLAPPEDWINDOW
- Store HWND, track DPI via GetDpiForWindow
- Implement HasWindowHandle (Win32WindowHandle)
- Implement HasDisplayHandle (WindowsDisplayHandle)
- Map all 15 PlatformWindow methods to Win32 equivalents:
  - `content_size()` → GetClientRect
  - `scale_factor()` → GetDpiForWindow / 96.0
  - `is_focused()` → GetForegroundWindow == hwnd
  - `minimize()` → ShowWindow(SW_MINIMIZE)
  - `maximize()` → ShowWindow(SW_MAXIMIZE)
  - `close()` → DestroyWindow
  - `begin_drag_move()` → SendMessage(WM_NCLBUTTONDOWN, HTCAPTION)
  - `set_title()` → SetWindowTextW
  - `request_redraw()` → InvalidateRect

### 0b: Event loop (`windows/platform.rs`)
- PeekMessageW / TranslateMessage / DispatchMessageW loop
- WndProc callback routes Win32 messages to PlatformEvent:
  - WM_MOUSEMOVE → MouseMove
  - WM_LBUTTONDOWN/UP → MouseDown/Up (Left)
  - WM_RBUTTONDOWN/UP → MouseDown/Up (Right)
  - WM_MOUSEWHEEL → ScrollWheel
  - WM_KEYDOWN/UP → KeyDown/Up (MapVirtualKeyW for translation)
  - WM_SIZE → WindowResize
  - WM_MOVE → WindowMove
  - WM_SETFOCUS/KILLFOCUS → WindowFocused/Blurred
  - WM_CLOSE → WindowCloseRequested
  - WM_DPICHANGED → ScaleFactorChanged
- 60fps frame pump: MsgWaitForMultipleObjects with 16ms timeout
- Thread-local callback storage (WndProc can't capture state)

### 0c: Platform services
- Clipboard: `arboard` crate (cross-platform, handles Win32 clipboard internally)
- Cursor: SetCursor with LoadCursorW (IDC_ARROW, IDC_HAND, IDC_IBEAM, etc.)
- File dialogs: COM IFileOpenDialog / IFileSaveDialog (modern Vista+ dialogs)
- open_url: ShellExecuteW with "open" verb
- screens: EnumDisplayMonitors + GetMonitorInfoW

### Estimated scope
- `window.rs`: ~300 lines
- `platform.rs`: ~400 lines (event loop + WndProc + services)
- `mod.rs`: ~5 lines

---

## Phase 1: Linux Backend (X11)

X11 first (wider compatibility), Wayland second.

### Crates
Following GPUI: `x11rb` for X11 protocol, `xkbcommon` for keyboard.

```toml
[target.'cfg(target_os = "linux")'.dependencies]
x11rb = { version = "0.13", features = ["cursor", "resource_manager", "xkb", "randr", "xinput"] }
xkbcommon = "0.8"
arboard = "3"          # Clipboard (handles X11 selections)
ashpd = "0.10"         # XDG portals for file dialogs (works on both X11/Wayland)
calloop = "0.14"       # Event loop (same as GPUI)
```

### 1a: X11 window creation (`linux/x11/client.rs`, `linux/window.rs`)
- x11rb::connect() to X server
- CreateWindow with visual from screen root
- Map window, set WM_NAME, WM_CLASS, _NET_WM_NAME
- Implement HasWindowHandle (XlibWindowHandle or XcbWindowHandle)
- Implement HasDisplayHandle (XlibDisplayHandle or XcbDisplayHandle)
- PlatformWindow methods:
  - `content_size()` → GetGeometry reply
  - `scale_factor()` → Xft.dpi resource or _XSETTINGS
  - `is_focused()` → GetInputFocus
  - `minimize()` → ClientMessage _NET_WM_STATE + _NET_WM_STATE_HIDDEN
  - `maximize()` → ClientMessage _NET_WM_STATE + _NET_WM_STATE_MAXIMIZED
  - `close()` → DestroyWindow
  - `begin_drag_move()` → ClientMessage _NET_WM_MOVERESIZE
  - `set_title()` → ChangeProperty _NET_WM_NAME
  - `request_redraw()` → SendEvent Expose

### 1b: X11 event loop
- calloop EventLoop wrapping x11rb connection fd
- Poll X11 events via connection.poll_for_event():
  - ButtonPress/Release → MouseDown/Up
  - MotionNotify → MouseMove
  - KeyPress/Release → KeyDown/Up (via xkbcommon keymap)
  - ConfigureNotify → WindowResize / WindowMove
  - FocusIn/Out → WindowFocused/Blurred
  - ClientMessage WM_DELETE_WINDOW → WindowCloseRequested
  - Expose → RedrawRequested
- 60fps frame pump via calloop timer source

### 1c: Platform services
- Clipboard: `arboard` (handles X11 CLIPBOARD selection)
- Cursor: xcursor via x11rb cursor handle
- File dialogs: `ashpd` (XDG Desktop Portal — freedesktop standard, works on GNOME/KDE/etc.)
- open_url: xdg-open via std::process::Command
- screens: RandR extension (x11rb randr feature)

### Estimated scope
- `x11/client.rs`: ~400 lines
- `linux/window.rs`: ~250 lines
- `linux/platform.rs`: ~200 lines

---

## Phase 2: Wayland Support

Add as alternative to X11 within the linux/ module. Runtime detection: check WAYLAND_DISPLAY env var.

### Crates
```toml
wayland-client = "0.31"
wayland-protocols = { version = "0.32", features = ["client", "unstable"] }
wayland-protocols-wlr = "0.3"
```

### 2a: Wayland client (`linux/wayland/client.rs`)
- Connect to Wayland display
- Registry: bind wl_compositor, wl_shm, xdg_wm_base, wl_seat
- xdg_surface + xdg_toplevel for window
- wl_pointer / wl_keyboard for input
- Frame callback for vsync-driven redraw

### 2b: Runtime detection (`linux/platform.rs`)
```rust
pub fn create_linux_platform() -> Box<dyn Platform> {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        Box::new(WaylandPlatform::new())
    } else {
        Box::new(X11Platform::new())
    }
}
```

### 2c: Shared code
- Clipboard, file dialogs, open_url — shared between X11 and Wayland (arboard + ashpd + xdg-open all work on both)
- LinuxWindow enum wrapping either X11Window or WaylandWindow
- Keyboard handling via xkbcommon (works on both)

### Estimated scope
- `wayland/client.rs`: ~500 lines
- Updates to `linux/platform.rs`: ~50 lines

---

## Phase 3: Polish

- Custom titlebar support on Windows (WM_NCCALCSIZE, DwmExtendFrameIntoClientArea)
- CSD (client-side decorations) on Linux Wayland
- IME / text input support (WM_IME_* on Windows, XIM/IBus on Linux)
- HiDPI: per-monitor DPI on Windows, fractional scaling on Wayland
- Touch input events
- Multi-monitor: proper screen enumeration and window positioning

---

## Build / CI Notes

- Windows builds: `cargo build --target x86_64-pc-windows-msvc`
- Linux builds: `cargo build --target x86_64-unknown-linux-gnu`
- CI: GitHub Actions with `windows-latest` and `ubuntu-latest` runners
- Linux CI deps: `apt install libx11-dev libxkbcommon-dev` (for x11rb and xkbcommon)
- Cross-compilation from macOS: possible via `cross` crate but native CI is easier

## Execution Order

**Phase 0 first** — Windows has a simpler windowing model (single message loop, no display server protocol) and is the higher-demand target. Phase 1 (Linux X11) shares clipboard/dialog crates with Phase 0. Phase 2 (Wayland) builds on Phase 1 infrastructure.
