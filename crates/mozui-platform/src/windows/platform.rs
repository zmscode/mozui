use crate::traits::{
    EventCallback, FileDialogOptions, Platform, PlatformWindow, Screen, TitlebarStyle,
    WindowOptions,
};
use mozui_events::{
    CursorStyle, Key, Modifiers, MouseButton, PlatformEvent, ScrollDelta, WindowId,
};
use mozui_style::{Point, Rect, Size};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmExtendFrameIntoClientArea, MARGINS};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, MAPVK_VK_TO_CHAR, VIRTUAL_KEY, VK_BACK, VK_DELETE, VK_DOWN, VK_END,
    VK_ESCAPE, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8,
    VK_F9, VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::window::WinWindow;

/// Per-window config accessible from WndProc via thread-local state.
#[derive(Clone)]
struct WindowConfig {
    min_size: Option<Size>,
    max_size: Option<Size>,
    titlebar_style: TitlebarStyle,
}

/// Thread-local storage for the event callback and window map.
/// WndProc is a C callback that can't capture Rust state, so we use thread-locals.
struct WndProcState {
    callback: EventCallback,
    window_map: HashMap<isize, WindowId>,
    window_configs: HashMap<isize, WindowConfig>,
}

thread_local! {
    static WNDPROC_STATE: RefCell<Option<WndProcState>> = const { RefCell::new(None) };
}

pub struct WinPlatform {
    next_window_id: u64,
    window_map: HashMap<isize, WindowId>,
    window_configs: HashMap<isize, WindowConfig>,
}

impl WinPlatform {
    pub fn new() -> Self {
        Self {
            next_window_id: 0,
            window_map: HashMap::new(),
            window_configs: HashMap::new(),
        }
    }

    fn allocate_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        id
    }
}

impl Platform for WinPlatform {
    fn run(&mut self, callback: EventCallback) -> ! {
        // Move window map and configs into thread-local state for WndProc access
        let window_map = std::mem::take(&mut self.window_map);
        let window_configs = std::mem::take(&mut self.window_configs);
        let window_ids: Vec<WindowId> = window_map.values().copied().collect();

        WNDPROC_STATE.with(|cell| {
            *cell.borrow_mut() = Some(WndProcState {
                callback,
                window_map,
                window_configs,
            });
        });

        // Initial draw
        for &wid in &window_ids {
            dispatch_event(wid, PlatformEvent::RedrawRequested);
        }

        // Win32 message loop with 60fps frame pump
        let mut msg = MSG::default();
        loop {
            // Drain all pending messages
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        std::process::exit(0);
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            // Request redraw each frame
            for &wid in &window_ids {
                dispatch_event(wid, PlatformEvent::RedrawRequested);
            }

            // Wait up to ~16ms for the next message (60fps)
            unsafe {
                MsgWaitForMultipleObjects(None, false, 16, QS_ALLINPUT);
            }
        }
    }

    fn open_window(&mut self, options: WindowOptions) -> (WindowId, Box<dyn PlatformWindow>) {
        let hwnd = create_win32_window(&options);
        let key = hwnd.0 as isize;
        let id = self.allocate_window_id();
        self.window_map.insert(key, id);
        self.window_configs.insert(
            key,
            WindowConfig {
                min_size: options.min_size,
                max_size: options.max_size,
                titlebar_style: options.titlebar,
            },
        );
        let window = WinWindow::new(hwnd, &options);
        (id, Box::new(window))
    }

    fn create_window(&self, options: WindowOptions) -> Box<dyn PlatformWindow> {
        let hwnd = create_win32_window(&options);
        Box::new(WinWindow::new(hwnd, &options))
    }

