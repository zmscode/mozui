use super::{MacWindow, ns_string};
use crate::platform::native_window::{
    NativeAnchor, NativeHostedSurfaceTarget, NativeInspectorHost, NativeMenuKind,
    NativePopoverBehavior, NativePopoverEdge, NativePopoverHandle, NativeSheetHandle,
    NativeSidebarHost, NativeToolbarDisplayMode, NativeToolbarGroupItem, NativeToolbarHostView,
    PlatformNativeMenu, PlatformNativeMenuItem, PlatformNativePopover, PlatformNativeSheet,
    PlatformNativeToolbar, PlatformNativeToolbarButton, PlatformNativeToolbarGroup,
    PlatformNativeToolbarItem, PlatformNativeToolbarSearchField, PlatformNativeWindow,
};
use crate::{Bounds, Pixels, PlatformWindow, SharedString};
use cocoa::{
    appkit::{
        NSViewHeightSizable, NSViewWidthSizable, NSVisualEffectBlendingMode,
        NSVisualEffectMaterial, NSVisualEffectState,
    },
    base::{id, nil},
    foundation::{NSPoint, NSRect, NSSize, NSUInteger},
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel, YES},
    sel, sel_impl,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, c_void},
    ptr,
    sync::Once,
};

#[derive(Default)]
struct WindowChromeState {
    toolbar_delegate: Option<id>,
    split_view_controller: Option<id>,
    content_item: Option<id>,
    content_view_controller: Option<id>,
    content_root_view: Option<id>,
    content_background_extended: bool,
    sidebar_item: Option<id>,
    inspector_item: Option<id>,
    sidebar_identifier: Option<String>,
    inspector_identifier: Option<String>,
    sidebar_overlays_content: bool,
    host_views: HashMap<String, *mut c_void>,
    sidebar_header_views: HashMap<String, *mut c_void>,
    sidebar_header_height_constraints: HashMap<String, *mut c_void>,
    popovers: HashMap<usize, PopoverEntry>,
    next_popover_handle: usize,
    sheets: HashMap<usize, SheetEntry>,
    next_sheet_handle: usize,
}

struct PopoverEntry {
    popover: id,
    delegate: id,
    host_identifier: Option<String>,
}

struct SheetEntry {
    sheet_window: id,
    delegate: id,
}

thread_local! {
    static WINDOW_CHROME_STATE: RefCell<HashMap<usize, WindowChromeState>> =
        RefCell::new(HashMap::new());
}

fn window_key(ns_window: id) -> usize {
    ns_window as usize
}

fn nsstring_to_rust(value: id) -> String {
    unsafe {
        if value.is_null() {
            return String::new();
        }
        let utf8: *const i8 = msg_send![value, UTF8String];
        if utf8.is_null() {
            String::new()
        } else {
            CStr::from_ptr(utf8).to_string_lossy().into_owned()
        }
    }
}

fn retain_id(object: id) -> id {
    unsafe {
        if !object.is_null() {
            let _: id = msg_send![object, retain];
        }
        object
    }
}

fn release_id(object: id) {
    unsafe {
        if !object.is_null() {
            let _: () = msg_send![object, release];
        }
    }
}

fn bounds_to_ns_rect(bounds: Bounds<Pixels>, parent_height: f64) -> NSRect {
    let x: f64 = bounds.origin.x.into();
    let y: f64 = bounds.origin.y.into();
    let width: f64 = bounds.size.width.into();
    let height: f64 = bounds.size.height.into();
    NSRect::new(
        NSPoint::new(x, parent_height - y - height),
        NSSize::new(width.max(1.0), height.max(1.0)),
    )
}

fn point_to_ns_point(point: crate::Point<Pixels>, parent_height: f64) -> NSPoint {
    let x: f64 = point.x.into();
    let y: f64 = point.y.into();
    NSPoint::new(x, parent_height - y)
}

fn ns_view_size(view: id) -> NSSize {
    unsafe {
        let bounds: NSRect = msg_send![view, bounds];
        bounds.size
    }
}

fn content_parent_view(raw_view: *mut c_void) -> id {
    unsafe {
        let ns_view = raw_view as id;
        let parent: id = msg_send![ns_view, superview];
        if parent.is_null() { ns_view } else { parent }
    }
}

fn supports_background_extension_view() -> bool {
    Class::get("NSBackgroundExtensionView").is_some()
}

fn with_window_state<R>(ns_window: id, f: impl FnOnce(&mut WindowChromeState) -> R) -> R {
    WINDOW_CHROME_STATE.with(|state| {
        let mut state = state.borrow_mut();
        f(state.entry(window_key(ns_window)).or_default())
    })
}

fn with_window_state_by_key<R>(key: usize, f: impl FnOnce(Option<&WindowChromeState>) -> R) -> R {
    WINDOW_CHROME_STATE.with(|state| {
        let state = state.borrow();
        f(state.get(&key))
    })
}

fn with_window_state_by_key_mut<R>(
    key: usize,
    f: impl FnOnce(Option<&mut WindowChromeState>) -> R,
) -> R {
    WINDOW_CHROME_STATE.with(|state| {
        let mut state = state.borrow_mut();
        f(state.get_mut(&key))
    })
}

fn set_host_view(ns_window: id, identifier: &str, view: id) {
    with_window_state(ns_window, |state| {
        state.host_views.insert(identifier.to_string(), view.cast());
    });
}

fn set_sidebar_header_view(ns_window: id, identifier: &str, view: id, height_constraint: id) {
    with_window_state(ns_window, |state| {
        state
            .sidebar_header_views
            .insert(identifier.to_string(), view.cast());
        state
            .sidebar_header_height_constraints
            .insert(identifier.to_string(), height_constraint.cast());
    });
}

fn remove_host_view(ns_window: id, identifier: &str) {
    with_window_state(ns_window, |state| {
        state.host_views.remove(identifier);
    });
}

fn remove_sidebar_header_view(ns_window: id, identifier: &str) {
    with_window_state(ns_window, |state| {
        state.sidebar_header_views.remove(identifier);
        if let Some(constraint) = state.sidebar_header_height_constraints.remove(identifier) {
            release_id(constraint as id);
        }
    });
}

unsafe fn clear_host_contents(host_view: id) {
    let subviews: id = msg_send![host_view, subviews];
    let count: NSUInteger = msg_send![subviews, count];
    for index in (0..count).rev() {
        let subview: id = msg_send![subviews, objectAtIndex: index];
        let _: () = msg_send![subview, removeFromSuperview];
    }
}

unsafe fn replace_host_contents(host_view: id, content_view: id) {
    unsafe {
        clear_host_contents(host_view);
    }
    if content_view.is_null() {
        return;
    }
    let current_parent: id = msg_send![content_view, superview];
    if !current_parent.is_null() {
        let _: () = msg_send![content_view, removeFromSuperview];
    }
    let bounds: NSRect = msg_send![host_view, bounds];
    let _: () = msg_send![content_view, setFrame: bounds];
    let _: () = msg_send![content_view, setAutoresizingMask: 18usize];
    let _: () = msg_send![host_view, addSubview: content_view];
}

fn display_mode_value(display_mode: NativeToolbarDisplayMode) -> isize {
    match display_mode {
        NativeToolbarDisplayMode::Default => 0,
        NativeToolbarDisplayMode::IconAndLabel => 1,
        NativeToolbarDisplayMode::IconOnly => 2,
        NativeToolbarDisplayMode::LabelOnly => 3,
    }
}

fn popover_behavior_value(behavior: NativePopoverBehavior) -> isize {
    match behavior {
        NativePopoverBehavior::ApplicationDefined => 0,
        NativePopoverBehavior::Transient => 1,
        NativePopoverBehavior::Semitransient => 2,
    }
}

fn popover_edge_value(edge: NativePopoverEdge) -> isize {
    match edge {
        NativePopoverEdge::Left => 0,
        NativePopoverEdge::Top => 1,
        NativePopoverEdge::Right => 2,
        NativePopoverEdge::Bottom => 3,
    }
}

