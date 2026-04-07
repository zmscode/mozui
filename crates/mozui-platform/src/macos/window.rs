use crate::traits::{PlatformWindow, WindowOptions};
use mozui_style::{Rect, Size};

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask, NSWindowTitleVisibility};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use raw_window_handle::{
    AppKitDisplayHandle, AppKitWindowHandle, DisplayHandle, HandleError, HasDisplayHandle,
    HasWindowHandle, WindowHandle,
};
use std::ptr::NonNull;

pub struct MacWindow {
    ns_window: Retained<NSWindow>,
    ns_view: Retained<objc2_app_kit::NSView>,
    scale_factor: f32,
}

impl MacWindow {
    pub fn new(mtm: MainThreadMarker, options: WindowOptions) -> Self {
        let content_rect = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(options.size.width as f64, options.size.height as f64),
        );

        let style = NSWindowStyleMask(
            NSWindowStyleMask::Borderless.0
                | NSWindowStyleMask::Resizable.0
                | NSWindowStyleMask::Miniaturizable.0
                | NSWindowStyleMask::Closable.0,
        );

        let ns_window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                content_rect,
                style,
                NSBackingStoreType::Buffered,
                false,
            )
        };

        ns_window.setTitlebarAppearsTransparent(true);
        ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        ns_window.setMovableByWindowBackground(false);

        let title = objc2_foundation::NSString::from_str(&options.title);
        ns_window.setTitle(&title);

        if let Some(min_size) = options.min_size {
            ns_window.setMinSize(NSSize::new(min_size.width as f64, min_size.height as f64));
        }

        ns_window.center();

        let ns_view = ns_window
            .contentView()
            .expect("Window must have content view");
        ns_view.setWantsLayer(true);

        let scale_factor = ns_window
            .screen()
            .map(|s| s.backingScaleFactor() as f32)
            .unwrap_or(1.0);

        if options.visible {
            ns_window.makeKeyAndOrderFront(None);
        }

        Self {
            ns_window,
            ns_view,
            scale_factor,
        }
    }
}

impl PlatformWindow for MacWindow {
    fn bounds(&self) -> Rect {
        let frame = self.ns_window.frame();
        Rect::new(
            frame.origin.x as f32,
            frame.origin.y as f32,
            frame.size.width as f32,
            frame.size.height as f32,
        )
    }

    fn set_bounds(&mut self, bounds: Rect) {
        let frame = NSRect::new(
            NSPoint::new(bounds.origin.x as f64, bounds.origin.y as f64),
            NSSize::new(bounds.size.width as f64, bounds.size.height as f64),
        );
        self.ns_window.setFrame_display(frame, true);
    }

    fn content_size(&self) -> Size {
        let frame = self.ns_view.frame();
        Size::new(frame.size.width as f32, frame.size.height as f32)
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    fn is_focused(&self) -> bool {
        self.ns_window.isKeyWindow()
    }

    fn set_title(&mut self, title: &str) {
        let ns_title = objc2_foundation::NSString::from_str(title);
        self.ns_window.setTitle(&ns_title);
    }

    fn minimize(&mut self) {
        self.ns_window.miniaturize(None);
    }

    fn maximize(&mut self) {
        self.ns_window.zoom(None);
    }

    fn close(&mut self) {
        self.ns_window.close();
    }

    fn request_redraw(&self) {
        self.ns_view.setNeedsDisplay(true);
    }
}

impl HasWindowHandle for MacWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let ptr = Retained::as_ptr(&self.ns_view) as *mut std::ffi::c_void;
        let non_null = NonNull::new(ptr).expect("NSView pointer must not be null");
        let handle = AppKitWindowHandle::new(non_null);
        Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
    }
}

impl HasDisplayHandle for MacWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let handle = AppKitDisplayHandle::new();
        Ok(unsafe { DisplayHandle::borrow_raw(handle.into()) })
    }
}
