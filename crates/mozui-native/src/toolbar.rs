use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::declare::ClassDecl;
use objc::runtime::{BOOL, Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::collections::HashMap;
use std::os::raw::c_void;
use std::sync::Once;

/// Built-in toolbar item identifiers provided by AppKit.
pub enum ToolbarItemId {
    /// Toggle sidebar visibility (the standard sidebar button).
    ToggleSidebar,
    /// Tracking separator aligned with the sidebar split view divider.
    SidebarTrackingSeparator,
    /// Flexible space between items.
    FlexibleSpace,
    /// Fixed space between items.
    Space,
    /// Custom item with an SF Symbol icon.
    SymbolButton {
        id: String,
        symbol: String,
        label: String,
        /// If true, marks the item as navigational (left-side, own Liquid Glass pill).
        navigational: bool,
    },
    /// A group of SF Symbol buttons displayed as a segmented control.
    /// Items in a group share a single Liquid Glass pill.
    SymbolGroup {
        id: String,
        /// (symbol_name, label) pairs for each segment.
        items: Vec<(String, String)>,
    },
}

impl ToolbarItemId {
    fn identifier_string(&self) -> &str {
        match self {
            Self::ToggleSidebar => "NSToolbarToggleSidebarItemIdentifier",
            Self::SidebarTrackingSeparator => "NSToolbarSidebarTrackingSeparatorItemIdentifier",
            Self::FlexibleSpace => "NSToolbarFlexibleSpaceItemIdentifier",
            Self::Space => "NSToolbarSpaceItemIdentifier",
            Self::SymbolButton { id, .. } => id.as_str(),
            Self::SymbolGroup { id, .. } => id.as_str(),
        }
    }

    fn to_ns_string(&self) -> id {
        unsafe { CocoaNSString::alloc(nil).init_str(self.identifier_string()) }
    }
}

/// Config for a custom toolbar item (symbol name + label).
struct ToolbarItemConfig {
    symbol: String,
    label: String,
    navigational: bool,
}

/// Config for a toolbar item group (segmented control).
struct ToolbarGroupConfig {
    items: Vec<(String, String)>, // (symbol, label) pairs
}

/// Installs an `NSToolbar` on the window associated with the given mozui `Window`.
///
/// On macOS 26+, the toolbar automatically adopts Liquid Glass appearance.
/// Each `SymbolButton` renders as an individual NSButton, giving it its own
/// Liquid Glass pill rather than merging with adjacent items.
pub fn install_toolbar(window: &Window, items: &[ToolbarItemId]) {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let ns_window: id = msg_send![ns_view, window];

        // Show the window title in the toolbar area
        let _: () = msg_send![ns_window, setTitleVisibility: 0_isize]; // NSWindowTitleVisible

        let toolbar_id = CocoaNSString::alloc(nil).init_str("mozui-main-toolbar");
        let toolbar: id = msg_send![class!(NSToolbar), alloc];
        let toolbar: id = msg_send![toolbar, initWithIdentifier: toolbar_id];

        // Collect item identifiers
        let item_ids: Vec<id> = items.iter().map(|i| i.to_ns_string()).collect();

        // Build config maps for custom items
        let mut configs: HashMap<String, ToolbarItemConfig> = HashMap::new();
        let mut group_configs: HashMap<String, ToolbarGroupConfig> = HashMap::new();
        for item in items {
            match item {
                ToolbarItemId::SymbolButton {
                    id,
                    symbol,
                    label,
                    navigational,
                } => {
                    configs.insert(
                        id.clone(),
                        ToolbarItemConfig {
                            symbol: symbol.clone(),
                            label: label.clone(),
                            navigational: *navigational,
                        },
                    );
                }
                ToolbarItemId::SymbolGroup { id, items: group } => {
                    group_configs.insert(
                        id.clone(),
                        ToolbarGroupConfig {
                            items: group.clone(),
                        },
                    );
                }
                _ => {}
            }
        }

        let delegate = create_toolbar_delegate(&item_ids, configs, group_configs);

        let _: () = msg_send![toolbar, setDelegate: delegate];
        let _: () = msg_send![toolbar, setDisplayMode: 2_isize]; // IconOnly

        let _: () = msg_send![ns_window, setToolbar: toolbar];
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}

// --- Toolbar Delegate using cocoa/objc crate ---

static REGISTER_DELEGATE: Once = Once::new();
static mut DELEGATE_CLASS: *const Class = std::ptr::null();

const ITEM_IDS_IVAR: &str = "_itemIdentifiers";
const ITEM_CONFIGS_IVAR: &str = "_itemConfigs";
const GROUP_CONFIGS_IVAR: &str = "_groupConfigs";

unsafe fn create_toolbar_delegate(
    item_ids: &[id],
    configs: HashMap<String, ToolbarItemConfig>,
    group_configs: HashMap<String, ToolbarGroupConfig>,
) -> id {
    REGISTER_DELEGATE.call_once(|| unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("MozuiToolbarDelegate", superclass).unwrap();

        decl.add_ivar::<*mut c_void>(ITEM_IDS_IVAR);
        decl.add_ivar::<*mut c_void>(ITEM_CONFIGS_IVAR);
        decl.add_ivar::<*mut c_void>(GROUP_CONFIGS_IVAR);

        extern "C" fn default_item_identifiers(_this: &Object, _sel: Sel, _toolbar: id) -> id {
            unsafe {
                let ptr: *mut c_void = *_this.get_ivar(ITEM_IDS_IVAR);
                let ids = &*(ptr as *const Vec<id>);
                let ns_array: id = msg_send![
                    class!(NSArray),
                    arrayWithObjects: ids.as_ptr()
                    count: ids.len()
                ];
                ns_array
            }
        }

        extern "C" fn allowed_item_identifiers(_this: &Object, _sel: Sel, _toolbar: id) -> id {
            default_item_identifiers(_this, _sel, _toolbar)
        }

        extern "C" fn item_for_identifier(
            _this: &Object,
            _sel: Sel,
            _toolbar: id,
            identifier: id,
            _will_insert: BOOL,
        ) -> id {
            unsafe {
                let item: id = msg_send![class!(NSToolbarItem), alloc];
                let item: id = msg_send![item, initWithItemIdentifier: identifier];

                // Check if this is a custom item with config
                let configs_ptr: *mut c_void = *_this.get_ivar(ITEM_CONFIGS_IVAR);
                let configs = &*(configs_ptr as *const HashMap<String, ToolbarItemConfig>);

                // Get the identifier as a Rust string
                let utf8: *const i8 = msg_send![identifier, UTF8String];
                let id_str = std::ffi::CStr::from_ptr(utf8).to_str().unwrap_or("");

                // Check group configs first
                let groups_ptr: *mut c_void = *_this.get_ivar(GROUP_CONFIGS_IVAR);
                let groups = &*(groups_ptr as *const HashMap<String, ToolbarGroupConfig>);

                if let Some(group) = groups.get(id_str) {
                    // Create NSToolbarItemGroup with SF Symbol images
                    let mut images: Vec<id> = Vec::new();
                    let mut labels: Vec<id> = Vec::new();
                    for (symbol, label) in &group.items {
                        let ns_sym = CocoaNSString::alloc(nil).init_str(symbol);
                        let img: id = msg_send![
                            class!(NSImage),
                            imageWithSystemSymbolName: ns_sym
                            accessibilityDescription: nil
                        ];
                        images.push(img);
                        labels.push(CocoaNSString::alloc(nil).init_str(label));
                    }

                    let images_arr: id = msg_send![
                        class!(NSArray),
                        arrayWithObjects: images.as_ptr()
                        count: images.len()
                    ];
                    let labels_arr: id = msg_send![
                        class!(NSArray),
                        arrayWithObjects: labels.as_ptr()
                        count: labels.len()
                    ];

                    // NSToolbarItemGroup with selectAny mode (0 = selectAny)
                    let group_item: id = msg_send![
                        class!(NSToolbarItemGroup),
                        groupWithItemIdentifier: identifier
                        images: images_arr
                        selectionMode: 0_isize
                        labels: labels_arr
                        target: nil
                        action: nil
                    ];
                    return group_item;
                }

                if let Some(config) = configs.get(id_str) {
                    // Create SF Symbol image
                    let ns_symbol = CocoaNSString::alloc(nil).init_str(&config.symbol);
                    let image: id = msg_send![
                        class!(NSImage),
                        imageWithSystemSymbolName: ns_symbol
                        accessibilityDescription: nil
                    ];

                    if image != nil {
                        let _: () = msg_send![item, setImage: image];
                    }

                    // Set label and tooltip
                    let ns_label = CocoaNSString::alloc(nil).init_str(&config.label);
                    let _: () = msg_send![item, setLabel: ns_label];
                    let _: () = msg_send![item, setToolTip: ns_label];

                    // Bordered gives Liquid Glass treatment
                    let _: () = msg_send![item, setBordered: true];

                    // Navigational items get their own glass pill on the left
                    if config.navigational {
                        let _: () = msg_send![item, setNavigational: true];
                    }
                }

                item
            }
        }

        decl.add_method(
            sel!(toolbarDefaultItemIdentifiers:),
            default_item_identifiers as extern "C" fn(&Object, Sel, id) -> id,
        );
        decl.add_method(
            sel!(toolbarAllowedItemIdentifiers:),
            allowed_item_identifiers as extern "C" fn(&Object, Sel, id) -> id,
        );
        decl.add_method(
            sel!(toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:),
            item_for_identifier as extern "C" fn(&Object, Sel, id, id, BOOL) -> id,
        );

        DELEGATE_CLASS = decl.register();
    });

    unsafe {
        let cls = DELEGATE_CLASS;
        let delegate: id = msg_send![cls, alloc];
        let delegate: id = msg_send![delegate, init];

        // Store item IDs — leak since toolbar lives for window lifetime
        let ids_box = Box::new(item_ids.to_vec());
        let ids_ptr = Box::into_raw(ids_box) as *mut c_void;
        (*delegate).set_ivar(ITEM_IDS_IVAR, ids_ptr);

        // Store item configs
        let configs_box = Box::new(configs);
        let configs_ptr = Box::into_raw(configs_box) as *mut c_void;
        (*delegate).set_ivar(ITEM_CONFIGS_IVAR, configs_ptr);

        // Store group configs
        let groups_box = Box::new(group_configs);
        let groups_ptr = Box::into_raw(groups_box) as *mut c_void;
        (*delegate).set_ivar(GROUP_CONFIGS_IVAR, groups_ptr);

        delegate
    }
}
