use mozui_events::{PlatformEvent, WindowId};
use mozui_style::{Point, Rect, Size};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// Title bar style for a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitlebarStyle {
    /// Native system title bar with standard decorations.
    #[default]
    Native,
    /// Transparent title bar — content extends behind it.
    /// Traffic light buttons remain visible on macOS.
    Transparent,
    /// Fully hidden title bar — no native chrome at all.
    /// You must provide your own window controls.
    Hidden,
}

pub struct WindowOptions {
    pub title: String,
    pub size: Size,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub position: Option<Point>,
    pub resizable: bool,
    pub visible: bool,
    pub titlebar: TitlebarStyle,
    /// Height of the titlebar area in logical points (macOS only).
    /// Traffic light buttons will be vertically centered in this area.
    /// Defaults to 38.0.
    pub titlebar_height: f32,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "mozui".to_string(),
            size: Size::new(800.0, 600.0),
            min_size: None,
            max_size: None,
            position: None,
            resizable: true,
            visible: true,
            titlebar: TitlebarStyle::default(),
            titlebar_height: 38.0,
        }
    }
}

pub struct Screen {
    pub bounds: Rect,
    pub work_area: Rect,
    pub scale_factor: f32,
}

/// Callback that receives platform events tagged with the target window.
pub type EventCallback = Box<dyn FnMut(WindowId, PlatformEvent)>;

/// Platform abstraction for OS-specific window management.
pub trait Platform {
    fn run(&mut self, callback: EventCallback) -> !;
    fn open_window(&mut self, options: WindowOptions) -> (WindowId, Box<dyn PlatformWindow>);
    fn screens(&self) -> Vec<Screen>;
    fn set_cursor(&self, cursor: mozui_events::CursorStyle);
    fn clipboard_read(&self) -> Option<String>;
    fn clipboard_write(&self, text: &str);
}

/// Read text from the system clipboard.
pub fn clipboard_read() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
        let pasteboard = NSPasteboard::generalPasteboard();
        let nstype = unsafe { NSPasteboardTypeString };
        return pasteboard.stringForType(nstype).map(|s| s.to_string());
    }
    #[cfg(not(target_os = "macos"))]
    None
}

/// Write text to the system clipboard.
pub fn clipboard_write(text: &str) {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
        use objc2_foundation::NSString;
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        let ns_string = NSString::from_str(text);
        let nstype = unsafe { NSPasteboardTypeString };
        let _ = pasteboard.setString_forType(&ns_string, nstype);
    }
}

/// Set the cursor globally (can be called from anywhere on macOS).
pub fn set_cursor_style(cursor: mozui_events::CursorStyle) {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSCursor;
        let ns_cursor = match cursor {
            mozui_events::CursorStyle::Arrow => NSCursor::arrowCursor(),
            mozui_events::CursorStyle::Hand => NSCursor::pointingHandCursor(),
            mozui_events::CursorStyle::Text => NSCursor::IBeamCursor(),
            mozui_events::CursorStyle::Crosshair => NSCursor::crosshairCursor(),
            mozui_events::CursorStyle::NotAllowed => NSCursor::operationNotAllowedCursor(),
            #[allow(deprecated)]
            mozui_events::CursorStyle::ResizeNS => NSCursor::resizeUpDownCursor(),
            #[allow(deprecated)]
            mozui_events::CursorStyle::ResizeEW => NSCursor::resizeLeftRightCursor(),
            mozui_events::CursorStyle::ResizeNESW | mozui_events::CursorStyle::ResizeNWSE => {
                NSCursor::crosshairCursor()
            }
        };
        ns_cursor.set();
    }
}

/// Open a URL in the default browser.
pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::{NSString, NSURL};
        let ns_string = NSString::from_str(url);
        if let Some(ns_url) = NSURL::URLWithString(&ns_string) {
            let workspace = objc2_app_kit::NSWorkspace::sharedWorkspace();
            workspace.openURL(&ns_url);
        }
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn();
    }
}

