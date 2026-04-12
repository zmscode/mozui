use mozui::{Bounds, Pixels, Window};
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::NSRect;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Retrieves the parent `NSView` pointer from a mozui `Window`.
///
/// Uses the `raw-window-handle` API to extract the AppKit window handle,
/// which contains a pointer to the window's content view (the custom MozuiView).
pub fn parent_ns_view(window: &Window) -> &NSView {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => {
            let ptr = h.ns_view.as_ptr() as *const NSView;
            unsafe { &*ptr }
        }
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}

/// Converts mozui bounds (top-left origin) to an `NSRect` (bottom-left origin).
pub fn bounds_to_ns_rect(bounds: Bounds<Pixels>, parent_height: f64) -> NSRect {
    let x: f64 = bounds.origin.x.into();
    let y: f64 = bounds.origin.y.into();
    let w: f64 = bounds.size.width.into();
    let h: f64 = bounds.size.height.into();

    let flipped_y = parent_height - y - h;

    NSRect::new(
        objc2_foundation::NSPoint::new(x, flipped_y),
        objc2_foundation::NSSize::new(w, h),
    )
}

/// Manages the lifecycle of a native `NSView` subview within the mozui view hierarchy.
///
/// This struct is designed to be stored in `with_element_state` so that the native
/// view persists across frames. On drop, the view is removed from its parent.
pub struct NativeViewState {
    view: Retained<NSView>,
    attached: bool,
}

impl NativeViewState {
    /// Create a new `NativeViewState` wrapping the given native view.
    pub fn new(view: Retained<NSView>) -> Self {
        Self {
            view,
            attached: false,
        }
    }

    /// Returns a reference to the underlying `NSView`.
    pub fn view(&self) -> &NSView {
        &self.view
    }

    /// Attach the native view to the parent (if not already) and update its frame.
    ///
    /// Call this during `prepaint` each frame to keep the native view
    /// positioned correctly within the mozui layout.
    pub fn attach_and_position(&mut self, parent: &NSView, bounds: Bounds<Pixels>) {
        let parent_frame = parent.frame();
        let frame = bounds_to_ns_rect(bounds, parent_frame.size.height);

        if !self.attached {
            parent.addSubview(&self.view);
            self.attached = true;
        }

        self.view.setFrame(frame);
    }

    /// Set whether the native view is hidden.
    pub fn set_hidden(&self, hidden: bool) {
        self.view.setHidden(hidden);
    }
}

impl Drop for NativeViewState {
    fn drop(&mut self) {
        if self.attached {
            self.view.removeFromSuperview();
        }
    }
}
