use cocoa::base::{id, nil};
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Configuration for a sheet window.
pub struct SheetConfig {
    pub width: f64,
    pub height: f64,
}

impl Default for SheetConfig {
    fn default() -> Self {
        Self {
            width: 400.0,
            height: 300.0,
        }
    }
}

/// Show a modal sheet attached to the window.
///
/// `content_view` is an `NSView` to display in the sheet.
/// Pass `nil` for an empty sheet.
/// Returns the sheet `NSWindow`.
pub fn show_sheet(window: &Window, content_view: id, config: SheetConfig) -> id {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let ns_window: id = msg_send![ns_view, window];

        // NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskResizable
        let style_mask: usize = (1 << 0) | (1 << 1) | (1 << 3);
        let content_rect = cocoa::foundation::NSRect::new(
            cocoa::foundation::NSPoint::new(0.0, 0.0),
            cocoa::foundation::NSSize::new(config.width, config.height),
        );

        let sheet_window: id = msg_send![class!(NSWindow), alloc];
        let sheet_window: id = msg_send![sheet_window,
            initWithContentRect: content_rect
            styleMask: style_mask
            backing: 2_isize  // NSBackingStoreBuffered
            defer: false
        ];

        if content_view != nil {
            let _: () = msg_send![sheet_window, setContentView: content_view];
        }

        let _: () = msg_send![ns_window, beginSheet: sheet_window completionHandler: nil];

        sheet_window
    }
}

/// End a sheet that was previously shown.
pub fn end_sheet(window: &Window, sheet_window: id) {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let ns_window: id = msg_send![ns_view, window];
        let _: () = msg_send![ns_window, endSheet: sheet_window];
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