fn toolbar_identifier_string(item: &PlatformNativeToolbarItem) -> SharedString {
    match item {
        PlatformNativeToolbarItem::ToggleSidebar => "NSToolbarToggleSidebarItemIdentifier".into(),
        PlatformNativeToolbarItem::SidebarTrackingSeparator => {
            "NSToolbarSidebarTrackingSeparatorItemIdentifier".into()
        }
        PlatformNativeToolbarItem::FlexibleSpace => "NSToolbarFlexibleSpaceItemIdentifier".into(),
        PlatformNativeToolbarItem::Space => "NSToolbarSpaceItemIdentifier".into(),
        PlatformNativeToolbarItem::Button(button) => button.identifier.clone(),
        PlatformNativeToolbarItem::Group(group) => group.identifier.clone(),
        PlatformNativeToolbarItem::SearchField(search) => search.identifier.clone(),
        PlatformNativeToolbarItem::HostView(host) => host.identifier.clone(),
    }
}

fn is_system_toolbar_identifier(identifier: &str) -> bool {
    matches!(
        identifier,
        "NSToolbarToggleSidebarItemIdentifier"
            | "NSToolbarSidebarTrackingSeparatorItemIdentifier"
            | "NSToolbarFlexibleSpaceItemIdentifier"
            | "NSToolbarSpaceItemIdentifier"
    )
}

struct ToolbarDelegateState {
    window_key: usize,
    item_identifiers: Vec<String>,
    buttons: HashMap<String, PlatformNativeToolbarButton>,
    groups: HashMap<String, PlatformNativeToolbarGroup>,
    searches: HashMap<String, PlatformNativeToolbarSearchField>,
    hosts: HashMap<String, NativeToolbarHostView>,
}

impl ToolbarDelegateState {
    fn new(window_key: usize, toolbar: PlatformNativeToolbar) -> Self {
        let mut item_identifiers = Vec::new();
        let mut buttons = HashMap::new();
        let mut groups = HashMap::new();
        let mut searches = HashMap::new();
        let mut hosts = HashMap::new();

        for item in toolbar.items {
            let identifier = toolbar_identifier_string(&item).to_string();
            item_identifiers.push(identifier.clone());
            match item {
                PlatformNativeToolbarItem::Button(button) => {
                    buttons.insert(identifier, button);
                }
                PlatformNativeToolbarItem::Group(group) => {
                    groups.insert(identifier, group);
                }
                PlatformNativeToolbarItem::SearchField(search) => {
                    searches.insert(identifier, search);
                }
                PlatformNativeToolbarItem::HostView(host) => {
                    hosts.insert(identifier, host);
                }
                _ => {}
            }
        }

        Self {
            window_key,
            item_identifiers,
            buttons,
            groups,
            searches,
            hosts,
        }
    }
}

static REGISTER_TOOLBAR_DELEGATE: Once = Once::new();
static mut TOOLBAR_DELEGATE_CLASS: *const Class = ptr::null();
const TOOLBAR_STATE_IVAR: &str = "_toolbarState";

fn toolbar_delegate_class() -> *const Class {
    unsafe {
        REGISTER_TOOLBAR_DELEGATE.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiNativeWindowToolbarDelegate", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(TOOLBAR_STATE_IVAR);

            extern "C" fn dealloc(this: &Object, _: Sel) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(TOOLBAR_STATE_IVAR);
                    if !ptr.is_null() {
                        drop(Box::from_raw(ptr as *mut ToolbarDelegateState));
                    }
                    let _: () = msg_send![super(this, class!(NSObject)), dealloc];
                }
            }

            extern "C" fn default_item_identifiers(this: &Object, _: Sel, _toolbar: id) -> id {
                unsafe {
                    let state =
                        &*((*this.get_ivar::<*mut c_void>(TOOLBAR_STATE_IVAR))
                            as *const ToolbarDelegateState);
                    let objects: Vec<id> = state
                        .item_identifiers
                        .iter()
                        .map(|identifier| ns_string(identifier))
                        .collect();
                    msg_send![class!(NSArray), arrayWithObjects: objects.as_ptr() count: objects.len()]
                }
            }

            extern "C" fn allowed_item_identifiers(this: &Object, sel: Sel, toolbar: id) -> id {
                default_item_identifiers(this, sel, toolbar)
            }

            extern "C" fn button_action(this: &Object, _: Sel, sender: id) {
                unsafe {
                    let state =
                        &mut *((*this.get_ivar::<*mut c_void>(TOOLBAR_STATE_IVAR))
                            as *mut ToolbarDelegateState);
                    let represented: id = msg_send![sender, representedObject];
                    let identifier = nsstring_to_rust(represented);
                    if let Some(button) = state.buttons.get_mut(&identifier)
                        && let Some(callback) = button.on_activate.as_ref()
                    {
                        callback();
                    }
                }
            }

            extern "C" fn group_action(this: &Object, _: Sel, sender: id) {
                unsafe {
                    let state =
                        &mut *((*this.get_ivar::<*mut c_void>(TOOLBAR_STATE_IVAR))
                            as *mut ToolbarDelegateState);
                    let represented: id = msg_send![sender, representedObject];
                    let identifier = nsstring_to_rust(represented);
                    if let Some(group) = state.groups.get_mut(&identifier)
                        && let Some(callback) = group.on_activate.as_ref()
                    {
                        let selected_segment: isize = msg_send![sender, selectedSegment];
                        callback(selected_segment.max(0) as usize);
                    }
                }
            }

            extern "C" fn text_did_change(this: &Object, _: Sel, notification: id) {
                unsafe {
                    let state =
                        &mut *((*this.get_ivar::<*mut c_void>(TOOLBAR_STATE_IVAR))
                            as *mut ToolbarDelegateState);
                    let field: id = msg_send![notification, object];
                    let represented: id = msg_send![field, representedObject];
                    let identifier = nsstring_to_rust(represented);
                    if let Some(search) = state.searches.get_mut(&identifier)
                        && let Some(callback) = search.on_change.as_ref()
                    {
                        let value: id = msg_send![field, stringValue];
                        callback(nsstring_to_rust(value));
                    }
                }
            }

            extern "C" fn text_did_end(this: &Object, _: Sel, notification: id) {
                unsafe {
                    let state =
                        &mut *((*this.get_ivar::<*mut c_void>(TOOLBAR_STATE_IVAR))
                            as *mut ToolbarDelegateState);
                    let field: id = msg_send![notification, object];
                    let represented: id = msg_send![field, representedObject];
                    let identifier = nsstring_to_rust(represented);
                    if let Some(search) = state.searches.get_mut(&identifier)
                        && let Some(callback) = search.on_submit.as_ref()
                    {
                        let value: id = msg_send![field, stringValue];
                        callback(nsstring_to_rust(value));
                    }
                }
            }

            extern "C" fn item_for_identifier(
                this: &Object,
                _: Sel,
                _toolbar: id,
                identifier: id,
                _will_insert: bool,
            ) -> id {
                unsafe {
                    let state =
                        &mut *((*this.get_ivar::<*mut c_void>(TOOLBAR_STATE_IVAR))
                            as *mut ToolbarDelegateState);
                    let identifier_string = nsstring_to_rust(identifier);

                    if is_system_toolbar_identifier(&identifier_string) {
                        return nil;
                    }

                    if let Some(button) = state.buttons.get(&identifier_string) {
                        return create_toolbar_button_item(this, button);
                    }
                    if let Some(group) = state.groups.get(&identifier_string) {
                        return create_toolbar_group_item(this, group);
                    }
                    if let Some(search) = state.searches.get(&identifier_string) {
                        return create_toolbar_search_item(this, search);
                    }
                    if let Some(host) = state.hosts.get(&identifier_string) {
                        return create_toolbar_host_item(state.window_key, host);
                    }

                    let item: id = msg_send![class!(NSToolbarItem), alloc];
                    msg_send![item, initWithItemIdentifier: identifier]
                }
            }

            decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
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
                item_for_identifier as extern "C" fn(&Object, Sel, id, id, bool) -> id,
            );
            decl.add_method(
                sel!(toolbarButtonAction:),
                button_action as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(toolbarGroupAction:),
                group_action as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(controlTextDidChange:),
                text_did_change as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(controlTextDidEndEditing:),
                text_did_end as extern "C" fn(&Object, Sel, id),
            );

            TOOLBAR_DELEGATE_CLASS = decl.register();
        });
        TOOLBAR_DELEGATE_CLASS
    }
}