    fn screens(&self) -> Vec<Screen> {
        let mut screens = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(
                None,
                None,
                Some(monitor_enum_proc),
                LPARAM(&mut screens as *mut Vec<Screen> as isize),
            );
        }
        screens
    }

    fn set_cursor(&self, cursor: CursorStyle) {
        let cursor_id = match cursor {
            CursorStyle::Arrow => IDC_ARROW,
            CursorStyle::Hand => IDC_HAND,
            CursorStyle::Text => IDC_IBEAM,
            CursorStyle::Crosshair => IDC_CROSS,
            CursorStyle::NotAllowed => IDC_NO,
            CursorStyle::ResizeNS => IDC_SIZENS,
            CursorStyle::ResizeEW => IDC_SIZEWE,
            CursorStyle::ResizeNESW => IDC_SIZENESW,
            CursorStyle::ResizeNWSE => IDC_SIZENWSE,
        };
        unsafe {
            let hcursor = LoadCursorW(None, cursor_id).unwrap_or_default();
            SetCursor(Some(hcursor));
        }
    }

    fn clipboard_read(&self) -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }

    fn clipboard_write(&self, text: &str) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    }

    fn open_url(&self, url: &str) {
        let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let verb: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            windows::Win32::UI::Shell::ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(wide.as_ptr()),
                None,
                None,
                SW_SHOW,
            );
        }
    }

    fn open_file_dialog(&self, _options: FileDialogOptions) -> Vec<PathBuf> {
        // TODO: COM IFileOpenDialog
        Vec::new()
    }

    fn save_file_dialog(&self, _options: FileDialogOptions) -> Option<PathBuf> {
        // TODO: COM IFileSaveDialog
        None
    }
}

// ── Window creation ────────────────────────────────────────────

fn create_win32_window(options: &WindowOptions) -> HWND {
    static REGISTERED: std::sync::Once = std::sync::Once::new();

    let class_name_str: Vec<u16> = "MozuiWindow\0".encode_utf16().collect();
    let class_name = PCWSTR(class_name_str.as_ptr());

    REGISTERED.call_once(|| unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW failed");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassExW(&wc);
    });

    let title: Vec<u16> = options
        .title
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let hinstance = unsafe { GetModuleHandleW(None).expect("GetModuleHandleW failed") };

    let style = match options.titlebar {
        crate::traits::TitlebarStyle::Native => WS_OVERLAPPEDWINDOW,
        crate::traits::TitlebarStyle::Transparent => WS_OVERLAPPEDWINDOW,
        crate::traits::TitlebarStyle::Hidden => WS_POPUP | WS_THICKFRAME | WS_SYSMENU,
    };

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            PCWSTR(title.as_ptr()),
            style
                | if options.visible {
                    WS_VISIBLE
                } else {
                    WINDOW_STYLE::default()
                },
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            options.size.width as i32,
            options.size.height as i32,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW failed")
    };

    // For Transparent titlebar, extend DWM frame into client area so content
    // renders behind the caption. A 1px top margin is enough to activate DWM
    // composition while WM_NCCALCSIZE removes the standard non-client area.
    if options.titlebar == crate::traits::TitlebarStyle::Transparent {
        let margins = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 1,
            cyBottomHeight: 0,
        };
        unsafe {
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        }
    }

    hwnd
}

