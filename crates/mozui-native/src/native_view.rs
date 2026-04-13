#[cfg(target_os = "macos")]
mod platform {
    use mozui::{Bounds, Pixels, Window};
    use objc2::rc::Retained;
    use objc2_app_kit::NSView;
    use objc2_foundation::NSRect;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

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

    fn bounds_to_ns_rect(bounds: Bounds<Pixels>, parent_height: f64) -> NSRect {
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

    pub struct NativeViewState {
        view: Retained<NSView>,
        attached: bool,
    }

    impl NativeViewState {
        pub fn new(view: Retained<NSView>) -> Self {
            Self {
                view,
                attached: false,
            }
        }

        pub fn view(&self) -> &NSView {
            &self.view
        }

        pub fn attach_and_position(&mut self, parent: &NSView, bounds: Bounds<Pixels>) {
            let parent_frame = parent.frame();
            let frame = bounds_to_ns_rect(bounds, parent_frame.size.height);

            if !self.attached {
                parent.addSubview(&self.view);
                self.attached = true;
            }

            self.view.setFrame(frame);
        }

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
}

#[cfg(target_os = "ios")]
mod platform {
    use mozui::{Bounds, Pixels, Window};
    use objc2::rc::Retained;
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_ui_kit::UIView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    pub fn parent_ui_view(window: &Window) -> Option<&UIView> {
        let handle = HasWindowHandle::window_handle(window).ok()?;
        match handle.as_raw() {
            RawWindowHandle::UiKit(h) => {
                let ptr = h.ui_view.as_ptr() as *const UIView;
                Some(unsafe { &*ptr })
            }
            _ => unreachable!("expected UIKit window handle on iOS"),
        }
    }

    fn bounds_to_cg_rect(bounds: Bounds<Pixels>) -> CGRect {
        let x: f64 = bounds.origin.x.into();
        let y: f64 = bounds.origin.y.into();
        let width: f64 = bounds.size.width.into();
        let height: f64 = bounds.size.height.into();
        CGRect::new(CGPoint::new(x, y), CGSize::new(width, height))
    }

    pub struct NativeViewState {
        view: Retained<UIView>,
        attached: bool,
    }

    impl NativeViewState {
        pub fn new(view: Retained<UIView>) -> Self {
            Self {
                view,
                attached: false,
            }
        }

        pub fn view(&self) -> &UIView {
            &self.view
        }

        pub fn attach_and_position(&mut self, parent: &UIView, bounds: Bounds<Pixels>) {
            if !self.attached {
                parent.addSubview(&self.view);
                self.attached = true;
            }

            self.view.setFrame(bounds_to_cg_rect(bounds));
        }

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
}

pub use platform::*;