unsafe fn create_toolbar_delegate(window_key: usize, toolbar: PlatformNativeToolbar) -> id {
    let cls = toolbar_delegate_class();
    let delegate: id = unsafe { msg_send![cls, alloc] };
    let delegate: id = unsafe { msg_send![delegate, init] };
    let state = Box::new(ToolbarDelegateState::new(window_key, toolbar));
    unsafe {
        (*delegate).set_ivar(TOOLBAR_STATE_IVAR, Box::into_raw(state) as *mut c_void);
    }
    delegate
}

unsafe fn create_toolbar_button_item(
    delegate: &Object,
    button: &PlatformNativeToolbarButton,
) -> id {
    let item_identifier = unsafe { ns_string(button.identifier.as_ref()) };
    let item: id = unsafe { msg_send![class!(NSToolbarItem), alloc] };
    let item: id = unsafe { msg_send![item, initWithItemIdentifier: item_identifier] };

    let button_view: id = unsafe { msg_send![class!(NSButton), alloc] };
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(32.0, 28.0));
    let button_view: id = unsafe { msg_send![button_view, initWithFrame: frame] };
    let title = unsafe { ns_string(button.label.as_ref()) };
    let symbol_name = unsafe { ns_string(button.symbol.as_ref()) };
    let image: id = unsafe {
        msg_send![
            class!(NSImage),
            imageWithSystemSymbolName: symbol_name
            accessibilityDescription: title
        ]
    };
    if !image.is_null() {
        let _: () = unsafe { msg_send![button_view, setImage: image] };
    }
    let _: () = unsafe { msg_send![button_view, setBordered: true] };
    if button.navigational {
        let _: () = unsafe { msg_send![button_view, setBezelStyle: 1isize] };
    }
    let represented = unsafe { ns_string(button.identifier.as_ref()) };
    let _: () = unsafe { msg_send![button_view, setRepresentedObject: represented] };
    let _: () = unsafe { msg_send![button_view, setTarget: delegate] };
    let _: () = unsafe { msg_send![button_view, setAction: sel!(toolbarButtonAction:)] };
    let _: () = unsafe { msg_send![item, setLabel: title] };
    let _: () = unsafe { msg_send![item, setToolTip: title] };
    let _: () = unsafe { msg_send![item, setView: button_view] };
    item
}

unsafe fn create_toolbar_group_item(delegate: &Object, group: &PlatformNativeToolbarGroup) -> id {
    let item_identifier = unsafe { ns_string(group.identifier.as_ref()) };
    let item: id = unsafe { msg_send![class!(NSToolbarItem), alloc] };
    let item: id = unsafe { msg_send![item, initWithItemIdentifier: item_identifier] };

    let width = (group.items.len() as f64 * 32.0).max(40.0);
    let control: id = unsafe { msg_send![class!(NSSegmentedControl), alloc] };
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, 28.0));
    let control: id = unsafe { msg_send![control, initWithFrame: frame] };
    let _: () = unsafe { msg_send![control, setSegmentCount: group.items.len() as isize] };
    let _: () = unsafe { msg_send![control, setTrackingMode: 1isize] };
    for (index, item) in group.items.iter().enumerate() {
        unsafe {
            set_segment_content(control, index, item);
        }
    }
    let represented = unsafe { ns_string(group.identifier.as_ref()) };
    let _: () = unsafe { msg_send![control, setRepresentedObject: represented] };
    let _: () = unsafe { msg_send![control, setTarget: delegate] };
    let _: () = unsafe { msg_send![control, setAction: sel!(toolbarGroupAction:)] };
    let _: () = unsafe { msg_send![item, setView: control] };
    item
}

unsafe fn set_segment_content(control: id, index: usize, item: &NativeToolbarGroupItem) {
    let label = unsafe { ns_string(item.label.as_ref()) };
    let symbol_name = unsafe { ns_string(item.symbol.as_ref()) };
    let image: id = unsafe {
        msg_send![
            class!(NSImage),
            imageWithSystemSymbolName: symbol_name
            accessibilityDescription: label
        ]
    };
    if !image.is_null() {
        let _: () = unsafe { msg_send![control, setImage: image forSegment: index as isize] };
    }
    let _: () = unsafe { msg_send![control, setLabel: label forSegment: index as isize] };
}

unsafe fn create_toolbar_search_item(
    delegate: &Object,
    search: &PlatformNativeToolbarSearchField,
) -> id {
    let item_identifier = unsafe { ns_string(search.identifier.as_ref()) };
    let search_item: id = unsafe { msg_send![class!(NSSearchToolbarItem), alloc] };
    let search_item: id =
        unsafe { msg_send![search_item, initWithItemIdentifier: item_identifier] };
    let search_field: id = unsafe { msg_send![search_item, searchField] };
    let represented = unsafe { ns_string(search.identifier.as_ref()) };
    let _: () = unsafe { msg_send![search_field, setRepresentedObject: represented] };
    let placeholder: id = unsafe { ns_string(search.placeholder.as_ref()) };
    let cell: id = unsafe { msg_send![search_field, cell] };
    let _: () = unsafe { msg_send![cell, setPlaceholderString: placeholder] };
    let _: () = unsafe { msg_send![search_field, setDelegate: delegate] };
    search_item
}

unsafe fn create_toolbar_host_item(window_key: usize, host: &NativeToolbarHostView) -> id {
    let item_identifier = unsafe { ns_string(host.identifier.as_ref()) };
    let item: id = unsafe { msg_send![class!(NSToolbarItem), alloc] };
    let item: id = unsafe { msg_send![item, initWithItemIdentifier: item_identifier] };
    let label = unsafe { ns_string(host.label.as_ref()) };
    let _: () = unsafe { msg_send![item, setLabel: label] };
    let _: () = unsafe { msg_send![item, setToolTip: label] };

    let view: id = unsafe { msg_send![class!(NSView), alloc] };
    let frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(host.width.into(), host.height.into()),
    );
    let view: id = unsafe { msg_send![view, initWithFrame: frame] };
    set_host_view(window_key as id, host.identifier.as_ref(), view);
    let _: () = unsafe { msg_send![item, setView: view] };
    item
}

struct MenuTargetState {
    callbacks: HashMap<String, Box<dyn Fn()>>,
}

static REGISTER_MENU_TARGET: Once = Once::new();
static mut MENU_TARGET_CLASS: *const Class = ptr::null();
const MENU_STATE_IVAR: &str = "_menuState";

fn menu_target_class() -> *const Class {
    unsafe {
        REGISTER_MENU_TARGET.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiNativeWindowMenuTarget", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(MENU_STATE_IVAR);

            extern "C" fn dealloc(this: &Object, _: Sel) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(MENU_STATE_IVAR);
                    if !ptr.is_null() {
                        drop(Box::from_raw(ptr as *mut MenuTargetState));
                    }
                    let _: () = msg_send![super(this, class!(NSObject)), dealloc];
                }
            }

            extern "C" fn menu_action(this: &Object, _: Sel, sender: id) {
                unsafe {
                    let state = &mut *((*this.get_ivar::<*mut c_void>(MENU_STATE_IVAR))
                        as *mut MenuTargetState);
                    let represented: id = msg_send![sender, representedObject];
                    let identifier = nsstring_to_rust(represented);
                    if let Some(callback) = state.callbacks.get_mut(&identifier) {
                        callback();
                    }
                }
            }

            decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
            decl.add_method(
                sel!(menuAction:),
                menu_action as extern "C" fn(&Object, Sel, id),
            );
            MENU_TARGET_CLASS = decl.register();
        });
        MENU_TARGET_CLASS
    }
}

unsafe fn create_menu_target(items: &mut [PlatformNativeMenuItem]) -> id {
    let cls = menu_target_class();
    let target: id = unsafe { msg_send![cls, alloc] };
    let target: id = unsafe { msg_send![target, init] };
    let mut callbacks = HashMap::new();
    for item in items.iter_mut() {
        if let Some(callback) = item.on_activate.take() {
            callbacks.insert(item.identifier.to_string(), callback);
        }
    }
    unsafe {
        (*target).set_ivar(
            MENU_STATE_IVAR,
            Box::into_raw(Box::new(MenuTargetState { callbacks })) as *mut c_void,
        );
    }
    target
}

struct PopoverDelegateState {
    window_key: usize,
    handle: usize,
    host_identifier: Option<String>,
    on_close: Option<Box<dyn Fn()>>,
}

struct SheetDelegateState {
    window_key: usize,
    handle: usize,
    host_identifier: Option<String>,
    on_close: Option<Box<dyn Fn()>>,
}

