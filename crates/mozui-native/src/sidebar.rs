use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[repr(C)]
#[derive(Clone, Copy)]
struct NSEdgeInsets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

/// Configuration for a native sidebar installed on a mozui window.
pub struct SidebarConfig {
    /// Width of the sidebar in points.
    pub width: f64,
    /// Minimum width when resizing.
    pub min_width: f64,
    /// Maximum width when resizing.
    pub max_width: f64,
    /// Sections to display in the sidebar.
    pub sections: Vec<SidebarSection>,
}

/// A section in the sidebar (e.g. "Favourites", "Locations").
pub struct SidebarSection {
    pub title: String,
    pub items: Vec<SidebarItem>,
}

/// An item in a sidebar section.
pub struct SidebarItem {
    pub title: String,
    /// SF Symbol name for the icon.
    pub symbol: String,
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            width: 220.0,
            min_width: 160.0,
            max_width: 400.0,
            sections: vec![],
        }
    }
}

/// Installs an `NSSplitViewController` on the window, creating a sidebar pane
/// with proper sidebar behavior and a content pane containing the existing
/// mozui Metal view.
///
/// On macOS 26+, the sidebar automatically adopts Liquid Glass appearance.
pub fn install_sidebar(window: &Window, config: SidebarConfig) {
    let mozui_view = get_raw_ns_view(window);

    unsafe {
        let ns_window: id = msg_send![mozui_view, window];

        // Create the sidebar view controller
        let sidebar_vc: id = msg_send![class!(NSViewController), alloc];
        let sidebar_vc: id = msg_send![sidebar_vc, init];

        let sidebar_view: id = msg_send![class!(NSVisualEffectView), alloc];
        let sidebar_view: id = msg_send![sidebar_view, init];
        let _: () = msg_send![sidebar_view, setMaterial: 7_isize]; // Sidebar
        let _: () = msg_send![sidebar_view, setBlendingMode: 0_isize]; // BehindWindow
        let _: () = msg_send![sidebar_view, setState: 1_isize]; // Active

        // Populate sidebar with content
        populate_sidebar(sidebar_view, &config.sections);

        let _: () = msg_send![sidebar_vc, setView: sidebar_view];

        // Create the content view controller wrapping the mozui Metal view
        let content_vc: id = msg_send![class!(NSViewController), alloc];
        let content_vc: id = msg_send![content_vc, init];
        let content_view: id = msg_send![mozui_view, superview];
        let _: () = msg_send![content_vc, setView: content_view];

        // Create NSSplitViewItems
        let sidebar_item: id =
            msg_send![class!(NSSplitViewItem), sidebarWithViewController: sidebar_vc];
        let content_item: id =
            msg_send![class!(NSSplitViewItem), contentListWithViewController: content_vc];

        let _: () = msg_send![sidebar_item, setMinimumThickness: config.min_width];
        let _: () = msg_send![sidebar_item, setMaximumThickness: config.max_width];
        let _: () = msg_send![sidebar_item, setPreferredThicknessFraction: 0.2_f64];

        // Create NSSplitViewController
        let split_vc: id = msg_send![class!(NSSplitViewController), alloc];
        let split_vc: id = msg_send![split_vc, init];
        let _: () = msg_send![split_vc, addSplitViewItem: sidebar_item];
        let _: () = msg_send![split_vc, addSplitViewItem: content_item];

        let _: () = msg_send![ns_window, setContentViewController: split_vc];

        // Set initial sidebar position
        let split_view: id = msg_send![split_vc, splitView];
        let _: () = msg_send![
            split_view,
            setPosition: config.width
            ofDividerAtIndex: 0_isize
        ];
    }
}

/// Toggle the sidebar visibility with animation.
pub fn toggle_sidebar(window: &Window) {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let ns_window: id = msg_send![ns_view, window];
        let split_vc: id = msg_send![ns_window, contentViewController];
        if split_vc != nil {
            let _: () = msg_send![split_vc, toggleSidebar: nil];
        }
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}

