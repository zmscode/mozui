use crate::traits::{PlatformWindow, TitlebarStyle, WindowOptions};
use mozui_style::{Rect, Size};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
use std::num::NonZeroIsize;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::InvalidateRect;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, GetClientRect, GetForegroundWindow, GetWindowRect, HTCAPTION, IsWindowVisible,
    IsZoomed, MoveWindow, SendMessageW, SetWindowTextW, ShowWindow, SW_MAXIMIZE, SW_MINIMIZE,
    WM_NCLBUTTONDOWN,
};

pub(crate) struct WinWindow {
    hwnd: HWND,
    titlebar_height: f32,
    titlebar_style: TitlebarStyle,
}

impl WinWindow {
    pub fn new(hwnd: HWND, options: &WindowOptions) -> Self {
        Self {
            hwnd,
            titlebar_height: options.titlebar_height,
            titlebar_style: options.titlebar,
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Raw HWND as isize for use as HashMap key.
    pub fn hwnd_isize(&self) -> isize {
        self.hwnd.0 as isize
    }
}

impl PlatformWindow for WinWindow {
    fn bounds(&self) -> Rect {
        let mut rect = RECT::default();
        unsafe {
            let _ = GetWindowRect(self.hwnd, &mut rect);
        }
        Rect::new(
            rect.left as f32,
            rect.top as f32,
            (rect.right - rect.left) as f32,
            (rect.bottom - rect.top) as f32,
        )
    }

    fn set_bounds(&mut self, bounds: Rect) {
        unsafe {
            let _ = MoveWindow(
                self.hwnd,
                bounds.origin.x as i32,
                bounds.origin.y as i32,
                bounds.size.width as i32,
                bounds.size.height as i32,
                true,
            );
        }
    }

    fn content_size(&self) -> Size {
        let mut rect = RECT::default();
        unsafe {
            let _ = GetClientRect(self.hwnd, &mut rect);
        }
        let dpi = self.scale_factor();
        Size::new(
            (rect.right - rect.left) as f32 / dpi,
            (rect.bottom - rect.top) as f32 / dpi,
        )
    }

    fn scale_factor(&self) -> f32 {
        unsafe { GetDpiForWindow(self.hwnd) as f32 / 96.0 }
    }

    fn is_focused(&self) -> bool {
        unsafe { GetForegroundWindow() == self.hwnd }
    }

    fn is_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.hwnd).as_bool() }
    }

    fn is_maximized(&self) -> bool {
        unsafe { IsZoomed(self.hwnd).as_bool() }
    }

    fn set_title(&mut self, title: &str) {
        let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let _ = SetWindowTextW(self.hwnd, windows::core::PCWSTR(wide.as_ptr()));
        }
    }

    fn minimize(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_MINIMIZE);
        }
    }

    fn maximize(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_MAXIMIZE);
        }
    }

    fn close(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }

    fn request_redraw(&self) {
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    fn begin_drag_move(&self) {
        unsafe {
            let _ = SendMessageW(
                self.hwnd,
                WM_NCLBUTTONDOWN,
                Some(windows::Win32::Foundation::WPARAM(HTCAPTION as usize)),
                Some(windows::Win32::Foundation::LPARAM(0)),
            );
        }
    }

    fn titlebar_height(&self) -> f32 {
        self.titlebar_height
    }

    fn titlebar_style(&self) -> TitlebarStyle {
        self.titlebar_style
    }
}

impl HasWindowHandle for WinWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let hwnd_isize = self.hwnd.0 as isize;
        let handle =
            Win32WindowHandle::new(NonZeroIsize::new(hwnd_isize).expect("HWND must not be null"));
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

impl HasDisplayHandle for WinWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let handle = WindowsDisplayHandle::new();
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Windows(handle)) })
    }
}
