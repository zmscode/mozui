use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::declare::ClassDecl;
use objc::runtime::{BOOL, Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::os::raw::c_void;
use std::sync::Once;

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
#[derive(Clone)]
pub struct SidebarSection {
    pub title: String,
    pub items: Vec<SidebarItem>,
}

/// An item in a sidebar section.
#[derive(Clone)]
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
/// with an `NSOutlineView` in source list style and a content pane containing
/// the existing mozui Metal view.
///
/// The source list style gives native sidebar appearance: proper row heights,
/// selection highlighting, section headers, and Liquid Glass on macOS 26+.
pub fn install_sidebar(window: &Window, config: SidebarConfig) {
    let mozui_view = get_raw_ns_view(window);

    unsafe {
        let ns_window: id = msg_send![mozui_view, window];

        // Create the sidebar view controller with an NSOutlineView
        let sidebar_vc: id = msg_send![class!(NSViewController), alloc];
        let sidebar_vc: id = msg_send![sidebar_vc, init];

        let sidebar_view = create_outline_sidebar(&config.sections);
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

// --- NSOutlineView source list sidebar ---

/// Pre-created stable item objects for the outline view.
/// Group items use tag -(section+1), child items use (section<<16)|item_idx.
struct SidebarData {
    sections: Vec<SidebarSection>,
    /// One NSNumber per section header (value = -(section+1))
    group_items: Vec<id>,
    /// One NSNumber per item, indexed [section][item] (value = (section<<16)|item)
    child_items: Vec<Vec<id>>,
}

static REGISTER_SIDEBAR_DS: Once = Once::new();
static mut SIDEBAR_DS_CLASS: *const Class = std::ptr::null();

const SIDEBAR_DATA_IVAR: &str = "_sidebarData";

fn create_outline_sidebar(sections: &[SidebarSection]) -> id {
    unsafe {
        // Create scroll view
        let scroll_view: id = msg_send![class!(NSScrollView), alloc];
        let scroll_view: id = msg_send![scroll_view, init];
        let _: () = msg_send![scroll_view, setHasVerticalScroller: true];
        let _: () = msg_send![scroll_view, setDrawsBackground: false];
        let _: () = msg_send![scroll_view, setAutohidesScrollers: true];

        // Create outline view
        let outline: id = msg_send![class!(NSOutlineView), alloc];
        let outline: id = msg_send![outline, init];
        let _: () = msg_send![outline, setStyle: 1_isize]; // NSTableViewStyleSourceList
        let _: () = msg_send![outline, setSelectionHighlightStyle: 6_isize]; // SourceList
        let _: () = msg_send![outline, setHeaderView: nil];
        let _: () = msg_send![outline, setFloatsGroupRows: false];
        let _: () = msg_send![outline, setIndentationPerLevel: 0.0_f64];
        let _: () = msg_send![outline, setRowSizeStyle: 2_isize]; // Medium

        // Create a single column
        let col_id = CocoaNSString::alloc(nil).init_str("SidebarColumn");
        let column: id = msg_send![class!(NSTableColumn), alloc];
        let column: id = msg_send![column, initWithIdentifier: col_id];
        let _: () = msg_send![column, setEditable: false];
        let _: () = msg_send![outline, addTableColumn: column];
        let _: () = msg_send![outline, setOutlineTableColumn: column];

        // Pre-create stable item objects
        let mut group_items = Vec::new();
        let mut child_items = Vec::new();
        for (si, section) in sections.iter().enumerate() {
            let group_tag = -((si as isize) + 1);
            let group: id = msg_send![class!(NSNumber), numberWithInteger: group_tag];
            group_items.push(group);

            let mut items = Vec::new();
            for ii in 0..section.items.len() {
                let item_tag = ((si as isize) << 16) | (ii as isize);
                let item: id = msg_send![class!(NSNumber), numberWithInteger: item_tag];
                items.push(item);
            }
            child_items.push(items);
        }

        let data = SidebarData {
            sections: sections.to_vec(),
            group_items,
            child_items,
        };

        let ds = create_sidebar_data_source(data);
        let _: () = msg_send![outline, setDataSource: ds];
        let _: () = msg_send![outline, setDelegate: ds];

        // Put outline in scroll view
        let _: () = msg_send![scroll_view, setDocumentView: outline];

        // Expand all sections
        let _: () = msg_send![outline, expandItem: nil expandChildren: true];

        scroll_view
    }
}

fn create_sidebar_data_source(data: SidebarData) -> id {
    unsafe {
        REGISTER_SIDEBAR_DS.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiSidebarDataSource", superclass).unwrap();

            decl.add_ivar::<*mut c_void>(SIDEBAR_DATA_IVAR);

            // numberOfChildrenOfItem:
            extern "C" fn number_of_children(
                this: &Object,
                _sel: Sel,
                _outline: id,
                item: id,
            ) -> isize {
                unsafe {
                    let data = get_data(this);
                    if item == nil {
                        data.sections.len() as isize
                    } else {
                        let tag: isize = msg_send![item, integerValue];
                        if tag < 0 {
                            // Group item: return number of children
                            let si = ((-tag) - 1) as usize;
                            data.sections[si].items.len() as isize
                        } else {
                            0
                        }
                    }
                }
            }

            // child:ofItem:
            extern "C" fn child_of_item(
                this: &Object,
                _sel: Sel,
                _outline: id,
                index: isize,
                item: id,
            ) -> id {
                unsafe {
                    let data = get_data(this);
                    if item == nil {
                        data.group_items[index as usize]
                    } else {
                        let tag: isize = msg_send![item, integerValue];
                        let si = ((-tag) - 1) as usize;
                        data.child_items[si][index as usize]
                    }
                }
            }

            // isItemExpandable:
            extern "C" fn is_expandable(_this: &Object, _sel: Sel, _outline: id, item: id) -> BOOL {
                unsafe {
                    if item == nil {
                        return true;
                    }
                    let tag: isize = msg_send![item, integerValue];
                    tag < 0
                }
            }

            // isGroupItem:
            extern "C" fn is_group_item(_this: &Object, _sel: Sel, _outline: id, item: id) -> BOOL {
                unsafe {
                    if item == nil {
                        return false;
                    }
                    let tag: isize = msg_send![item, integerValue];
                    tag < 0
                }
            }

            // viewForTableColumn:item:
            extern "C" fn view_for_item(
                this: &Object,
                _sel: Sel,
                outline: id,
                _column: id,
                item: id,
            ) -> id {
                unsafe {
                    let data = get_data(this);
                    let tag: isize = msg_send![item, integerValue];

                    if tag < 0 {
                        let si = ((-tag) - 1) as usize;
                        create_header_cell(outline, &data.sections[si].title)
                    } else {
                        let si = (tag >> 16) as usize;
                        let ii = (tag & 0xFFFF) as usize;
                        let sidebar_item = &data.sections[si].items[ii];
                        create_item_cell(outline, &sidebar_item.title, &sidebar_item.symbol)
                    }
                }
            }

            // shouldSelectItem:
            extern "C" fn should_select(_this: &Object, _sel: Sel, _outline: id, item: id) -> BOOL {
                unsafe {
                    let tag: isize = msg_send![item, integerValue];
                    tag >= 0
                }
            }

            decl.add_method(
                sel!(outlineView:numberOfChildrenOfItem:),
                number_of_children as extern "C" fn(&Object, Sel, id, id) -> isize,
            );
            decl.add_method(
                sel!(outlineView:child:ofItem:),
                child_of_item as extern "C" fn(&Object, Sel, id, isize, id) -> id,
            );
            decl.add_method(
                sel!(outlineView:isItemExpandable:),
                is_expandable as extern "C" fn(&Object, Sel, id, id) -> BOOL,
            );
            decl.add_method(
                sel!(outlineView:isGroupItem:),
                is_group_item as extern "C" fn(&Object, Sel, id, id) -> BOOL,
            );
            decl.add_method(
                sel!(outlineView:viewForTableColumn:item:),
                view_for_item as extern "C" fn(&Object, Sel, id, id, id) -> id,
            );
            decl.add_method(
                sel!(outlineView:shouldSelectItem:),
                should_select as extern "C" fn(&Object, Sel, id, id) -> BOOL,
            );

            SIDEBAR_DS_CLASS = decl.register();
        });

        let cls = SIDEBAR_DS_CLASS;
        let ds: id = msg_send![cls, alloc];
        let ds: id = msg_send![ds, init];

        let data_box = Box::new(data);
        let data_ptr = Box::into_raw(data_box) as *mut c_void;
        (*ds).set_ivar(SIDEBAR_DATA_IVAR, data_ptr);

        ds
    }
}

fn get_data<'a>(obj: &Object) -> &'a SidebarData {
    unsafe {
        let ptr: *mut c_void = *obj.get_ivar(SIDEBAR_DATA_IVAR);
        &*(ptr as *const SidebarData)
    }
}

fn create_header_cell(outline: id, title: &str) -> id {
    unsafe {
        let cell_id = CocoaNSString::alloc(nil).init_str("HeaderCell");
        let mut cell: id = msg_send![outline, makeViewWithIdentifier: cell_id owner: nil];
        if cell == nil {
            cell = msg_send![class!(NSTableCellView), alloc];
            cell = msg_send![cell, init];
            let _: () = msg_send![cell, setIdentifier: cell_id];

            let label: id = msg_send![class!(NSTextField),
                labelWithString: CocoaNSString::alloc(nil).init_str("")];
            let _: () = msg_send![label, setTranslatesAutoresizingMaskIntoConstraints: false];
            let font: id = msg_send![class!(NSFont), systemFontOfSize: 11.0_f64 weight: 0.3_f64];
            let _: () = msg_send![label, setFont: font];
            let color: id = msg_send![class!(NSColor), secondaryLabelColor];
            let _: () = msg_send![label, setTextColor: color];
            let _: () = msg_send![cell, addSubview: label];
            let _: () = msg_send![cell, setTextField: label];

            let l: id = msg_send![label, leadingAnchor];
            let pl: id = msg_send![cell, leadingAnchor];
            let c: id = msg_send![l, constraintEqualToAnchor: pl constant: 4.0_f64];
            let _: () = msg_send![c, setActive: true];
            let cy: id = msg_send![label, centerYAnchor];
            let pcy: id = msg_send![cell, centerYAnchor];
            let c: id = msg_send![cy, constraintEqualToAnchor: pcy];
            let _: () = msg_send![c, setActive: true];
        }

        let tf: id = msg_send![cell, textField];
        let ns_title = CocoaNSString::alloc(nil).init_str(title);
        let _: () = msg_send![tf, setStringValue: ns_title];
        cell
    }
}

fn create_item_cell(outline: id, title: &str, symbol_name: &str) -> id {
    unsafe {
        let cell_id = CocoaNSString::alloc(nil).init_str("ItemCell");
        let mut cell: id = msg_send![outline, makeViewWithIdentifier: cell_id owner: nil];
        if cell == nil {
            cell = msg_send![class!(NSTableCellView), alloc];
            cell = msg_send![cell, init];
            let _: () = msg_send![cell, setIdentifier: cell_id];

            // Image view
            let placeholder: id = msg_send![class!(NSImage), alloc];
            let placeholder: id = msg_send![placeholder, init];
            let iv: id = msg_send![class!(NSImageView), imageViewWithImage: placeholder];
            let _: () = msg_send![iv, setTranslatesAutoresizingMaskIntoConstraints: false];
            let tint: id = msg_send![class!(NSColor), controlAccentColor];
            let _: () = msg_send![iv, setContentTintColor: tint];
            let _: () = msg_send![cell, addSubview: iv];
            let _: () = msg_send![cell, setImageView: iv];

            let w: id = msg_send![iv, widthAnchor];
            let c: id = msg_send![w, constraintEqualToConstant: 18.0_f64];
            let _: () = msg_send![c, setActive: true];
            let h: id = msg_send![iv, heightAnchor];
            let c: id = msg_send![h, constraintEqualToConstant: 18.0_f64];
            let _: () = msg_send![c, setActive: true];

            // Text field
            let label: id = msg_send![class!(NSTextField),
                labelWithString: CocoaNSString::alloc(nil).init_str("")];
            let _: () = msg_send![label, setTranslatesAutoresizingMaskIntoConstraints: false];
            let font: id = msg_send![class!(NSFont), systemFontOfSize: 13.0_f64];
            let _: () = msg_send![label, setFont: font];
            let _: () = msg_send![cell, addSubview: label];
            let _: () = msg_send![cell, setTextField: label];

            // Layout: [4px][icon 18px][6px][label][4px]
            let il: id = msg_send![iv, leadingAnchor];
            let cl: id = msg_send![cell, leadingAnchor];
            let c: id = msg_send![il, constraintEqualToAnchor: cl constant: 4.0_f64];
            let _: () = msg_send![c, setActive: true];

            let icy: id = msg_send![iv, centerYAnchor];
            let ccy: id = msg_send![cell, centerYAnchor];
            let c: id = msg_send![icy, constraintEqualToAnchor: ccy];
            let _: () = msg_send![c, setActive: true];

            let ll: id = msg_send![label, leadingAnchor];
            let it: id = msg_send![iv, trailingAnchor];
            let c: id = msg_send![ll, constraintEqualToAnchor: it constant: 6.0_f64];
            let _: () = msg_send![c, setActive: true];

            let lcy: id = msg_send![label, centerYAnchor];
            let c: id = msg_send![lcy, constraintEqualToAnchor: ccy];
            let _: () = msg_send![c, setActive: true];

            let lt: id = msg_send![label, trailingAnchor];
            let ct: id = msg_send![cell, trailingAnchor];
            let c: id = msg_send![lt, constraintEqualToAnchor: ct constant: -4.0_f64];
            let _: () = msg_send![c, setActive: true];
        }

        // Update image
        let iv: id = msg_send![cell, imageView];
        let ns_symbol = CocoaNSString::alloc(nil).init_str(symbol_name);
        let image: id = msg_send![
            class!(NSImage),
            imageWithSystemSymbolName: ns_symbol
            accessibilityDescription: nil
        ];
        if image != nil {
            let _: () = msg_send![iv, setImage: image];
        }

        // Update text
        let tf: id = msg_send![cell, textField];
        let ns_title = CocoaNSString::alloc(nil).init_str(title);
        let _: () = msg_send![tf, setStringValue: ns_title];
        cell
    }
}
