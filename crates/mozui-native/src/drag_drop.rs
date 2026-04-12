use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::declare::ClassDecl;
use objc::runtime::{BOOL, Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::os::raw::c_void;
use std::sync::Once;

/// Types of data that can be dragged/dropped.
pub enum DragType {
    /// File paths (UTI: public.file-url).
    Files,
    /// Plain text (UTI: public.utf8-plain-text).
    Text,
    /// URLs (UTI: public.url).
    Urls,
}

impl DragType {
    fn uti_string(&self) -> &str {
        match self {
            Self::Files => "public.file-url",
            Self::Text => "public.utf8-plain-text",
            Self::Urls => "public.url",
        }
    }
}

/// Callback for drop events. Receives a list of dropped strings (paths, text, or URLs).
pub type DropHandler = Box<dyn Fn(Vec<String>) + 'static>;

/// Register the window's content view as a drag destination.
///
/// When items of the specified types are dropped, the handler is called
/// with the dropped data.
pub fn register_drop_target(window: &Window, types: &[DragType], handler: DropHandler) {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        // Register for drag types
        let uti_strings: Vec<id> = types
            .iter()
            .map(|t| CocoaNSString::alloc(nil).init_str(t.uti_string()))
            .collect();
        let types_arr: id = msg_send![
            class!(NSArray),
            arrayWithObjects: uti_strings.as_ptr()
            count: uti_strings.len()
        ];
        let _: () = msg_send![ns_view, registerForDraggedTypes: types_arr];

        // Create the drop delegate and leak it — it lives for the window's lifetime,
        // same pattern as the toolbar delegate in toolbar.rs.
        let _delegate = create_drop_delegate(handler);
    }
}

// --- Drop delegate ---

static REGISTER_DROP_DELEGATE: Once = Once::new();
static mut DROP_DELEGATE_CLASS: *const Class = std::ptr::null();

const DROP_HANDLER_IVAR: &str = "_dropHandler";

fn create_drop_delegate(handler: DropHandler) -> id {
    unsafe {
        REGISTER_DROP_DELEGATE.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiDropDelegate", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(DROP_HANDLER_IVAR);

            // draggingEntered:
            extern "C" fn dragging_entered(_this: &Object, _sel: Sel, _sender: id) -> usize {
                // NSDragOperationCopy = 1
                1
            }

            // performDragOperation:
            extern "C" fn perform_drag(this: &Object, _sel: Sel, sender: id) -> BOOL {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(DROP_HANDLER_IVAR);
                    if ptr.is_null() {
                        return false;
                    }
                    let handler = &*(ptr as *const DropHandler);

                    let pasteboard: id = msg_send![sender, draggingPasteboard];
                    let items: id = msg_send![pasteboard, pasteboardItems];
                    let count: usize = msg_send![items, count];

                    let mut dropped: Vec<String> = Vec::new();

                    for i in 0..count {
                        let item: id = msg_send![items, objectAtIndex: i];

                        // Try file URL first
                        let file_type = CocoaNSString::alloc(nil).init_str("public.file-url");
                        let value: id = msg_send![item, stringForType: file_type];
                        if value != nil {
                            let utf8: *const i8 = msg_send![value, UTF8String];
                            let s = std::ffi::CStr::from_ptr(utf8)
                                .to_str()
                                .unwrap_or("")
                                .to_string();
                            dropped.push(s);
                            continue;
                        }

                        // Try plain text
                        let text_type =
                            CocoaNSString::alloc(nil).init_str("public.utf8-plain-text");
                        let value: id = msg_send![item, stringForType: text_type];
                        if value != nil {
                            let utf8: *const i8 = msg_send![value, UTF8String];
                            let s = std::ffi::CStr::from_ptr(utf8)
                                .to_str()
                                .unwrap_or("")
                                .to_string();
                            dropped.push(s);
                        }
                    }

                    handler(dropped);
                    true
                }
            }

            decl.add_method(
                sel!(draggingEntered:),
                dragging_entered as extern "C" fn(&Object, Sel, id) -> usize,
            );
            decl.add_method(
                sel!(performDragOperation:),
                perform_drag as extern "C" fn(&Object, Sel, id) -> BOOL,
            );

            DROP_DELEGATE_CLASS = decl.register();
        });

        let cls = DROP_DELEGATE_CLASS;
        let delegate: id = msg_send![cls, alloc];
        let delegate: id = msg_send![delegate, init];

        let handler_box = Box::new(handler);
        let handler_ptr = Box::into_raw(handler_box) as *mut c_void;
        (*delegate).set_ivar(DROP_HANDLER_IVAR, handler_ptr);

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