// ── WndProc ────────────────────────────────────────────────────

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_MOUSEMOVE => {
            let pos = lparam_to_point(lparam, hwnd);
            let mods = get_modifiers();
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::MouseMove {
                    position: pos,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let pos = lparam_to_point(lparam, hwnd);
            let mods = get_modifiers();
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::MouseDown {
                    button: MouseButton::Left,
                    position: pos,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let pos = lparam_to_point(lparam, hwnd);
            let mods = get_modifiers();
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::MouseUp {
                    button: MouseButton::Left,
                    position: pos,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            let pos = lparam_to_point(lparam, hwnd);
            let mods = get_modifiers();
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::MouseDown {
                    button: MouseButton::Right,
                    position: pos,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            let pos = lparam_to_point(lparam, hwnd);
            let mods = get_modifiers();
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::MouseUp {
                    button: MouseButton::Right,
                    position: pos,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_MBUTTONDOWN => {
            let pos = lparam_to_point(lparam, hwnd);
            let mods = get_modifiers();
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::MouseDown {
                    button: MouseButton::Middle,
                    position: pos,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_MBUTTONUP => {
            let pos = lparam_to_point(lparam, hwnd);
            let mods = get_modifiers();
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::MouseUp {
                    button: MouseButton::Middle,
                    position: pos,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let pos = lparam_to_point(lparam, hwnd);
            let mods = get_modifiers();
            let delta_raw = (wparam.0 >> 16) as i16;
            let dy = delta_raw as f32 / WHEEL_DELTA as f32;
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::ScrollWheel {
                    delta: ScrollDelta::Lines(0.0, -dy),
                    position: pos,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_MOUSEHWHEEL => {
            let pos = lparam_to_point(lparam, hwnd);
            let mods = get_modifiers();
            let delta_raw = (wparam.0 >> 16) as i16;
            let dx = delta_raw as f32 / WHEEL_DELTA as f32;
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::ScrollWheel {
                    delta: ScrollDelta::Lines(dx, 0.0),
                    position: pos,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = VIRTUAL_KEY(wparam.0 as u16);
            let key = vk_to_key(vk);
            let mods = get_modifiers();
            let is_repeat = (lparam.0 & (1 << 30)) != 0;
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::KeyDown {
                    key,
                    modifiers: mods,
                    is_repeat,
                },
            );
            LRESULT(0)
        }
        WM_KEYUP | WM_SYSKEYUP => {
            let vk = VIRTUAL_KEY(wparam.0 as u16);
            let key = vk_to_key(vk);
            let mods = get_modifiers();
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::KeyUp {
                    key,
                    modifiers: mods,
                },
            );
            LRESULT(0)
        }
        WM_SIZE => {
            let width = (lparam.0 & 0xFFFF) as u16 as f32;
            let height = ((lparam.0 >> 16) & 0xFFFF) as u16 as f32;
            let dpi = unsafe { GetDpiForWindow(hwnd) as f32 / 96.0 };
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::WindowResize {
                    size: Size::new(width / dpi, height / dpi),
                },
            );
            LRESULT(0)
        }
        WM_MOVE => {
            let x = (lparam.0 & 0xFFFF) as i16 as f32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
            dispatch_hwnd_event(
                hwnd,
                PlatformEvent::WindowMove {
                    position: Point::new(x, y),
                },
            );
            LRESULT(0)
        }
        WM_SETFOCUS => {
            dispatch_hwnd_event(hwnd, PlatformEvent::WindowFocused);
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            dispatch_hwnd_event(hwnd, PlatformEvent::WindowBlurred);
            LRESULT(0)
        }
        WM_CLOSE => {
            dispatch_hwnd_event(hwnd, PlatformEvent::WindowCloseRequested);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let new_dpi = (wparam.0 & 0xFFFF) as u16;
            let scale = new_dpi as f32 / 96.0;
            // Resize window to the suggested rectangle
            let suggested = unsafe { &*(lparam.0 as *const RECT) };
            unsafe {
                let _ = MoveWindow(
                    hwnd,
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    true,
                );
            }
            dispatch_hwnd_event(hwnd, PlatformEvent::ScaleFactorChanged { scale });
            LRESULT(0)
        }
        WM_NCCALCSIZE => {
            if wparam.0 != 0 {
                let is_transparent = WNDPROC_STATE.with(|cell| {
                    if let Ok(state) = cell.try_borrow() {
                        if let Some(ref s) = *state {
                            let key = hwnd.0 as isize;
                            return s
                                .window_configs
                                .get(&key)
                                .map(|c| c.titlebar_style == TitlebarStyle::Transparent)
                                .unwrap_or(false);
                        }
                    }
                    false
                });
                if is_transparent {
                    // Return 0 to remove the standard non-client area (caption + borders).
                    // DwmExtendFrameIntoClientArea preserves the DWM shadow.
                    return LRESULT(0);
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_GETMINMAXINFO => {
            let config = WNDPROC_STATE.with(|cell| {
                if let Ok(state) = cell.try_borrow() {
                    if let Some(ref s) = *state {
                        let key = hwnd.0 as isize;
                        return s.window_configs.get(&key).cloned();
                    }
                }
                None
            });
            if let Some(config) = config {
                let info = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };
                let dpi = unsafe { GetDpiForWindow(hwnd) as f32 / 96.0 };
                if let Some(min) = config.min_size {
                    info.ptMinTrackSize.x = (min.width * dpi) as i32;
                    info.ptMinTrackSize.y = (min.height * dpi) as i32;
                }
                if let Some(max) = config.max_size {
                    info.ptMaxTrackSize.x = (max.width * dpi) as i32;
                    info.ptMaxTrackSize.y = (max.height * dpi) as i32;
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

// ── Event dispatch helpers ─────────────────────────────────────

fn dispatch_event(window_id: WindowId, event: PlatformEvent) {
    WNDPROC_STATE.with(|cell| {
        if let Ok(mut state) = cell.try_borrow_mut() {
            if let Some(ref mut s) = *state {
                (s.callback)(window_id, event);
            }
        }
    });
}

fn dispatch_hwnd_event(hwnd: HWND, event: PlatformEvent) {
    WNDPROC_STATE.with(|cell| {
        if let Ok(mut state) = cell.try_borrow_mut() {
            if let Some(ref mut s) = *state {
                let key = hwnd.0 as isize;
                let wid = s.window_map.get(&key).copied().unwrap_or(WindowId::MAIN);
                (s.callback)(wid, event);
            }
        }
    });
}

// ── Input helpers ──────────────────────────────────────────────

fn lparam_to_point(lparam: LPARAM, hwnd: HWND) -> Point {
    let x = (lparam.0 & 0xFFFF) as i16 as f32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
    let dpi = unsafe { GetDpiForWindow(hwnd) as f32 / 96.0 };
    Point::new(x / dpi, y / dpi)
}

fn get_modifiers() -> Modifiers {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
    unsafe {
        Modifiers {
            shift: GetKeyState(0x10) < 0, // VK_SHIFT
            ctrl: GetKeyState(0x11) < 0,  // VK_CONTROL
            alt: GetKeyState(0x12) < 0,   // VK_MENU
            meta: GetKeyState(0x5B) < 0 || GetKeyState(0x5C) < 0, // VK_LWIN | VK_RWIN
        }
    }
}

fn vk_to_key(vk: VIRTUAL_KEY) -> Key {
    match vk {
        VK_RETURN => Key::Enter,
        VK_ESCAPE => Key::Escape,
        VK_TAB => Key::Tab,
        VK_BACK => Key::Backspace,
        VK_DELETE => Key::Delete,
        VK_SPACE => Key::Space,
        VK_UP => Key::ArrowUp,
        VK_DOWN => Key::ArrowDown,
        VK_LEFT => Key::ArrowLeft,
        VK_RIGHT => Key::ArrowRight,
        VK_HOME => Key::Home,
        VK_END => Key::End,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,
        VK_F1 => Key::F1,
        VK_F2 => Key::F2,
        VK_F3 => Key::F3,
        VK_F4 => Key::F4,
        VK_F5 => Key::F5,
        VK_F6 => Key::F6,
        VK_F7 => Key::F7,
        VK_F8 => Key::F8,
        VK_F9 => Key::F9,
        VK_F10 => Key::F10,
        VK_F11 => Key::F11,
        VK_F12 => Key::F12,
        _ => {
            let ch = unsafe { MapVirtualKeyW(vk.0 as u32, MAPVK_VK_TO_CHAR) };
            if ch > 0 {
                if let Some(c) = char::from_u32(ch) {
                    if !c.is_control() {
                        return Key::Character(c.to_ascii_lowercase());
                    }
                }
            }
            Key::Unknown
        }
    }
}

// ── Monitor enumeration ────────────────────────────────────────

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> windows::core::BOOL {
    let screens = unsafe { &mut *(lparam.0 as *mut Vec<Screen>) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(hmonitor, &mut info).as_bool() } {
        let bounds = rect_to_rect(info.rcMonitor);
        let work_area = rect_to_rect(info.rcWork);
        let mut dpi_x: u32 = 96;
        let mut dpi_y: u32 = 96;
        let _ = unsafe { GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
        screens.push(Screen {
            bounds,
            work_area,
            scale_factor: dpi_x as f32 / 96.0,
        });
    }
    true.into()
}

fn rect_to_rect(r: RECT) -> Rect {
    Rect::new(
        r.left as f32,
        r.top as f32,
        (r.right - r.left) as f32,
        (r.bottom - r.top) as f32,
    )
}