static REGISTER_POPOVER_DELEGATE: Once = Once::new();
static mut POPOVER_DELEGATE_CLASS: *const Class = ptr::null();
const POPOVER_STATE_IVAR: &str = "_popoverState";

static REGISTER_SHEET_DELEGATE: Once = Once::new();
static mut SHEET_DELEGATE_CLASS: *const Class = ptr::null();
const SHEET_STATE_IVAR: &str = "_sheetState";

fn popover_delegate_class() -> *const Class {
    unsafe {
        REGISTER_POPOVER_DELEGATE.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiNativeWindowPopoverDelegate", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(POPOVER_STATE_IVAR);

            extern "C" fn dealloc(this: &Object, _: Sel) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(POPOVER_STATE_IVAR);
                    if !ptr.is_null() {
                        drop(Box::from_raw(ptr as *mut PopoverDelegateState));
                    }
                    let _: () = msg_send![super(this, class!(NSObject)), dealloc];
                }
            }

            extern "C" fn popover_did_close(this: &Object, _: Sel, _notification: id) {
                unsafe {
                    let state = &mut *((*this.get_ivar::<*mut c_void>(POPOVER_STATE_IVAR))
                        as *mut PopoverDelegateState);
                    if let Some(callback) = state.on_close.take() {
                        callback();
                    }
                    with_window_state_by_key_mut(state.window_key, |window_state| {
                        let Some(window_state) = window_state else {
                            return;
                        };
                        if let Some(identifier) = state.host_identifier.as_deref() {
                            window_state.host_views.remove(identifier);
                        }
                        if let Some(entry) = window_state.popovers.remove(&state.handle) {
                            release_id(entry.popover);
                            release_id(entry.delegate);
                        }
                    });
                }
            }

            decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
            decl.add_method(
                sel!(popoverDidClose:),
                popover_did_close as extern "C" fn(&Object, Sel, id),
            );
            POPOVER_DELEGATE_CLASS = decl.register();
        });
        POPOVER_DELEGATE_CLASS
    }
}

unsafe fn create_popover_delegate(
    window_key: usize,
    handle: usize,
    host_identifier: Option<String>,
    on_close: Option<Box<dyn Fn()>>,
) -> id {
    let cls = popover_delegate_class();
    let delegate: id = unsafe { msg_send![cls, alloc] };
    let delegate: id = unsafe { msg_send![delegate, init] };
    let state = Box::new(PopoverDelegateState {
        window_key,
        handle,
        host_identifier,
        on_close,
    });
    unsafe {
        (*delegate).set_ivar(POPOVER_STATE_IVAR, Box::into_raw(state) as *mut c_void);
    }
    delegate
}

fn sheet_delegate_class() -> *const Class {
    unsafe {
        REGISTER_SHEET_DELEGATE.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiNativeWindowSheetDelegate", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(SHEET_STATE_IVAR);

            extern "C" fn dealloc(this: &Object, _: Sel) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(SHEET_STATE_IVAR);
                    if !ptr.is_null() {
                        drop(Box::from_raw(ptr as *mut SheetDelegateState));
                    }
                    let _: () = msg_send![super(this, class!(NSObject)), dealloc];
                }
            }

            extern "C" fn window_will_close(this: &Object, _: Sel, _notification: id) {
                unsafe {
                    let state = &mut *((*this.get_ivar::<*mut c_void>(SHEET_STATE_IVAR))
                        as *mut SheetDelegateState);
                    if let Some(callback) = state.on_close.take() {
                        callback();
                    }
                    with_window_state_by_key_mut(state.window_key, |window_state| {
                        let Some(window_state) = window_state else {
                            return;
                        };
                        if let Some(identifier) = state.host_identifier.as_deref() {
                            window_state.host_views.remove(identifier);
                        }
                        if let Some(entry) = window_state.sheets.remove(&state.handle) {
                            release_id(entry.sheet_window);
                            release_id(entry.delegate);
                        }
                    });
                }
            }

            decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
            decl.add_method(
                sel!(windowWillClose:),
                window_will_close as extern "C" fn(&Object, Sel, id),
            );
            SHEET_DELEGATE_CLASS = decl.register();
        });
        SHEET_DELEGATE_CLASS
    }
}

unsafe fn create_sheet_delegate(
    window_key: usize,
    handle: usize,
    host_identifier: Option<String>,
    on_close: Option<Box<dyn Fn()>>,
) -> id {
    let cls = sheet_delegate_class();
    let delegate: id = msg_send![cls, alloc];
    let delegate: id = msg_send![delegate, init];
    let state = Box::new(SheetDelegateState {
        window_key,
        handle,
        host_identifier,
        on_close,
    });
    unsafe {
        (*delegate).set_ivar(SHEET_STATE_IVAR, Box::into_raw(state) as *mut c_void);
    }
    delegate
}

fn resolve_toolbar_anchor_view(ns_window: id, identifier: &str) -> Option<id> {
    unsafe {
        let toolbar: id = msg_send![ns_window, toolbar];
        if toolbar.is_null() {
            return None;
        }
        let items: id = msg_send![toolbar, items];
        let count: NSUInteger = msg_send![items, count];
        for index in 0..count {
            let item: id = msg_send![items, objectAtIndex: index];
            let item_identifier: id = msg_send![item, itemIdentifier];
            if nsstring_to_rust(item_identifier) != identifier {
                continue;
            }

            let responds: bool = msg_send![item, respondsToSelector: sel!(searchField)];
            if responds {
                let search_field: id = msg_send![item, searchField];
                if !search_field.is_null() {
                    return Some(search_field);
                }
            }

            let view: id = msg_send![item, view];
            if !view.is_null() {
                return Some(view);
            }
        }
        None
    }
}

fn resolve_anchor(
    ns_window: id,
    raw_view: *mut c_void,
    anchor: &NativeAnchor,
) -> Option<(id, NSRect)> {
    unsafe {
        match anchor {
            NativeAnchor::ContentBounds(bounds) => {
                let view = raw_view as id;
                let parent_height = ns_view_size(view).height;
                Some((view, bounds_to_ns_rect(*bounds, parent_height)))
            }
            NativeAnchor::ContentPoint(point) => {
                let view = raw_view as id;
                let parent_height = ns_view_size(view).height;
                let ns_point = point_to_ns_point(*point, parent_height);
                Some((view, NSRect::new(ns_point, NSSize::new(1.0, 1.0))))
            }
            NativeAnchor::ToolbarItem(identifier) => {
                resolve_toolbar_anchor_view(ns_window, identifier.as_ref()).map(|view| {
                    let bounds: NSRect = msg_send![view, bounds];
                    (view, bounds)
                })
            }
        }
    }
}

const SIDEBAR_HEADER_HEIGHT: f64 = 38.0;

