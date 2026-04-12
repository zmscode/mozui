use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::os::raw::c_void;
use std::sync::Once;

/// A menu item in a native menu.
pub enum MenuItem {
    /// A clickable action with title, optional key equivalent, and callback.
    Action {
        title: String,
        key: Option<String>,
        action: Box<dyn Fn() + 'static>,
    },
    /// A submenu containing child items.
    Submenu { title: String, items: Vec<MenuItem> },
    /// A visual separator line.
    Separator,
}

impl MenuItem {
    /// Create a simple action item.
    pub fn action(title: impl Into<String>, action: impl Fn() + 'static) -> Self {
        Self::Action {
            title: title.into(),
            key: None,
            action: Box::new(action),
        }
    }

    /// Create an action item with a keyboard shortcut.
    pub fn action_with_key(
        title: impl Into<String>,
        key: impl Into<String>,
        action: impl Fn() + 'static,
    ) -> Self {
        Self::Action {
            title: title.into(),
            key: Some(key.into()),
            action: Box::new(action),
        }
    }

    /// Create a submenu.
    pub fn submenu(title: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self::Submenu {
            title: title.into(),
            items,
        }
    }

    /// Create a separator.
    pub fn separator() -> Self {
        Self::Separator
    }
}

/// Build an `NSMenu` from a list of `MenuItem`s. Returns the raw `NSMenu` id.
pub fn build_menu(title: &str, items: &[MenuItem]) -> id {
    unsafe {
        let ns_title = CocoaNSString::alloc(nil).init_str(title);
        let menu: id = msg_send![class!(NSMenu), alloc];
        let menu: id = msg_send![menu, initWithTitle: ns_title];

        for item in items {
            let ns_item = build_menu_item(item);
            let _: () = msg_send![menu, addItem: ns_item];
        }

        menu
    }
}

/// Show a context menu at the current mouse location relative to the window's content view.
pub fn show_context_menu(window: &Window, items: &[MenuItem]) {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let menu = build_menu("", items);
        let event: id = msg_send![class!(NSApp), currentEvent];
        let _: () = msg_send![
            class!(NSMenu),
            popUpContextMenu: menu
            withEvent: event
            forView: ns_view
        ];
    }
}

/// Install a menu in the application's main menu bar.
pub fn install_menu_bar_item(title: &str, items: &[MenuItem]) {
    unsafe {
        let main_menu: id = msg_send![class!(NSApp), mainMenu];
        let menu = build_menu(title, items);

        let ns_title = CocoaNSString::alloc(nil).init_str(title);
        let bar_item: id = msg_send![class!(NSMenuItem), alloc];
        let bar_item: id = msg_send![bar_item, initWithTitle: ns_title
            action: nil
            keyEquivalent: CocoaNSString::alloc(nil).init_str("")];
        let _: () = msg_send![bar_item, setSubmenu: menu];
        let _: () = msg_send![main_menu, addItem: bar_item];
    }
}

fn build_menu_item(item: &MenuItem) -> id {
    unsafe {
        match item {
            MenuItem::Separator => msg_send![class!(NSMenuItem), separatorItem],
            MenuItem::Action { title, key, action } => {
                let ns_title = CocoaNSString::alloc(nil).init_str(title);
                let ns_key = CocoaNSString::alloc(nil).init_str(key.as_deref().unwrap_or(""));

                let target = create_action_target(action);
                let ns_item: id = msg_send![class!(NSMenuItem), alloc];
                let ns_item: id = msg_send![ns_item,
                    initWithTitle: ns_title
                    action: sel!(performAction:)
                    keyEquivalent: ns_key];
                let _: () = msg_send![ns_item, setTarget: target];
                ns_item
            }
            MenuItem::Submenu { title, items } => {
                let ns_title = CocoaNSString::alloc(nil).init_str(title);
                let ns_item: id = msg_send![class!(NSMenuItem), alloc];
                let ns_item: id = msg_send![ns_item,
                    initWithTitle: ns_title
                    action: nil
                    keyEquivalent: CocoaNSString::alloc(nil).init_str("")];

                let submenu = build_menu(title, items);
                let _: () = msg_send![ns_item, setSubmenu: submenu];
                ns_item
            }
        }
    }
}

// --- Action target ---

static REGISTER_ACTION_TARGET: Once = Once::new();
static mut ACTION_TARGET_CLASS: *const Class = std::ptr::null();
const CALLBACK_IVAR: &str = "_callback";

fn create_action_target(action: &Box<dyn Fn() + 'static>) -> id {
    unsafe {
        REGISTER_ACTION_TARGET.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiMenuActionTarget", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(CALLBACK_IVAR);

            extern "C" fn perform_action(this: &Object, _sel: Sel, _sender: id) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
                    let callback = &*(ptr as *const Box<dyn Fn()>);
                    callback();
                }
            }

            decl.add_method(
                sel!(performAction:),
                perform_action as extern "C" fn(&Object, Sel, id),
            );

            ACTION_TARGET_CLASS = decl.register();
        });

        let cls = ACTION_TARGET_CLASS;
        let target: id = msg_send![cls, alloc];
        let target: id = msg_send![target, init];

        // Clone the Box and leak it — menu items live for app lifetime
        let action_clone: Box<dyn Fn()> = {
            // We need to store the callback. Since we can't clone a Box<dyn Fn()>,
            // we store a pointer to the original (caller must ensure it outlives the menu).
            let ptr = action as *const Box<dyn Fn()>;
            // Re-box a pointer to the trait object
            std::ptr::read(ptr)
        };
        let callback_box = Box::new(action_clone);
        let callback_ptr = Box::into_raw(callback_box) as *mut c_void;
        (*target).set_ivar(CALLBACK_IVAR, callback_ptr);

        target
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
