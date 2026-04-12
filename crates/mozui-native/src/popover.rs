use cocoa::base::{id, nil};
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Edge to which the popover arrow points.
pub enum PopoverEdge {
    Top,
    Left,
    Bottom,
    Right,
}

/// Popover behavior when clicking outside.
pub enum PopoverBehavior {
    /// Closes when clicking outside.
    Transient,
    /// Stays open; must be closed programmatically.
    ApplicationDefined,
    /// Closes when interacting with another window.
    Semitransient,
}

/// Configuration for a native popover.
pub struct PopoverConfig {
    /// Size of the popover content area.
    pub width: f64,
    pub height: f64,
    /// Preferred edge for the popover arrow.
    pub edge: PopoverEdge,
    /// Close behavior.
    pub behavior: PopoverBehavior,
}

impl Default for PopoverConfig {
    fn default() -> Self {
        Self {
            width: 300.0,
            height: 200.0,
            edge: PopoverEdge::Bottom,
            behavior: PopoverBehavior::Transient,
        }
    }
}

/// Show a popover anchored to the window's content view.
///
/// `content_view` is an `NSView` to display inside the popover.
/// Pass `nil` to create an empty popover with the configured size.
pub fn show_popover(window: &Window, content_view: id, config: PopoverConfig) -> id {
    let ns_view = get_raw_ns_view(window);

    unsafe {
        let popover: id = msg_send![class!(NSPopover), alloc];
        let popover: id = msg_send![popover, init];

        let behavior: isize = match config.behavior {
            PopoverBehavior::ApplicationDefined => 0,
            PopoverBehavior::Transient => 1,
            PopoverBehavior::Semitransient => 2,
        };
        let _: () = msg_send![popover, setBehavior: behavior];

        let size = cocoa::foundation::NSSize::new(config.width, config.height);
        let _: () = msg_send![popover, setContentSize: size];

        // Create a view controller for the content
        let vc: id = msg_send![class!(NSViewController), alloc];
        let vc: id = msg_send![vc, init];

        if content_view != nil {
            let _: () = msg_send![vc, setView: content_view];
        } else {
            let empty: id = msg_send![class!(NSView), alloc];
            let frame =
                cocoa::foundation::NSRect::new(cocoa::foundation::NSPoint::new(0.0, 0.0), size);
            let empty: id = msg_send![empty, initWithFrame: frame];
            let _: () = msg_send![vc, setView: empty];
        }

        let _: () = msg_send![popover, setContentViewController: vc];

        let edge: isize = match config.edge {
            PopoverEdge::Top => 1,    // NSRectEdgeMinY
            PopoverEdge::Left => 0,   // NSRectEdgeMinX
            PopoverEdge::Bottom => 3, // NSRectEdgeMaxY
            PopoverEdge::Right => 2,  // NSRectEdgeMaxX
        };

        let bounds: cocoa::foundation::NSRect = msg_send![ns_view, bounds];
        let _: () = msg_send![popover,
            showRelativeToRect: bounds
            ofView: ns_view
            preferredEdge: edge
        ];

        popover
    }
}

/// Close a popover.
pub fn close_popover(popover: id) {
    unsafe {
        let _: () = msg_send![popover, close];
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
