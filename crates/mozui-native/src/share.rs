use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// An item to share.
pub enum ShareItem {
    /// A text string.
    Text(String),
    /// A file URL (path).
    FilePath(String),
    /// A URL string.
    Url(String),
}

/// Show a native share picker anchored to the window.
///
/// Opens `NSSharingServicePicker` with the given items.
/// The picker appears near the specified position (in window coordinates).
pub fn show_share_picker(window: &Window, items: &[ShareItem], anchor_x: f64, anchor_y: f64) {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let mut ns_items: Vec<id> = Vec::new();

        for item in items {
            let ns_item: id = match item {
                ShareItem::Text(text) => CocoaNSString::alloc(nil).init_str(text),
                ShareItem::FilePath(path) => {
                    let ns_path = CocoaNSString::alloc(nil).init_str(path);
                    msg_send![class!(NSURL), fileURLWithPath: ns_path]
                }
                ShareItem::Url(url) => {
                    let ns_str = CocoaNSString::alloc(nil).init_str(url);
                    msg_send![class!(NSURL), URLWithString: ns_str]
                }
            };
            ns_items.push(ns_item);
        }

        let items_arr: id = msg_send![
            class!(NSArray),
            arrayWithObjects: ns_items.as_ptr()
            count: ns_items.len()
        ];

        let picker: id = msg_send![class!(NSSharingServicePicker), alloc];
        let picker: id = msg_send![picker, initWithItems: items_arr];

        let rect = cocoa::foundation::NSRect::new(
            cocoa::foundation::NSPoint::new(anchor_x, anchor_y),
            cocoa::foundation::NSSize::new(1.0, 1.0),
        );

        let _: () = msg_send![picker,
            showRelativeToRect: rect
            ofView: ns_view
            preferredEdge: 3_isize  // NSRectEdgeMaxY (bottom)
        ];
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
