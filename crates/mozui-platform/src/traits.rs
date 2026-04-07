use mozui_events::PlatformEvent;
use mozui_style::{Point, Rect, Size};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct WindowOptions {
    pub title: String,
    pub size: Size,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub position: Option<Point>,
    pub resizable: bool,
    pub visible: bool,
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
        }
    }
}

pub struct Screen {
    pub bounds: Rect,
    pub work_area: Rect,
    pub scale_factor: f32,
}

/// Callback that receives platform events.
pub type EventCallback = Box<dyn FnMut(PlatformEvent)>;

/// Platform abstraction for OS-specific window management.
pub trait Platform {
    fn run(&mut self, callback: EventCallback) -> !;
    fn open_window(&mut self, options: WindowOptions) -> Box<dyn PlatformWindow>;
    fn screens(&self) -> Vec<Screen>;
    fn set_cursor(&self, cursor: mozui_events::CursorStyle);
    fn clipboard_read(&self) -> Option<String>;
    fn clipboard_write(&self, text: &str);
}

/// Handle to a platform-native window.
pub trait PlatformWindow: HasWindowHandle + HasDisplayHandle {
    fn bounds(&self) -> Rect;
    fn set_bounds(&mut self, bounds: Rect);
    fn content_size(&self) -> Size;
    fn scale_factor(&self) -> f32;
    fn is_focused(&self) -> bool;
    fn set_title(&mut self, title: &str);
    fn minimize(&mut self);
    fn maximize(&mut self);
    fn close(&mut self);
    fn request_redraw(&self);
}
