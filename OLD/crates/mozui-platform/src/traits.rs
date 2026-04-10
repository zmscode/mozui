use mozui_events::{PlatformEvent, WindowId};
use mozui_style::{Point, Rect, Size};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::cell::RefCell;

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

/// Platform abstraction for OS-specific window management and services.
pub trait Platform {
    fn run(&mut self, callback: EventCallback) -> !;
    fn open_window(&mut self, options: WindowOptions) -> (WindowId, Box<dyn PlatformWindow>);
    fn create_window(&self, options: WindowOptions) -> Box<dyn PlatformWindow>;
    fn screens(&self) -> Vec<Screen>;
    fn set_cursor(&self, cursor: mozui_events::CursorStyle);
    fn clipboard_read(&self) -> Option<String>;
    fn clipboard_write(&self, text: &str);
    fn open_url(&self, url: &str);
    fn open_file_dialog(&self, options: FileDialogOptions) -> Vec<std::path::PathBuf>;
    fn save_file_dialog(&self, options: FileDialogOptions) -> Option<std::path::PathBuf>;
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

// ── Thread-local platform services ──────────────────────────────
//
// Elements (text_input, link, etc.) need platform services during event
// dispatch but don't have access to the Platform instance. These thread-local
// service functions are installed once during App::run() and provide a stable
// interface for any code on the main thread.

struct PlatformServices {
    clipboard_read: Box<dyn Fn() -> Option<String>>,
    clipboard_write: Box<dyn Fn(&str)>,
    set_cursor: Box<dyn Fn(mozui_events::CursorStyle)>,
    open_url: Box<dyn Fn(&str)>,
    open_file_dialog: Box<dyn Fn(FileDialogOptions) -> Vec<std::path::PathBuf>>,
    save_file_dialog: Box<dyn Fn(FileDialogOptions) -> Option<std::path::PathBuf>>,
    create_window: Box<dyn Fn(WindowOptions) -> Box<dyn PlatformWindow>>,
}

thread_local! {
    static SERVICES: RefCell<Option<PlatformServices>> = const { RefCell::new(None) };
}

/// Install platform services for the current thread. Called once by App::run().
///
/// # Safety
/// The `platform` reference must outlive all subsequent calls to the free
/// functions (`clipboard_read`, `open_url`, etc.). In practice, the platform
/// is owned by `App::run()` which never returns.
pub fn install_services(platform: &dyn Platform) {
    // Erase the borrow lifetime to create 'static closures. This is safe
    // because the platform is owned by App::run() which never returns (-> !),
    // so it outlives all service calls.
    let platform: &'static dyn Platform = unsafe { std::mem::transmute(platform) };
    SERVICES.with(|cell| {
        *cell.borrow_mut() = Some(PlatformServices {
            clipboard_read: Box::new(move || platform.clipboard_read()),
            clipboard_write: Box::new(move |text| platform.clipboard_write(text)),
            set_cursor: Box::new(move |cursor| platform.set_cursor(cursor)),
            open_url: Box::new(move |url| platform.open_url(url)),
            open_file_dialog: Box::new(move |opts| platform.open_file_dialog(opts)),
            save_file_dialog: Box::new(move |opts| platform.save_file_dialog(opts)),
            create_window: Box::new(move |opts| platform.create_window(opts)),
        });
    });
}

fn with_services<R>(f: impl FnOnce(&PlatformServices) -> R) -> R {
    SERVICES.with(|cell| {
        let guard = cell.borrow();
        let services = guard
            .as_ref()
            .expect("Platform services not installed. Call install_services() first.");
        f(services)
    })
}

// ── Public free functions (delegate to thread-local services) ───

/// Read text from the system clipboard.
pub fn clipboard_read() -> Option<String> {
    with_services(|s| (s.clipboard_read)())
}

/// Write text to the system clipboard.
pub fn clipboard_write(text: &str) {
    with_services(|s| (s.clipboard_write)(text))
}

/// Set the mouse cursor style.
pub fn set_cursor_style(cursor: mozui_events::CursorStyle) {
    with_services(|s| (s.set_cursor)(cursor))
}

/// Open a URL in the default browser.
pub fn open_url(url: &str) {
    with_services(|s| (s.open_url)(url))
}

/// Show a native open file dialog. Returns selected file path(s), or empty if cancelled.
pub fn open_file_dialog(options: FileDialogOptions) -> Vec<std::path::PathBuf> {
    with_services(|s| (s.open_file_dialog)(options))
}

/// Show a native save file dialog. Returns the selected path, or None if cancelled.
pub fn save_file_dialog(options: FileDialogOptions) -> Option<std::path::PathBuf> {
    with_services(|s| (s.save_file_dialog)(options))
}

/// Create a new platform window.
pub fn create_window(options: WindowOptions) -> Box<dyn PlatformWindow> {
    with_services(|s| (s.create_window)(options))
}