fn populate_sidebar(sidebar_view: id, sections: &[SidebarSection]) {
    unsafe {
        // Create a vertical stack view to hold all sidebar content
        let stack: id = msg_send![class!(NSStackView), alloc];
        let stack: id = msg_send![stack, init];
        let _: () = msg_send![stack, setOrientation: 1_isize]; // Vertical
        let _: () = msg_send![stack, setAlignment: 5_isize]; // NSLayoutAttributeLeading
        let _: () = msg_send![stack, setSpacing: 2.0_f64];
        let _: () = msg_send![stack, setTranslatesAutoresizingMaskIntoConstraints: false];

        // Edge insets for the stack
        let _: () = msg_send![stack, setEdgeInsets: NSEdgeInsets {
            top: 8.0, left: 12.0, bottom: 8.0, right: 12.0,
        }];

        for (i, section) in sections.iter().enumerate() {
            if i > 0 {
                // Add spacing between sections
                let spacer: id = msg_send![class!(NSView), alloc];
                let spacer: id = msg_send![spacer, init];
                let _: () =
                    msg_send![spacer, setTranslatesAutoresizingMaskIntoConstraints: false];
                let _: () = msg_send![stack, addArrangedSubview: spacer];

                // Height constraint for spacer
                let spacer_height: id = msg_send![spacer, heightAnchor];
                let constraint: id =
                    msg_send![spacer_height, constraintEqualToConstant: 8.0_f64];
                let _: () = msg_send![constraint, setActive: true];
            }

            // Section header
            let header = create_section_header(&section.title);
            let _: () = msg_send![stack, addArrangedSubview: header];

            // Section items
            for item in &section.items {
                let row = create_sidebar_row(&item.title, &item.symbol);
                let _: () = msg_send![stack, addArrangedSubview: row];

                // Make rows fill width
                let row_leading: id = msg_send![row, leadingAnchor];
                let stack_leading: id = msg_send![stack, leadingAnchor];
                let constraint: id =
                    msg_send![row_leading, constraintEqualToAnchor: stack_leading];
                let _: () = msg_send![constraint, setActive: true];

                let row_trailing: id = msg_send![row, trailingAnchor];
                let stack_trailing: id = msg_send![stack, trailingAnchor];
                let constraint: id =
                    msg_send![row_trailing, constraintEqualToAnchor: stack_trailing];
                let _: () = msg_send![constraint, setActive: true];
            }
        }

        // Add stack view to sidebar
        let _: () = msg_send![sidebar_view, addSubview: stack];

        // Pin stack to sidebar edges
        let stack_top: id = msg_send![stack, topAnchor];
        let parent_top: id = msg_send![sidebar_view, topAnchor];
        let constraint: id = msg_send![stack_top, constraintEqualToAnchor: parent_top];
        let _: () = msg_send![constraint, setActive: true];

        let stack_leading: id = msg_send![stack, leadingAnchor];
        let parent_leading: id = msg_send![sidebar_view, leadingAnchor];
        let constraint: id = msg_send![stack_leading, constraintEqualToAnchor: parent_leading];
        let _: () = msg_send![constraint, setActive: true];

        let stack_trailing: id = msg_send![stack, trailingAnchor];
        let parent_trailing: id = msg_send![sidebar_view, trailingAnchor];
        let constraint: id =
            msg_send![stack_trailing, constraintEqualToAnchor: parent_trailing];
        let _: () = msg_send![constraint, setActive: true];
    }
}

fn create_section_header(title: &str) -> id {
    unsafe {
        let label: id = msg_send![class!(NSTextField), alloc];
        let label: id = msg_send![label, init];
        let ns_title = CocoaNSString::alloc(nil).init_str(title);
        let _: () = msg_send![label, setStringValue: ns_title];
        let _: () = msg_send![label, setBezeled: false];
        let _: () = msg_send![label, setDrawsBackground: false];
        let _: () = msg_send![label, setEditable: false];
        let _: () = msg_send![label, setSelectable: false];

        // Small, semibold, secondary label color
        let font: id = msg_send![class!(NSFont), systemFontOfSize: 11.0_f64 weight: 0.3_f64];
        let _: () = msg_send![label, setFont: font];
        let color: id = msg_send![class!(NSColor), secondaryLabelColor];
        let _: () = msg_send![label, setTextColor: color];

        let _: () = msg_send![label, setTranslatesAutoresizingMaskIntoConstraints: false];

        label
    }
}

fn create_sidebar_row(title: &str, symbol_name: &str) -> id {
    unsafe {
        let row: id = msg_send![class!(NSStackView), alloc];
        let row: id = msg_send![row, init];
        let _: () = msg_send![row, setOrientation: 0_isize]; // Horizontal
        let _: () = msg_send![row, setAlignment: 10_isize]; // NSLayoutAttributeCenterY
        let _: () = msg_send![row, setSpacing: 6.0_f64];
        let _: () = msg_send![row, setTranslatesAutoresizingMaskIntoConstraints: false];

        // Edge insets
        let _: () = msg_send![row, setEdgeInsets: NSEdgeInsets {
            top: 3.0, left: 8.0, bottom: 3.0, right: 8.0,
        }];

        // SF Symbol icon
        let ns_symbol = CocoaNSString::alloc(nil).init_str(symbol_name);
        let image: id = msg_send![
            class!(NSImage),
            imageWithSystemSymbolName: ns_symbol
            accessibilityDescription: nil
        ];

        if image != nil {
            let image_view: id = msg_send![class!(NSImageView), imageViewWithImage: image];
            let _: () =
                msg_send![image_view, setTranslatesAutoresizingMaskIntoConstraints: false];

            // Tint the icon with accent color
            let tint: id = msg_send![class!(NSColor), controlAccentColor];
            let _: () = msg_send![image_view, setContentTintColor: tint];

            // Size constraint
            let w_anchor: id = msg_send![image_view, widthAnchor];
            let constraint: id = msg_send![w_anchor, constraintEqualToConstant: 16.0_f64];
            let _: () = msg_send![constraint, setActive: true];
            let h_anchor: id = msg_send![image_view, heightAnchor];
            let constraint: id = msg_send![h_anchor, constraintEqualToConstant: 16.0_f64];
            let _: () = msg_send![constraint, setActive: true];

            let _: () = msg_send![row, addArrangedSubview: image_view];
        }

        // Text label
        let label: id = msg_send![class!(NSTextField), alloc];
        let label: id = msg_send![label, init];
        let ns_title = CocoaNSString::alloc(nil).init_str(title);
        let _: () = msg_send![label, setStringValue: ns_title];
        let _: () = msg_send![label, setBezeled: false];
        let _: () = msg_send![label, setDrawsBackground: false];
        let _: () = msg_send![label, setEditable: false];
        let _: () = msg_send![label, setSelectable: false];
        let _: () = msg_send![label, setTranslatesAutoresizingMaskIntoConstraints: false];

        let font: id = msg_send![class!(NSFont), systemFontOfSize: 13.0_f64];
        let _: () = msg_send![label, setFont: font];
        let color: id = msg_send![class!(NSColor), labelColor];
        let _: () = msg_send![label, setTextColor: color];

        let _: () = msg_send![row, addArrangedSubview: label];

        row
    }
}