unsafe fn create_sidebar_host_views(width: f64) -> (id, id, id, id) {
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width.max(1.0), 420.0));

    let root: id = unsafe { msg_send![class!(NSView), alloc] };
    let root: id = unsafe { msg_send![root, initWithFrame: frame] };
    let _: () =
        unsafe { msg_send![root, setAutoresizingMask: NSViewWidthSizable | NSViewHeightSizable] };

    if !supports_background_extension_view() {
        let effect: id = unsafe { msg_send![class!(NSVisualEffectView), alloc] };
        let effect: id = unsafe { msg_send![effect, initWithFrame: frame] };
        let _: () = unsafe { msg_send![effect, setMaterial: NSVisualEffectMaterial::Sidebar] };
        let _: () =
            unsafe { msg_send![effect, setBlendingMode: NSVisualEffectBlendingMode::BehindWindow] };
        let _: () =
            unsafe { msg_send![effect, setState: NSVisualEffectState::FollowsWindowActiveState] };
        let _: () = unsafe {
            msg_send![effect, setAutoresizingMask: NSViewWidthSizable | NSViewHeightSizable]
        };
        let _: () = unsafe { msg_send![root, addSubview: effect positioned: -1i64 relativeTo: nil] };
        let _: () = unsafe { msg_send![effect, release] };
    }

    let header: id = unsafe { msg_send![class!(NSView), alloc] };
    let header: id = unsafe { msg_send![header, initWithFrame: frame] };
    let body: id = unsafe { msg_send![class!(NSView), alloc] };
    let body: id = unsafe { msg_send![body, initWithFrame: frame] };

    let _: () = unsafe { msg_send![header, setTranslatesAutoresizingMaskIntoConstraints: 0i8] };
    let _: () = unsafe { msg_send![body, setTranslatesAutoresizingMaskIntoConstraints: 0i8] };
    let _: () = unsafe { msg_send![header, setHidden: 1i8] };

    let _: () = unsafe { msg_send![root, addSubview: header] };
    let _: () = unsafe { msg_send![root, addSubview: body] };

    let has_safe_area: bool =
        unsafe { msg_send![root, respondsToSelector: sel!(safeAreaLayoutGuide)] };
    let guide_top: id = if has_safe_area {
        let guide: id = unsafe { msg_send![root, safeAreaLayoutGuide] };
        unsafe { msg_send![guide, topAnchor] }
    } else {
        unsafe { msg_send![root, topAnchor] }
    };

    let root_leading: id = unsafe { msg_send![root, leadingAnchor] };
    let root_trailing: id = unsafe { msg_send![root, trailingAnchor] };
    let root_bottom: id = unsafe { msg_send![root, bottomAnchor] };

    let header_top: id = unsafe { msg_send![header, topAnchor] };
    let header_leading: id = unsafe { msg_send![header, leadingAnchor] };
    let header_trailing: id = unsafe { msg_send![header, trailingAnchor] };
    let header_bottom: id = unsafe { msg_send![header, bottomAnchor] };
    let header_height: id = unsafe { msg_send![header, heightAnchor] };

    let body_top: id = unsafe { msg_send![body, topAnchor] };
    let body_leading: id = unsafe { msg_send![body, leadingAnchor] };
    let body_trailing: id = unsafe { msg_send![body, trailingAnchor] };
    let body_bottom: id = unsafe { msg_send![body, bottomAnchor] };

    let c1: id = unsafe { msg_send![header_top, constraintEqualToAnchor: guide_top] };
    let c2: id = unsafe { msg_send![header_leading, constraintEqualToAnchor: root_leading] };
    let c3: id = unsafe { msg_send![header_trailing, constraintEqualToAnchor: root_trailing] };
    let c4: id = unsafe { msg_send![header_height, constraintEqualToConstant: 0.0_f64] };
    let c5: id = unsafe { msg_send![body_top, constraintEqualToAnchor: header_bottom] };
    let c6: id = unsafe { msg_send![body_leading, constraintEqualToAnchor: root_leading] };
    let c7: id = unsafe { msg_send![body_trailing, constraintEqualToAnchor: root_trailing] };
    let c8: id = unsafe { msg_send![body_bottom, constraintEqualToAnchor: root_bottom] };

    for constraint in [c1, c2, c3, c4, c5, c6, c7, c8] {
        let _: () = unsafe { msg_send![constraint, setActive: 1i8] };
    }
    let _: () = unsafe { msg_send![c4, retain] };

    let _: () = unsafe { msg_send![header, release] };
    let _: () = unsafe { msg_send![body, release] };

    (root, header, body, c4)
}

unsafe fn create_background_extended_content_view(content_view: id, sidebar_on_trailing: bool) -> id {
    if content_view.is_null() || !supports_background_extension_view() {
        return content_view;
    }

    let Some(background_extension_class) = Class::get("NSBackgroundExtensionView") else {
        return content_view;
    };

    let frame: NSRect = unsafe { msg_send![content_view, frame] };
    let bg_ext: id = unsafe { msg_send![background_extension_class, alloc] };
    let bg_ext: id = unsafe { msg_send![bg_ext, initWithFrame: frame] };
    let _: () = unsafe {
        msg_send![bg_ext, setAutoresizingMask: NSViewWidthSizable | NSViewHeightSizable]
    };

    let supports_auto_place: bool =
        unsafe { msg_send![bg_ext, respondsToSelector: sel!(setAutomaticallyPlacesContentView:)] };
    if supports_auto_place {
        let _: () = unsafe { msg_send![bg_ext, setAutomaticallyPlacesContentView: 0i8] };
    }

    let supports_content_view: bool =
        unsafe { msg_send![bg_ext, respondsToSelector: sel!(setContentView:)] };
    if supports_content_view {
        let _: () = unsafe { msg_send![bg_ext, setContentView: content_view] };
    } else {
        let current_parent: id = unsafe { msg_send![content_view, superview] };
        if !current_parent.is_null() {
            let _: () = unsafe { msg_send![content_view, removeFromSuperview] };
        }
        let _: () = unsafe { msg_send![bg_ext, addSubview: content_view] };
    }

    let _: () = unsafe { msg_send![content_view, setTranslatesAutoresizingMaskIntoConstraints: 0i8] };

    let content_top: id = unsafe { msg_send![content_view, topAnchor] };
    let content_bottom: id = unsafe { msg_send![content_view, bottomAnchor] };
    let content_leading: id = unsafe { msg_send![content_view, leadingAnchor] };
    let content_trailing: id = unsafe { msg_send![content_view, trailingAnchor] };

    let ext_top: id = unsafe { msg_send![bg_ext, topAnchor] };
    let ext_bottom: id = unsafe { msg_send![bg_ext, bottomAnchor] };
    let ext_leading: id = unsafe { msg_send![bg_ext, leadingAnchor] };
    let ext_trailing: id = unsafe { msg_send![bg_ext, trailingAnchor] };

    let has_safe_area: bool =
        unsafe { msg_send![bg_ext, respondsToSelector: sel!(safeAreaLayoutGuide)] };
    let safe_overlap_anchor: id = if has_safe_area {
        let guide: id = unsafe { msg_send![bg_ext, safeAreaLayoutGuide] };
        if sidebar_on_trailing {
            unsafe { msg_send![guide, trailingAnchor] }
        } else {
            unsafe { msg_send![guide, leadingAnchor] }
        }
    } else if sidebar_on_trailing {
        ext_trailing
    } else {
        ext_leading
    };

    let c1: id = unsafe { msg_send![content_top, constraintEqualToAnchor: ext_top] };
    let c2: id = unsafe { msg_send![content_bottom, constraintEqualToAnchor: ext_bottom] };
    let (c3, c4): (id, id) = if sidebar_on_trailing {
        (
            unsafe { msg_send![content_leading, constraintEqualToAnchor: ext_leading] },
            unsafe { msg_send![content_trailing, constraintEqualToAnchor: safe_overlap_anchor] },
        )
    } else {
        (
            unsafe { msg_send![content_trailing, constraintEqualToAnchor: ext_trailing] },
            unsafe { msg_send![content_leading, constraintEqualToAnchor: safe_overlap_anchor] },
        )
    };

    for constraint in [c1, c2, c3, c4] {
        let _: () = unsafe { msg_send![constraint, setActive: 1i8] };
    }

    bg_ext
}

unsafe fn ensure_content_background_extension(ns_window: id, sidebar_on_trailing: bool) {
    if !supports_background_extension_view() {
        return;
    }

    let state = with_window_state_by_key(window_key(ns_window), |state| {
        state.and_then(|state| {
            if state.content_background_extended {
                None
            } else {
                Some((
                    state.content_view_controller,
                    state.content_item,
                    state.content_root_view,
                ))
            }
        })
    });

    let Some((Some(content_vc), content_item, Some(content_root_view))) = state else {
        return;
    };

    let background_extended_view =
        unsafe { create_background_extended_content_view(content_root_view, sidebar_on_trailing) };
    if background_extended_view == content_root_view {
        return;
    }

    let _: () = unsafe { msg_send![content_vc, setView: background_extended_view] };
    if let Some(content_item) = content_item {
        let responds: bool = unsafe {
            msg_send![
                content_item,
                respondsToSelector: sel!(setAutomaticallyAdjustsSafeAreaInsets:)
            ]
        };
        if responds {
            let _: () = unsafe {
                msg_send![content_item, setAutomaticallyAdjustsSafeAreaInsets: 1i8]
            };
        }
    }

    with_window_state(ns_window, |state| {
        state.content_background_extended = true;
    });
}

