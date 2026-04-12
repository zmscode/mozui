use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::os::raw::c_void;
use std::sync::Once;

/// Configuration for a native search field installed in the toolbar.
pub struct SearchFieldConfig {
    /// Placeholder text shown when empty.
    pub placeholder: String,
    /// Callback invoked when the search text changes.
    pub on_change: Option<Box<dyn Fn(String) + 'static>>,
    /// Callback invoked when the user presses Enter.
    pub on_submit: Option<Box<dyn Fn(String) + 'static>>,
}

impl Default for SearchFieldConfig {
    fn default() -> Self {
        Self {
            placeholder: "Search".into(),
            on_change: None,
            on_submit: None,
        }
    }
}

/// Installs an `NSSearchField` as a toolbar item.
///
/// Returns the toolbar item identifier to include in toolbar item lists.
/// The search field uses `NSSearchToolbarItem` (macOS 11+) for proper
/// toolbar integration with expand/collapse behavior.
pub fn install_search_toolbar_item(window: &Window, config: SearchFieldConfig) -> String {
    let ns_view = get_raw_ns_view(window);
    let item_id = "mozui-search";

    unsafe {
        let ns_window: id = msg_send![ns_view, window];
        let toolbar: id = msg_send![ns_window, toolbar];

        if toolbar != nil {
            // Create NSSearchToolbarItem (macOS 11+)
            let ns_id = CocoaNSString::alloc(nil).init_str(item_id);
            let search_item: id = msg_send![class!(NSSearchToolbarItem), alloc];
            let search_item: id = msg_send![search_item, initWithItemIdentifier: ns_id];

            // Get the search field from the toolbar item
            let search_field: id = msg_send![search_item, searchField];
            let ns_placeholder = CocoaNSString::alloc(nil).init_str(&config.placeholder);
            let placeholder_cell: id = msg_send![search_field, cell];
            let _: () = msg_send![placeholder_cell, setPlaceholderString: ns_placeholder];

            // Set up delegate for callbacks
            if config.on_change.is_some() || config.on_submit.is_some() {
                let delegate = create_search_delegate(config.on_change, config.on_submit);
                let _: () = msg_send![search_field, setDelegate: delegate];
            }
        }
    }

    item_id.into()
}

/// Creates a standalone `NSSearchField` and adds it to the window's content view.
pub fn create_search_field(window: &Window, config: SearchFieldConfig) -> id {
    let ns_view = get_raw_ns_view(window);

    unsafe {
        let search_field: id = msg_send![class!(NSSearchField), alloc];
        let search_field: id = msg_send![search_field, init];
        let _: () = msg_send![search_field, setTranslatesAutoresizingMaskIntoConstraints: false];

        let ns_placeholder = CocoaNSString::alloc(nil).init_str(&config.placeholder);
        let cell: id = msg_send![search_field, cell];
        let _: () = msg_send![cell, setPlaceholderString: ns_placeholder];

        if config.on_change.is_some() || config.on_submit.is_some() {
            let delegate = create_search_delegate(config.on_change, config.on_submit);
            let _: () = msg_send![search_field, setDelegate: delegate];
        }

        let parent: id = msg_send![ns_view, superview];
        let _: () = msg_send![parent, addSubview: search_field];

        search_field
    }
}

// --- Search field delegate ---

static REGISTER_SEARCH_DELEGATE: Once = Once::new();
static mut SEARCH_DELEGATE_CLASS: *const Class = std::ptr::null();

const ON_CHANGE_IVAR: &str = "_onChange";
const ON_SUBMIT_IVAR: &str = "_onSubmit";

fn create_search_delegate(
    on_change: Option<Box<dyn Fn(String) + 'static>>,
    on_submit: Option<Box<dyn Fn(String) + 'static>>,
) -> id {
    unsafe {
        REGISTER_SEARCH_DELEGATE.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiSearchDelegate", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(ON_CHANGE_IVAR);
            decl.add_ivar::<*mut c_void>(ON_SUBMIT_IVAR);

            // controlTextDidChange:
            extern "C" fn text_did_change(this: &Object, _sel: Sel, notification: id) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(ON_CHANGE_IVAR);
                    if !ptr.is_null() {
                        let callback = &*(ptr as *const Box<dyn Fn(String)>);
                        let obj: id = msg_send![notification, object];
                        let value: id = msg_send![obj, stringValue];
                        let utf8: *const i8 = msg_send![value, UTF8String];
                        let text = std::ffi::CStr::from_ptr(utf8)
                            .to_str()
                            .unwrap_or("")
                            .to_string();
                        callback(text);
                    }
                }
            }

            // controlTextDidEndEditing: (Enter pressed)
            extern "C" fn text_did_end(this: &Object, _sel: Sel, notification: id) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(ON_SUBMIT_IVAR);
                    if !ptr.is_null() {
                        let callback = &*(ptr as *const Box<dyn Fn(String)>);
                        let obj: id = msg_send![notification, object];
                        let value: id = msg_send![obj, stringValue];
                        let utf8: *const i8 = msg_send![value, UTF8String];
                        let text = std::ffi::CStr::from_ptr(utf8)
                            .to_str()
                            .unwrap_or("")
                            .to_string();
                        callback(text);
                    }
                }
            }

            decl.add_method(
                sel!(controlTextDidChange:),
                text_did_change as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(controlTextDidEndEditing:),
                text_did_end as extern "C" fn(&Object, Sel, id),
            );

            SEARCH_DELEGATE_CLASS = decl.register();
        });

        let cls = SEARCH_DELEGATE_CLASS;
        let delegate: id = msg_send![cls, alloc];
        let delegate: id = msg_send![delegate, init];

        if let Some(cb) = on_change {
            let ptr = Box::into_raw(Box::new(cb)) as *mut c_void;
            (*delegate).set_ivar(ON_CHANGE_IVAR, ptr);
        } else {
            (*delegate).set_ivar(ON_CHANGE_IVAR, std::ptr::null_mut::<c_void>());
        }

        if let Some(cb) = on_submit {
            let ptr = Box::into_raw(Box::new(cb)) as *mut c_void;
            (*delegate).set_ivar(ON_SUBMIT_IVAR, ptr);
        } else {
            (*delegate).set_ivar(ON_SUBMIT_IVAR, std::ptr::null_mut::<c_void>());
        }

        delegate
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
