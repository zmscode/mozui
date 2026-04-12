use cocoa::base::id;
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Configuration for an inspector panel.
pub struct InspectorConfig {
    /// Width of the inspector panel.
    pub width: f64,
    /// Whether the inspector is initially visible.
    pub is_visible: bool,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            width: 260.0,
            is_visible: false,
        }
    }
}

/// Install an inspector panel on the window's split view controller.
///
/// The inspector appears as a trailing panel in the NSSplitViewController hierarchy.
/// Requires the window to use a split view controller (e.g., from sidebar installation).
pub fn install_inspector(window: &Window, config: InspectorConfig) -> id {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let ns_window: id = msg_send![ns_view, window];
        let content_vc: id = msg_send![ns_window, contentViewController];

        // Create an empty view controller for the inspector content
        let inspector_vc: id = msg_send![class!(NSViewController), alloc];
        let inspector_vc: id = msg_send![inspector_vc, init];

        let inspector_view: id = msg_send![class!(NSView), alloc];
        let frame = cocoa::foundation::NSRect::new(
            cocoa::foundation::NSPoint::new(0.0, 0.0),
            cocoa::foundation::NSSize::new(config.width, 400.0),
        );
        let inspector_view: id = msg_send![inspector_view, initWithFrame: frame];
        let _: () = msg_send![inspector_vc, setView: inspector_view];

        // Create NSSplitViewItem with inspector behavior
        // NSSplitViewItemBehaviorDefault = 0, Sidebar = 1, ContentList = 2, Inspector = 3 (macOS 14+)
        let split_item: id =
            msg_send![class!(NSSplitViewItem), inspectorWithViewController: inspector_vc];

        let _: () = msg_send![split_item, setMinimumThickness: config.width];
        let _: () = msg_send![split_item, setMaximumThickness: config.width * 1.5];
        let _: () = msg_send![split_item, setCollapsed: !config.is_visible];

        // Add to split view controller
        let _: () = msg_send![content_vc, addSplitViewItem: split_item];

        split_item
    }
}

/// Toggle the inspector panel visibility.
pub fn toggle_inspector(inspector_item: id) {
    unsafe {
        let collapsed: bool = msg_send![inspector_item, isCollapsed];
        let _: () = msg_send![inspector_item, setCollapsed: !collapsed];
    }
}

/// Set inspector visibility.
pub fn set_inspector_visible(inspector_item: id, visible: bool) {
    unsafe {
        let _: () = msg_send![inspector_item, setCollapsed: !visible];
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
