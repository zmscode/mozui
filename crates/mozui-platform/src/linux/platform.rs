use crate::traits::{
    EventCallback, FileDialogOptions, Platform, PlatformWindow, Screen, WindowOptions,
};
use mozui_events::CursorStyle;
use std::path::PathBuf;

use super::wayland::WaylandPlatform;
use super::x11::X11Platform;

/// Runtime-dispatching Linux platform.
/// Checks `WAYLAND_DISPLAY` to decide between Wayland and X11.
pub enum LinuxPlatform {
    X11(X11Platform),
    Wayland(WaylandPlatform),
}

impl LinuxPlatform {
    pub fn new() -> Self {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            LinuxPlatform::Wayland(WaylandPlatform::new())
        } else {
            LinuxPlatform::X11(X11Platform::new())
        }
    }
}

impl Platform for LinuxPlatform {
    fn run(&mut self, callback: EventCallback) -> ! {
        match self {
            LinuxPlatform::X11(p) => p.run(callback),
            LinuxPlatform::Wayland(p) => p.run(callback),
        }
    }

    fn open_window(&mut self, options: WindowOptions) -> (mozui_events::WindowId, Box<dyn PlatformWindow>) {
        match self {
            LinuxPlatform::X11(p) => p.open_window(options),
            LinuxPlatform::Wayland(p) => p.open_window(options),
        }
    }

    fn create_window(&self, options: WindowOptions) -> Box<dyn PlatformWindow> {
        match self {
            LinuxPlatform::X11(p) => p.create_window(options),
            LinuxPlatform::Wayland(p) => p.create_window(options),
        }
    }

    fn screens(&self) -> Vec<Screen> {
        match self {
            LinuxPlatform::X11(p) => p.screens(),
            LinuxPlatform::Wayland(p) => p.screens(),
        }
    }

    fn set_cursor(&self, cursor: CursorStyle) {
        match self {
            LinuxPlatform::X11(p) => p.set_cursor(cursor),
            LinuxPlatform::Wayland(p) => p.set_cursor(cursor),
        }
    }

    fn clipboard_read(&self) -> Option<String> {
        match self {
            LinuxPlatform::X11(p) => p.clipboard_read(),
            LinuxPlatform::Wayland(p) => p.clipboard_read(),
        }
    }

    fn clipboard_write(&self, text: &str) {
        match self {
            LinuxPlatform::X11(p) => p.clipboard_write(text),
            LinuxPlatform::Wayland(p) => p.clipboard_write(text),
        }
    }

    fn open_url(&self, url: &str) {
        match self {
            LinuxPlatform::X11(p) => p.open_url(url),
            LinuxPlatform::Wayland(p) => p.open_url(url),
        }
    }

    fn open_file_dialog(&self, options: FileDialogOptions) -> Vec<PathBuf> {
        match self {
            LinuxPlatform::X11(p) => p.open_file_dialog(options),
            LinuxPlatform::Wayland(p) => p.open_file_dialog(options),
        }
    }

    fn save_file_dialog(&self, options: FileDialogOptions) -> Option<PathBuf> {
        match self {
            LinuxPlatform::X11(p) => p.save_file_dialog(options),
            LinuxPlatform::Wayland(p) => p.save_file_dialog(options),
        }
    }
}
