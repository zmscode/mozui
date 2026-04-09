use crate::traits::{PlatformWindow, TitlebarStyle};
use mozui_style::{Rect, Size};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WebCanvasWindowHandle, WebDisplayHandle, WindowHandle,
};
use std::ptr::NonNull;
use web_sys::HtmlCanvasElement;

/// A browser canvas acting as a mozui window.
pub struct WebWindow {
    canvas: HtmlCanvasElement,
    canvas_id: u32,
    titlebar_height: f32,
    titlebar_style: TitlebarStyle,
}

impl WebWindow {
    pub fn new(
        canvas: HtmlCanvasElement,
        canvas_id: u32,
        titlebar_style: TitlebarStyle,
        titlebar_height: f32,
    ) -> Self {
        Self {
            canvas,
            canvas_id,
            titlebar_height,
            titlebar_style,
        }
    }

    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }
}

impl PlatformWindow for WebWindow {
    fn bounds(&self) -> Rect {
        let rect = self.canvas.get_bounding_client_rect();
        Rect::new(
            rect.x() as f32,
            rect.y() as f32,
            rect.width() as f32,
            rect.height() as f32,
        )
    }

    fn set_bounds(&mut self, bounds: Rect) {
        let style = self.canvas.style();
        let _ = style.set_property("width", &format!("{}px", bounds.size.width));
        let _ = style.set_property("height", &format!("{}px", bounds.size.height));
    }

    fn content_size(&self) -> Size {
        Size::new(
            self.canvas.client_width() as f32,
            self.canvas.client_height() as f32,
        )
    }

    fn scale_factor(&self) -> f32 {
        web_sys::window()
            .map(|w| w.device_pixel_ratio() as f32)
            .unwrap_or(1.0)
    }

    fn is_focused(&self) -> bool {
        web_sys::window()
            .and_then(|w| w.document())
            .map(|d| d.has_focus().unwrap_or(false))
            .unwrap_or(false)
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn is_maximized(&self) -> bool {
        false
    }

    fn set_title(&mut self, title: &str) {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            doc.set_title(title);
        }
    }

    fn minimize(&mut self) {}
    fn maximize(&mut self) {}
    fn close(&mut self) {}

    fn request_redraw(&self) {
        // Redraw is driven by requestAnimationFrame in the event loop
    }

    fn begin_drag_move(&self) {}

    fn titlebar_height(&self) -> f32 {
        self.titlebar_height
    }

    fn titlebar_style(&self) -> TitlebarStyle {
        self.titlebar_style
    }
}

impl HasWindowHandle for WebWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = WebCanvasWindowHandle::new(
            NonNull::new(self.canvas_id as *mut std::ffi::c_void).unwrap(),
        );
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::WebCanvas(handle)) })
    }
}

impl HasDisplayHandle for WebWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let handle = WebDisplayHandle::new();
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Web(handle)) })
    }
}