unsafe fn ensure_split_view_controller(
    ns_window: id,
    raw_view: *mut c_void,
    extend_background_for_sidebar: bool,
) -> id {
    if let Some(existing) = with_window_state(ns_window, |state| state.split_view_controller) {
        if extend_background_for_sidebar {
            unsafe { ensure_content_background_extension(ns_window, false) };
        }
        return existing;
    }

    let mozui_view = raw_view as id;
    let content_view = content_parent_view(raw_view);
    let content_vc_view = if extend_background_for_sidebar {
        unsafe { create_background_extended_content_view(content_view, false) }
    } else {
        content_view
    };

    let content_vc: id = unsafe { msg_send![class!(NSViewController), alloc] };
    let content_vc: id = unsafe { msg_send![content_vc, init] };
    let _: () = unsafe { msg_send![content_vc, setView: content_vc_view] };

    let content_item: id =
        unsafe { msg_send![class!(NSSplitViewItem), splitViewItemWithViewController: content_vc] };
    let split_vc: id = unsafe { msg_send![class!(NSSplitViewController), alloc] };
    let split_vc: id = unsafe { msg_send![split_vc, init] };
    let _: () = unsafe { msg_send![split_vc, addSplitViewItem: content_item] };
    let _: () = unsafe { msg_send![ns_window, setContentViewController: split_vc] };
    let _: () = unsafe { msg_send![content_view, addSubview: mozui_view] };

    if extend_background_for_sidebar {
        let responds: bool = unsafe {
            msg_send![
                content_item,
                respondsToSelector: sel!(setAutomaticallyAdjustsSafeAreaInsets:)
            ]
        };
        if responds {
            let _: () =
                unsafe { msg_send![content_item, setAutomaticallyAdjustsSafeAreaInsets: 1i8] };
        }
    }

    with_window_state(ns_window, |state| {
        state.split_view_controller = Some(split_vc);
        state.content_item = Some(content_item);
        state.content_view_controller = Some(content_vc);
        state.content_root_view = Some(content_view);
        state.content_background_extended = extend_background_for_sidebar;
    });

    split_vc
}

unsafe fn install_or_update_host_item(
    ns_window: id,
    raw_view: *mut c_void,
    identifier: &str,
    width: Pixels,
    min_width: Pixels,
    max_width: Pixels,
    visible: bool,
    is_sidebar: bool,
    overlays_content: bool,
    overlay_inset: Pixels,
    overlay_corner_radius: Pixels,
) -> bool {
    if is_sidebar && overlays_content {
        unsafe {
            remove_split_sidebar_item(ns_window);
            return install_or_update_overlay_sidebar_host(
                ns_window,
                raw_view,
                identifier,
                width,
                visible,
                overlay_inset,
                overlay_corner_radius,
            );
        }
    }

    if is_sidebar {
        unsafe {
            remove_overlay_sidebar_host(ns_window);
        }
    }

    let split_vc = unsafe { ensure_split_view_controller(ns_window, raw_view, is_sidebar) };

    if is_sidebar {
        let sidebar_identifier = identifier.to_string();
        let needs_sidebar_rebuild = with_window_state(ns_window, |state| {
            state.sidebar_item.is_some()
                && (!state.host_views.contains_key(&sidebar_identifier)
                    || !state.sidebar_header_views.contains_key(&sidebar_identifier))
        });
        if needs_sidebar_rebuild {
            unsafe {
                remove_split_sidebar_item(ns_window);
            }
        }
    }

    let existing_item = with_window_state(ns_window, |state| {
        if is_sidebar {
            state.sidebar_item
        } else {
            state.inspector_item
        }
    });

    let host_view = if let Some(existing_item) = existing_item {
        unsafe {
            let vc: id = msg_send![existing_item, viewController];
            let view: id = msg_send![vc, view];
            let _: () = msg_send![existing_item, setMinimumThickness: Into::<f64>::into(min_width)];
            let _: () = msg_send![existing_item, setMaximumThickness: Into::<f64>::into(max_width)];
            let _: () = msg_send![existing_item, setCollapsed: !visible];
            view
        }
    } else {
        unsafe {
            let vc: id = msg_send![class!(NSViewController), alloc];
            let vc: id = msg_send![vc, init];
            let frame = NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(Into::<f64>::into(width), 400.0),
            );
            let view: id = if is_sidebar {
                let (root, header, body, header_height_constraint) =
                    create_sidebar_host_views(Into::<f64>::into(width));
                set_sidebar_header_view(ns_window, identifier, header, header_height_constraint);
                set_host_view(ns_window, identifier, body);
                root
            } else {
                let view: id = msg_send![class!(NSView), alloc];
                let view: id = msg_send![view, initWithFrame: frame];
                set_host_view(ns_window, identifier, view);
                view
            };
            let _: () = msg_send![vc, setView: view];
            let item: id = if is_sidebar {
                msg_send![class!(NSSplitViewItem), sidebarWithViewController: vc]
            } else {
                msg_send![class!(NSSplitViewItem), inspectorWithViewController: vc]
            };
            let _: () = msg_send![item, setMinimumThickness: Into::<f64>::into(min_width)];
            let _: () = msg_send![item, setMaximumThickness: Into::<f64>::into(max_width)];
            let _: () = msg_send![item, setCollapsed: !visible];
            if is_sidebar {
                let _: () = msg_send![split_vc, insertSplitViewItem: item atIndex: 0isize];
            } else {
                let _: () = msg_send![split_vc, addSplitViewItem: item];
            }

            with_window_state(ns_window, |state| {
                if is_sidebar {
                    state.sidebar_item = Some(item);
                    state.sidebar_identifier = Some(identifier.to_string());
                    state.sidebar_overlays_content = false;
                } else {
                    state.inspector_item = Some(item);
                    state.inspector_identifier = Some(identifier.to_string());
                }
            });

            view
        }
    };

    if !is_sidebar {
        set_host_view(ns_window, identifier, host_view);
    }
    if is_sidebar {
        let split_view: id = unsafe { msg_send![split_vc, splitView] };
        let sidebar_width = Into::<f64>::into(width)
            .max(Into::<f64>::into(min_width))
            .min(Into::<f64>::into(max_width));
        let _: () =
            unsafe { msg_send![split_view, setPosition: sidebar_width ofDividerAtIndex: 0isize] };
        let _: () = unsafe { msg_send![split_view, adjustSubviews] };
    }
    true
}

unsafe fn remove_split_sidebar_item(ns_window: id) {
    let (split_vc, sidebar_item, sidebar_identifier) = with_window_state(ns_window, |state| {
        let split_vc = state.split_view_controller;
        let sidebar_item = state.sidebar_item.take();
        let sidebar_identifier = state.sidebar_identifier.clone();
        state.sidebar_overlays_content = false;
        (split_vc, sidebar_item, sidebar_identifier)
    });

    if let (Some(split_vc), Some(sidebar_item)) = (split_vc, sidebar_item) {
        let _: () = unsafe { msg_send![split_vc, removeSplitViewItem: sidebar_item] };
    }
    if let Some(identifier) = sidebar_identifier.as_deref() {
        remove_host_view(ns_window, identifier);
        remove_sidebar_header_view(ns_window, identifier);
    }
}

unsafe fn remove_overlay_sidebar_host(ns_window: id) {
    let (overlay_view, sidebar_identifier) = with_window_state(ns_window, |state| {
        if !state.sidebar_overlays_content {
            return (None, state.sidebar_identifier.clone());
        }

        state.sidebar_overlays_content = false;
        (
            state
                .sidebar_identifier
                .as_ref()
                .and_then(|identifier| state.host_views.remove(identifier)),
            state.sidebar_identifier.clone(),
        )
    });

    if let Some(overlay_view) = overlay_view {
        let overlay_view = overlay_view as id;
        let _: () = unsafe { msg_send![overlay_view, removeFromSuperview] };
    }
    if let Some(identifier) = sidebar_identifier.as_deref() {
        remove_sidebar_header_view(ns_window, identifier);
    }
}

