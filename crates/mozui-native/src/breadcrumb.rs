use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// A path component in the breadcrumb bar.
pub struct BreadcrumbItem {
    pub title: String,
    /// SF Symbol name for the icon (e.g. "folder.fill", "internaldrive").
    pub symbol: Option<String>,
}

/// Configuration for a native path bar (breadcrumb) installed on a mozui window.
pub struct BreadcrumbConfig {
    /// Path components to display.
    pub items: Vec<BreadcrumbItem>,
    /// Height of the path bar in points.
    pub height: f64,
}

impl Default for BreadcrumbConfig {
    fn default() -> Self {
        Self {
            items: vec![],
            height: 28.0,
        }
    }
}

/// Installs an `NSPathControl` at the bottom of the content pane.
///
/// Must be called after `install_sidebar` if using a sidebar, so the
/// breadcrumb attaches to the content pane (not the full window).
pub fn install_breadcrumb(window: &Window, config: BreadcrumbConfig) {
    let ns_view = get_raw_ns_view(window);

    unsafe {
        // The mozui Metal view's superview is the content pane
        // (either the window's contentView or the split view's content pane)
        let content_pane: id = msg_send![ns_view, superview];

        // Create NSPathControl
        let path_control: id = msg_send![class!(NSPathControl), alloc];
        let path_control: id = msg_send![path_control, init];
        let _: () = msg_send![path_control, setPathStyle: 0_isize]; // NSPathStyleStandard
        let _: () =
            msg_send![path_control, setTranslatesAutoresizingMaskIntoConstraints: false];

        // Build path items
        let items = create_path_items(&config.items);
        let ns_array: id = msg_send![
            class!(NSArray),
            arrayWithObjects: items.as_ptr()
            count: items.len()
        ];
        let _: () = msg_send![path_control, setPathItems: ns_array];

        // Add to content pane
        let _: () = msg_send![content_pane, addSubview: path_control];

        // Pin to bottom of content pane, full width
        let bottom: id = msg_send![path_control, bottomAnchor];
        let parent_bottom: id = msg_send![content_pane, bottomAnchor];
        let constraint: id = msg_send![bottom, constraintEqualToAnchor: parent_bottom];
        let _: () = msg_send![constraint, setActive: true];

        let leading: id = msg_send![path_control, leadingAnchor];
        let parent_leading: id = msg_send![content_pane, leadingAnchor];
        let constraint: id = msg_send![leading, constraintEqualToAnchor: parent_leading];
        let _: () = msg_send![constraint, setActive: true];

        let trailing: id = msg_send![path_control, trailingAnchor];
        let parent_trailing: id = msg_send![content_pane, trailingAnchor];
        let constraint: id = msg_send![trailing, constraintEqualToAnchor: parent_trailing];
        let _: () = msg_send![constraint, setActive: true];

        let height_anchor: id = msg_send![path_control, heightAnchor];
        let constraint: id =
            msg_send![height_anchor, constraintEqualToConstant: config.height];
        let _: () = msg_send![constraint, setActive: true];
    }
}

fn create_path_items(items: &[BreadcrumbItem]) -> Vec<id> {
    items.iter().map(|item| create_path_item(item)).collect()
}

fn create_path_item(item: &BreadcrumbItem) -> id {
    unsafe {
        let path_item: id = msg_send![class!(NSPathControlItem), alloc];
        let path_item: id = msg_send![path_item, init];

        let ns_title = CocoaNSString::alloc(nil).init_str(&item.title);
        let _: () = msg_send![path_item, setTitle: ns_title];

        if let Some(symbol) = &item.symbol {
            let ns_symbol = CocoaNSString::alloc(nil).init_str(symbol);
            let image: id = msg_send![
                class!(NSImage),
                imageWithSystemSymbolName: ns_symbol
                accessibilityDescription: nil
            ];
            if image != nil {
                let _: () = msg_send![path_item, setImage: image];
            }
        }

        path_item
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