/// Options for a file dialog.
#[derive(Debug, Clone, Default)]
pub struct FileDialogOptions {
    /// Dialog title.
    pub title: Option<String>,
    /// Allowed file type extensions (e.g. ["png", "jpg"]).
    pub allowed_types: Vec<String>,
    /// Whether to allow selecting multiple files (open dialog only).
    pub multiple: bool,
    /// Whether to allow selecting directories instead of files.
    pub directories: bool,
    /// Default filename for save dialogs.
    pub default_name: Option<String>,
}

/// Show a native open file dialog. Returns selected file path(s), or empty if cancelled.
pub fn open_file_dialog(options: FileDialogOptions) -> Vec<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
        use objc2_foundation::NSString;

        let mtm = objc2::MainThreadMarker::new()
            .expect("File dialogs must be called from the main thread");
        let panel = NSOpenPanel::openPanel(mtm);
        panel.setCanChooseFiles(!options.directories);
        panel.setCanChooseDirectories(options.directories);
        panel.setAllowsMultipleSelection(options.multiple);

        if let Some(title) = &options.title {
            panel.setTitle(Some(&NSString::from_str(title)));
        }

        let response = panel.runModal();
        if response == NSModalResponseOK {
            let urls = panel.URLs();
            let mut paths = Vec::new();
            for i in 0..urls.len() {
                if let Some(path) = urls.objectAtIndex(i).path() {
                    paths.push(std::path::PathBuf::from(path.to_string()));
                }
            }
            return paths;
        }
        Vec::new()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = options;
        Vec::new()
    }
}

/// Show a native save file dialog. Returns the selected path, or None if cancelled.
pub fn save_file_dialog(options: FileDialogOptions) -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSModalResponseOK, NSSavePanel};
        use objc2_foundation::NSString;

        let mtm = objc2::MainThreadMarker::new()
            .expect("File dialogs must be called from the main thread");
        let panel = NSSavePanel::savePanel(mtm);

        if let Some(title) = &options.title {
            panel.setTitle(Some(&NSString::from_str(title)));
        }
        if let Some(name) = &options.default_name {
            panel.setNameFieldStringValue(&NSString::from_str(name));
        }

        let response = panel.runModal();
        if response == NSModalResponseOK {
            if let Some(url) = panel.URL() {
                if let Some(path) = url.path() {
                    return Some(std::path::PathBuf::from(path.to_string()));
                }
            }
        }
        None
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = options;
        None
    }
}

/// Create a new platform window (can be called from the event loop).
pub fn create_window(options: WindowOptions) -> Box<dyn PlatformWindow> {
    #[cfg(target_os = "macos")]
    {
        let mtm = objc2::MainThreadMarker::new().expect("Must be called from the main thread");
        Box::new(crate::macos::window::MacWindow::new(mtm, options))
    }
    #[cfg(not(target_os = "macos"))]
    {
        panic!("Unsupported platform");
    }
}

/// Handle to a platform-native window.
pub trait PlatformWindow: HasWindowHandle + HasDisplayHandle {
    fn bounds(&self) -> Rect;
    fn set_bounds(&mut self, bounds: Rect);
    fn content_size(&self) -> Size;
    fn scale_factor(&self) -> f32;
    fn is_focused(&self) -> bool;
    fn is_visible(&self) -> bool;
    fn is_maximized(&self) -> bool;
    fn set_title(&mut self, title: &str);
    fn minimize(&mut self);
    fn maximize(&mut self);
    fn close(&mut self);
    fn request_redraw(&self);
    /// Start a window drag operation (for custom title bars).
    fn begin_drag_move(&self);
    /// Height of the titlebar area in logical points.
    fn titlebar_height(&self) -> f32;
    /// The titlebar style this window was created with.
    fn titlebar_style(&self) -> TitlebarStyle;
}