unsafe fn install_or_update_overlay_sidebar_host(
    ns_window: id,
    raw_view: *mut c_void,
    identifier: &str,
    width: Pixels,
    visible: bool,
    overlay_inset: Pixels,
    overlay_corner_radius: Pixels,
) -> bool {
    let parent_view = content_parent_view(raw_view);
    if parent_view.is_null() {
        return false;
    }

    let existing_view = with_window_state(ns_window, |state| {
        state.sidebar_identifier = Some(identifier.to_string());
        state.sidebar_overlays_content = true;
        state.host_views.get(identifier).copied()
    });

    let width: f64 = width.into();
    let inset: f64 = overlay_inset.into();
    let corner_radius: f64 = overlay_corner_radius.into();
    let host_view = if let Some(existing_view) = existing_view {
        let host_view = existing_view as id;
        let parent_bounds: NSRect = unsafe { msg_send![parent_view, bounds] };
        let frame = NSRect::new(
            NSPoint::new(inset, inset),
            NSSize::new(
                width.max(1.0),
                (parent_bounds.size.height - inset * 2.0).max(1.0),
            ),
        );
        let _: () = unsafe { msg_send![host_view, setFrame: frame] };
        unsafe { style_overlay_sidebar_host(host_view, corner_radius) };
        host_view
    } else {
        let parent_bounds: NSRect = unsafe { msg_send![parent_view, bounds] };
        let frame = NSRect::new(
            NSPoint::new(inset, inset),
            NSSize::new(
                width.max(1.0),
                (parent_bounds.size.height - inset * 2.0).max(1.0),
            ),
        );
        let host_view: id = unsafe { msg_send![class!(NSVisualEffectView), alloc] };
        let host_view: id = unsafe { msg_send![host_view, initWithFrame: frame] };
        let _: () = unsafe { msg_send![host_view, setAutoresizingMask: NSViewHeightSizable] };
        unsafe { style_overlay_sidebar_host(host_view, corner_radius) };
        let _: () = unsafe { msg_send![parent_view, addSubview: host_view] };
        host_view
    };

    let _: () = unsafe { msg_send![host_view, setHidden: !visible] };
    set_host_view(ns_window, identifier, host_view);
    true
}

unsafe fn style_overlay_sidebar_host(host_view: id, corner_radius: f64) {
    let _: () = unsafe { msg_send![host_view, setMaterial: NSVisualEffectMaterial::Sidebar] };
    let _: () =
        unsafe { msg_send![host_view, setBlendingMode: NSVisualEffectBlendingMode::WithinWindow] };
    let _: () = unsafe { msg_send![host_view, setState: NSVisualEffectState::Active] };
    let _: () = unsafe { msg_send![host_view, setWantsLayer: YES] };

    let layer: id = unsafe { msg_send![host_view, layer] };
    if layer.is_null() {
        return;
    }

    // Keep the sidebar as a shaped native surface while letting only softened
    // color bleed from the content beneath it.
    let fill_color: id = unsafe {
        msg_send![class!(NSColor),
            colorWithSRGBRed: 0.10_f64
            green: 0.09_f64
            blue: 0.14_f64
            alpha: 0.42_f64
        ]
    };
    let border_color: id = unsafe {
        msg_send![class!(NSColor),
            colorWithSRGBRed: 1.0_f64
            green: 1.0_f64
            blue: 1.0_f64
            alpha: 0.06_f64
        ]
    };
    let fill_cg: id = unsafe { msg_send![fill_color, CGColor] };
    let border_cg: id = unsafe { msg_send![border_color, CGColor] };

    let _: () = unsafe { msg_send![layer, setCornerRadius: corner_radius] };
    let _: () = unsafe { msg_send![layer, setMasksToBounds: YES] };
    let _: () = unsafe { msg_send![layer, setBackgroundColor: fill_cg] };
    let _: () = unsafe { msg_send![layer, setBorderColor: border_cg] };
    let _: () = unsafe { msg_send![layer, setBorderWidth: 1.0_f64] };
}

impl PlatformNativeWindow for MacWindow {
    fn set_native_toolbar(&self, toolbar: PlatformNativeToolbar) {
        unsafe {
            let ns_window = self.raw_native_window_ptr() as id;
            if ns_window.is_null() {
                return;
            }
            let shows_title = toolbar.shows_title;
            let display_mode = toolbar.display_mode;
            let delegate = create_toolbar_delegate(window_key(ns_window), toolbar);
            let delegate = retain_id(delegate);

            let toolbar_id = ns_string("mozui-native-toolbar");
            let toolbar_obj: id = msg_send![class!(NSToolbar), alloc];
            let toolbar_obj: id = msg_send![toolbar_obj, initWithIdentifier: toolbar_id];

            let _: () = msg_send![toolbar_obj, setDelegate: delegate];
            let _: () = msg_send![toolbar_obj, setDisplayMode: display_mode_value(display_mode)];
            let _: () = msg_send![ns_window, setToolbar: toolbar_obj];
            let _: () = msg_send![
                ns_window,
                setTitleVisibility: if shows_title { 0isize } else { 1isize }
            ];

            with_window_state(ns_window, |state| {
                if let Some(previous) = state.toolbar_delegate.replace(delegate) {
                    release_id(previous);
                }
            });
        }
    }

