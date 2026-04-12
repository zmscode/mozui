use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// A tab definition for the tab view controller.
pub struct TabDefinition {
    pub title: String,
    pub identifier: String,
    /// Optional SF Symbol name for the tab icon.
    pub symbol: Option<String>,
}

impl TabDefinition {
    pub fn new(identifier: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            identifier: identifier.into(),
            symbol: None,
        }
    }

    pub fn symbol(mut self, name: impl Into<String>) -> Self {
        self.symbol = Some(name.into());
        self
    }
}

/// Tab view style.
pub enum TabViewStyle {
    /// Segmented control above content (default).
    SegmentedTop,
    /// Segmented control below content.
    SegmentedBottom,
    /// Toolbar-integrated tabs.
    Toolbar,
    /// No visible tab selector (programmatic switching only).
    Unspecified,
}

/// Install a native `NSTabViewController` on the window.
///
/// Each tab gets its own `NSViewController` with an empty view.
/// Returns the `NSTabViewController` for programmatic tab switching.
pub fn install_tab_view(window: &Window, tabs: &[TabDefinition], style: TabViewStyle) -> id {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let ns_window: id = msg_send![ns_view, window];

        let tab_vc: id = msg_send![class!(NSTabViewController), alloc];
        let tab_vc: id = msg_send![tab_vc, init];

        // NSTabViewControllerTabStyle: 0=SegmentedTop, 1=SegmentedBottom, 2=Toolbar, 3=Unspecified
        let tab_style: isize = match style {
            TabViewStyle::SegmentedTop => 0,
            TabViewStyle::SegmentedBottom => 1,
            TabViewStyle::Toolbar => 2,
            TabViewStyle::Unspecified => 3,
        };
        let _: () = msg_send![tab_vc, setTabStyle: tab_style];

        for tab in tabs {
            let child_vc: id = msg_send![class!(NSViewController), alloc];
            let child_vc: id = msg_send![child_vc, init];

            let child_view: id = msg_send![class!(NSView), alloc];
            let child_view: id = msg_send![child_view, init];
            let _: () = msg_send![child_vc, setView: child_view];

            let ns_title = CocoaNSString::alloc(nil).init_str(&tab.title);
            let _: () = msg_send![child_vc, setTitle: ns_title];

            // Create tab view item
            let ns_id = CocoaNSString::alloc(nil).init_str(&tab.identifier);
            let tab_item: id = msg_send![class!(NSTabViewItem), alloc];
            let tab_item: id = msg_send![tab_item, initWithIdentifier: ns_id];
            let _: () = msg_send![tab_item, setLabel: ns_title];
            let _: () = msg_send![tab_item, setViewController: child_vc];

            if let Some(ref symbol) = tab.symbol {
                let ns_symbol = CocoaNSString::alloc(nil).init_str(symbol);
                let image: id = msg_send![
                    class!(NSImage),
                    imageWithSystemSymbolName: ns_symbol
                    accessibilityDescription: nil
                ];
                if image != nil {
                    let _: () = msg_send![tab_item, setImage: image];
                }
            }

            let _: () = msg_send![tab_vc, addTabViewItem: tab_item];
        }

        let _: () = msg_send![ns_window, setContentViewController: tab_vc];

        tab_vc
    }
}

/// Select a tab by index.
pub fn select_tab(tab_vc: id, index: usize) {
    unsafe {
        let _: () = msg_send![tab_vc, setSelectedTabViewItemIndex: index as isize];
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
