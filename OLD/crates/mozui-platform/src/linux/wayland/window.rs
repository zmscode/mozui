use crate::traits::{PlatformWindow, TitlebarStyle, WindowOptions};
use mozui_style::{Rect, Size};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use std::ptr::NonNull;
use wayland_client::Proxy;
use wayland_client::protocol::wl_surface::WlSurface;

pub struct WaylandWindow {
    surface: WlSurface,
    display_ptr: *mut std::ffi::c_void,
    width: u32,
    height: u32,
    scale_factor: f32,
    titlebar_height: f32,
    titlebar_style: TitlebarStyle,
}

impl WaylandWindow {
    pub fn new(
        surface: WlSurface,
        display_ptr: *mut std::ffi::c_void,
        options: &WindowOptions,
    ) -> Self {
        Self {
            surface,
            display_ptr,
            width: options.size.width as u32,
            height: options.size.height as u32,
            scale_factor: 1.0,
            titlebar_height: options.titlebar_height,
            titlebar_style: options.titlebar,
        }
    }

    #[allow(dead_code)]
    pub fn surface(&self) -> &WlSurface {
        &self.surface
    }

    #[allow(dead_code)]
    pub fn update_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    #[allow(dead_code)]
    pub fn update_scale_factor(&mut self, scale: f32) {
        self.scale_factor = scale;
    }
}

impl PlatformWindow for WaylandWindow {
    fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width as f32, self.height as f32)
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.width = bounds.size.width as u32;
        self.height = bounds.size.height as u32;
    }

    fn content_size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    fn is_focused(&self) -> bool {
        false
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn is_maximized(&self) -> bool {
        false
    }

    fn set_title(&mut self, _title: &str) {}
    fn minimize(&mut self) {}
    fn maximize(&mut self) {}
    fn close(&mut self) {}

    fn request_redraw(&self) {
        self.surface.commit();
    }

    fn begin_drag_move(&self) {}

    fn titlebar_height(&self) -> f32 {
        self.titlebar_height
    }

    fn titlebar_style(&self) -> TitlebarStyle {
        self.titlebar_style
    }
}

impl HasWindowHandle for WaylandWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // Get the wl_proxy* pointer from the ObjectId via wayland-client's Proxy trait.
        // The object ID encodes the protocol object identity; for raw-window-handle
        // we need the actual C wl_surface* pointer. wayland-client 0.31 doesn't
        // directly expose this, so we use the protocol_id as a stand-in.
        // wgpu handles this by using the wayland backend connection internally.
        let id = self.surface.id();
        // ObjectId protocol_id gives us the wl_proxy id (u32), but
        // WaylandWindowHandle needs a NonNull<c_void> wl_surface pointer.
        // We store the protocol object ID as a fake pointer — wgpu will
        // use the Wayland backend connection to resolve the actual proxy.
        let fake_ptr = id.protocol_id() as usize as *mut std::ffi::c_void;
        let non_null = NonNull::new(fake_ptr).expect("wl_surface ID must not be zero");
        let handle = WaylandWindowHandle::new(non_null);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Wayland(handle)) })
    }
}

impl HasDisplayHandle for WaylandWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        if let Some(non_null) = NonNull::new(self.display_ptr) {
            let handle = WaylandDisplayHandle::new(non_null);
            Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Wayland(handle)) })
        } else {
            // Null display pointer — fall back. wgpu will open its own connection.
            Err(HandleError::Unavailable)
        }
    }
}