    fn focus_native_search_item(&self, identifier: &str) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return false;
        }
        if let Some(view) = resolve_toolbar_anchor_view(ns_window, identifier) {
            unsafe {
                let _: bool = msg_send![ns_window, makeFirstResponder: view];
            }
            true
        } else {
            false
        }
    }

    fn show_native_menu(&self, mut menu: PlatformNativeMenu) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        let raw_view = self.raw_native_view_ptr();
        if ns_window.is_null() || raw_view.is_null() {
            return false;
        }

        unsafe {
            let Some((anchor_view, anchor_rect)) =
                resolve_anchor(ns_window, raw_view, &menu.anchor)
            else {
                return false;
            };
            let target = create_menu_target(&mut menu.items);
            let menu_obj: id = msg_send![class!(NSMenu), alloc];
            let menu_obj: id = msg_send![menu_obj, initWithTitle: ns_string(match menu.kind {
                NativeMenuKind::Popup => "Popup",
                NativeMenuKind::Suggestions => "Suggestions",
            })];
            for item in menu.items {
                let menu_item: id = if item.separator {
                    msg_send![class!(NSMenuItem), separatorItem]
                } else {
                    let title = ns_string(item.title.as_ref());
                    let menu_item: id = msg_send![class!(NSMenuItem), alloc];
                    let menu_item: id = msg_send![menu_item, initWithTitle: title action: sel!(menuAction:) keyEquivalent: ns_string("")];
                    let _: () = msg_send![menu_item, setTarget: target];
                    let represented = ns_string(item.identifier.as_ref());
                    let _: () = msg_send![menu_item, setRepresentedObject: represented];
                    let _: () = msg_send![menu_item, setEnabled: item.enabled];
                    if let Some(symbol) = item.symbol {
                        let image: id = msg_send![
                            class!(NSImage),
                            imageWithSystemSymbolName: ns_string(symbol.as_ref())
                            accessibilityDescription: title
                        ];
                        if !image.is_null() {
                            let _: () = msg_send![menu_item, setImage: image];
                        }
                    }
                    menu_item
                };
                let _: () = msg_send![menu_obj, addItem: menu_item];
            }
            let location = anchor_rect.origin;
            let _: bool = msg_send![
                menu_obj,
                popUpMenuPositioningItem: nil
                atLocation: location
                inView: anchor_view
            ];
            release_id(target);
            true
        }
    }

    fn show_native_popover(&self, popover: PlatformNativePopover) -> Option<NativePopoverHandle> {
        let ns_window = self.raw_native_window_ptr() as id;
        let raw_view = self.raw_native_view_ptr();
        if ns_window.is_null() || raw_view.is_null() {
            return None;
        }

        unsafe {
            let Some((anchor_view, anchor_rect)) =
                resolve_anchor(ns_window, raw_view, &popover.anchor)
            else {
                return None;
            };

            let handle = with_window_state(ns_window, |state| {
                state.next_popover_handle += 1;
                state.next_popover_handle
            });

            let host_identifier = popover.host_identifier.as_ref().map(ToString::to_string);
            let delegate = create_popover_delegate(
                window_key(ns_window),
                handle,
                host_identifier.clone(),
                popover.on_close,
            );
            let delegate = retain_id(delegate);

            let popover_obj: id = msg_send![class!(NSPopover), alloc];
            let popover_obj: id = msg_send![popover_obj, init];
            let _: () =
                msg_send![popover_obj, setBehavior: popover_behavior_value(popover.behavior)];
            let _: () = msg_send![
                popover_obj,
                setContentSize: NSSize::new(
                    Into::<f64>::into(popover.size.width),
                    Into::<f64>::into(popover.size.height)
                )
            ];

            let vc: id = msg_send![class!(NSViewController), alloc];
            let vc: id = msg_send![vc, init];
            let view: id = msg_send![class!(NSView), alloc];
            let view: id = msg_send![
                view,
                initWithFrame: NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(
                        Into::<f64>::into(popover.size.width),
                        Into::<f64>::into(popover.size.height)
                    )
                )
            ];
            let _: () = msg_send![vc, setView: view];
            let _: () = msg_send![popover_obj, setContentViewController: vc];
            let _: () = msg_send![popover_obj, setDelegate: delegate];

            if let Some(identifier) = host_identifier.as_deref() {
                set_host_view(ns_window, identifier, view);
            }

            let _: () = msg_send![
                popover_obj,
                showRelativeToRect: anchor_rect
                ofView: anchor_view
                preferredEdge: popover_edge_value(popover.edge)
            ];

            with_window_state(ns_window, |state| {
                state.popovers.insert(
                    handle,
                    PopoverEntry {
                        popover: retain_id(popover_obj),
                        delegate,
                        host_identifier,
                    },
                );
            });

            Some(NativePopoverHandle(handle))
        }
    }

    fn close_native_popover(&self, handle: NativePopoverHandle) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return false;
        }

        let entry = with_window_state(ns_window, |state| state.popovers.remove(&handle.0));
        if let Some(entry) = entry {
            unsafe {
                let _: () = msg_send![entry.popover, close];
            }
            if let Some(identifier) = entry.host_identifier.as_deref() {
                remove_host_view(ns_window, identifier);
            }
            release_id(entry.popover);
            release_id(entry.delegate);
            true
        } else {
            false
        }
    }

    fn raw_native_popover_ptr(&self, handle: NativePopoverHandle) -> Option<*mut c_void> {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return None;
        }
        with_window_state_by_key(window_key(ns_window), |state| {
            state
                .and_then(|state| state.popovers.get(&handle.0))
                .map(|entry| entry.popover.cast())
        })
    }

    fn show_native_sheet(&self, sheet: PlatformNativeSheet) -> Option<NativeSheetHandle> {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return None;
        }

        unsafe {
            let handle = with_window_state(ns_window, |state| {
                state.next_sheet_handle += 1;
                state.next_sheet_handle
            });

            let host_identifier = sheet.host_identifier.as_ref().map(ToString::to_string);
            let delegate = create_sheet_delegate(
                window_key(ns_window),
                handle,
                host_identifier.clone(),
                sheet.on_close,
            );
            let delegate = retain_id(delegate);

            let style_mask: usize = if sheet.resizable {
                (1 << 0) | (1 << 1) | (1 << 3)
            } else {
                (1 << 0) | (1 << 1)
            };
            let content_rect = NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(
                    Into::<f64>::into(sheet.size.width),
                    Into::<f64>::into(sheet.size.height),
                ),
            );

            let sheet_window: id = msg_send![class!(NSWindow), alloc];
            let sheet_window: id = msg_send![
                sheet_window,
                initWithContentRect: content_rect
                styleMask: style_mask
                backing: 2isize
                defer: false
            ];
            let _: () = msg_send![sheet_window, setDelegate: delegate];

            if let Some(identifier) = host_identifier.as_deref() {
                let content_view: id = msg_send![sheet_window, contentView];
                if !content_view.is_null() {
                    set_host_view(ns_window, identifier, content_view);
                }
            }

            let _: () = msg_send![ns_window, beginSheet: sheet_window completionHandler: nil];

            with_window_state(ns_window, |state| {
                state.sheets.insert(
                    handle,
                    SheetEntry {
                        sheet_window: retain_id(sheet_window),
                        delegate,
                    },
                );
            });

            Some(NativeSheetHandle(handle))
        }
    }

    fn close_native_sheet(&self, handle: NativeSheetHandle) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return false;
        }

        let sheet_window = with_window_state_by_key(window_key(ns_window), |state| {
            state
                .and_then(|state| state.sheets.get(&handle.0))
                .map(|entry| entry.sheet_window)
        });
        let Some(sheet_window) = sheet_window else {
            return false;
        };

        unsafe {
            let _: () = msg_send![ns_window, endSheet: sheet_window];
            let _: () = msg_send![sheet_window, close];
        }
        true
    }

    fn install_native_sidebar_host(&self, host: NativeSidebarHost) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        let raw_view = self.raw_native_view_ptr();
        if ns_window.is_null() || raw_view.is_null() {
            return false;
        }

        unsafe {
            install_or_update_host_item(
                ns_window,
                raw_view,
                host.identifier.as_ref(),
                host.width,
                host.min_width,
                host.max_width,
                host.visible,
                true,
                host.overlays_content,
                host.overlay_inset,
                host.overlay_corner_radius,
            )
        }
    }

    fn install_native_inspector_host(&self, host: NativeInspectorHost) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        let raw_view = self.raw_native_view_ptr();
        if ns_window.is_null() || raw_view.is_null() {
            return false;
        }

        unsafe {
            install_or_update_host_item(
                ns_window,
                raw_view,
                host.identifier.as_ref(),
                host.width,
                host.min_width,
                host.max_width,
                host.visible,
                false,
                false,
                Pixels::ZERO,
                Pixels::ZERO,
            )
        }
    }

    fn set_native_host_visibility(&self, identifier: &str, visible: bool) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return false;
        }
        with_window_state_by_key_mut(window_key(ns_window), |state| {
            let Some(state) = state else {
                return false;
            };
            if state.sidebar_identifier.as_deref() == Some(identifier) {
                if state.sidebar_overlays_content {
                    if let Some(view) = state.host_views.get(identifier).copied() {
                        let view = view as id;
                        unsafe {
                            let _: () = msg_send![view, setHidden: !visible];
                        }
                        return true;
                    }
                } else if let Some(item) = state.sidebar_item {
                    unsafe {
                        let _: () = msg_send![item, setCollapsed: !visible];
                    }
                    return true;
                }
            }
            if state.inspector_identifier.as_deref() == Some(identifier)
                && let Some(item) = state.inspector_item
            {
                unsafe {
                    let _: () = msg_send![item, setCollapsed: !visible];
                }
                return true;
            }
            false
        })
    }

    fn raw_native_host_view_ptr(&self, identifier: &str) -> Option<*mut c_void> {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return None;
        }
        with_window_state_by_key(window_key(ns_window), |state| {
            state.and_then(|state| state.host_views.get(identifier).copied())
        })
    }

    fn set_native_host_content(&self, identifier: &str, content_view: *mut c_void) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return false;
        }
        let Some(host_view) = self
            .raw_native_host_view_ptr(identifier)
            .map(|view| view as id)
        else {
            return false;
        };
        unsafe {
            replace_host_contents(host_view, content_view as id);
        }
        true
    }

    fn attach_native_hosted_surface(
        &self,
        identifier: &str,
        surface_view: *mut c_void,
        target: NativeHostedSurfaceTarget,
    ) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return false;
        }

        match target {
            NativeHostedSurfaceTarget::Sidebar | NativeHostedSurfaceTarget::Inspector => {
                let Some(host_view) = self
                    .raw_native_host_view_ptr(identifier)
                    .map(|view| view as id)
                else {
                    return false;
                };
                unsafe {
                    replace_host_contents(host_view, surface_view as id);
                }
                true
            }
            NativeHostedSurfaceTarget::SidebarHeader => {
                let header_view = with_window_state_by_key(window_key(ns_window), |state| {
                    state.and_then(|state| state.sidebar_header_views.get(identifier).copied())
                });
                let height_constraint = with_window_state_by_key(window_key(ns_window), |state| {
                    state.and_then(|state| {
                        state
                            .sidebar_header_height_constraints
                            .get(identifier)
                            .copied()
                    })
                });
                let Some(header_view) = header_view.map(|view| view as id) else {
                    return false;
                };
                unsafe {
                    replace_host_contents(header_view, surface_view as id);
                    let _: () = msg_send![header_view, setHidden: 0i8];
                    if let Some(height_constraint) = height_constraint {
                        let _: () = msg_send![
                            height_constraint as id,
                            setConstant: SIDEBAR_HEADER_HEIGHT
                        ];
                    }
                }
                true
            }
        }
    }

    fn clear_native_host_content(&self, identifier: &str) -> bool {
        let ns_window = self.raw_native_window_ptr() as id;
        if ns_window.is_null() {
            return false;
        }
        let Some(host_view) = self
            .raw_native_host_view_ptr(identifier)
            .map(|view| view as id)
        else {
            return false;
        };
        unsafe {
            clear_host_contents(host_view);
        }
        true
    }
}
