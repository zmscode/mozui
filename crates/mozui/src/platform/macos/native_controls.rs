use super::{BoolExt, ns_string};
use crate::platform::native_controls::{
    ButtonConfig, ButtonStyle, NativeControlState, PlatformNativeControls, ProgressConfig,
    ProgressStyle, SliderConfig, SwitchConfig, TextFieldConfig, TextFieldStyle,
};
use crate::{Bounds, Pixels};
use cocoa::{
    base::{id, nil},
    foundation::{NSInteger, NSPoint, NSRect, NSSize},
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use std::ffi::c_void;
use std::ptr;
use std::sync::Once;

pub struct MacNativeControls;

pub static MAC_NATIVE_CONTROLS: MacNativeControls = MacNativeControls;

const CALLBACK_IVAR: &str = "_callback";

static REGISTER_VOID_TARGET: Once = Once::new();
static mut VOID_TARGET_CLASS: *const Class = ptr::null();

static REGISTER_BOOL_TARGET: Once = Once::new();
static mut BOOL_TARGET_CLASS: *const Class = ptr::null();

static REGISTER_F64_TARGET: Once = Once::new();
static mut F64_TARGET_CLASS: *const Class = ptr::null();

static REGISTER_TEXT_TARGET: Once = Once::new();
static mut TEXT_TARGET_CLASS: *const Class = ptr::null();

const CHANGE_CALLBACK_IVAR: &str = "_changeCallback";
const SUBMIT_CALLBACK_IVAR: &str = "_submitCallback";

fn bool_target_class() -> *const Class {
    unsafe {
        REGISTER_BOOL_TARGET.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiNativeBoolTarget", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(CALLBACK_IVAR);

            extern "C" fn perform(this: &Object, _: Sel, sender: id) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
                    if !ptr.is_null() {
                        let callback = &*(ptr as *const Box<dyn Fn(bool)>);
                        let state: NSInteger = msg_send![sender, state];
                        callback(state != 0);
                    }
                }
            }

            extern "C" fn dealloc(this: &Object, _: Sel) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
                    if !ptr.is_null() {
                        drop(Box::from_raw(ptr as *mut Box<dyn Fn(bool)>));
                    }
                    let _: () = msg_send![super(this, class!(NSObject)), dealloc];
                }
            }

            decl.add_method(
                sel!(performAction:),
                perform as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
            BOOL_TARGET_CLASS = decl.register();
        });

        BOOL_TARGET_CLASS
    }
}

fn f64_target_class() -> *const Class {
    unsafe {
        REGISTER_F64_TARGET.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiNativeF64Target", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(CALLBACK_IVAR);

            extern "C" fn perform(this: &Object, _: Sel, sender: id) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
                    if !ptr.is_null() {
                        let callback = &*(ptr as *const Box<dyn Fn(f64)>);
                        let value: f64 = msg_send![sender, doubleValue];
                        callback(value);
                    }
                }
            }

            extern "C" fn dealloc(this: &Object, _: Sel) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
                    if !ptr.is_null() {
                        drop(Box::from_raw(ptr as *mut Box<dyn Fn(f64)>));
                    }
                    let _: () = msg_send![super(this, class!(NSObject)), dealloc];
                }
            }

            decl.add_method(
                sel!(performAction:),
                perform as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
            F64_TARGET_CLASS = decl.register();
        });

        F64_TARGET_CLASS
    }
}

fn void_target_class() -> *const Class {
    unsafe {
        REGISTER_VOID_TARGET.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiNativeVoidTarget", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(CALLBACK_IVAR);

            extern "C" fn perform(this: &Object, _: Sel, _sender: id) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
                    if !ptr.is_null() {
                        let callback = &*(ptr as *const Box<dyn Fn()>);
                        callback();
                    }
                }
            }

            extern "C" fn dealloc(this: &Object, _: Sel) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(CALLBACK_IVAR);
                    if !ptr.is_null() {
                        drop(Box::from_raw(ptr as *mut Box<dyn Fn()>));
                    }
                    let _: () = msg_send![super(this, class!(NSObject)), dealloc];
                }
            }

            decl.add_method(
                sel!(performAction:),
                perform as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
            VOID_TARGET_CLASS = decl.register();
        });

        VOID_TARGET_CLASS
    }
}

fn text_target_class() -> *const Class {
    unsafe {
        REGISTER_TEXT_TARGET.call_once(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("MozuiNativeTextTarget", superclass).unwrap();
            decl.add_ivar::<*mut c_void>(CHANGE_CALLBACK_IVAR);
            decl.add_ivar::<*mut c_void>(SUBMIT_CALLBACK_IVAR);

            extern "C" fn control_text_did_change(this: &Object, _: Sel, notification: id) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(CHANGE_CALLBACK_IVAR);
                    if ptr.is_null() {
                        return;
                    }
                    let callback = &*(ptr as *const Box<dyn Fn(String)>);
                    let object: id = msg_send![notification, object];
                    let value = current_string_value(object);
                    callback(value);
                }
            }

            extern "C" fn perform(this: &Object, _: Sel, sender: id) {
                unsafe {
                    let ptr: *mut c_void = *this.get_ivar(SUBMIT_CALLBACK_IVAR);
                    if ptr.is_null() {
                        return;
                    }
                    let callback = &*(ptr as *const Box<dyn Fn(String)>);
                    let value = current_string_value(sender);
                    callback(value);
                }
            }

            extern "C" fn dealloc(this: &Object, _: Sel) {
                unsafe {
                    let change_ptr: *mut c_void = *this.get_ivar(CHANGE_CALLBACK_IVAR);
                    if !change_ptr.is_null() {
                        drop(Box::from_raw(change_ptr as *mut Box<dyn Fn(String)>));
                    }
                    let submit_ptr: *mut c_void = *this.get_ivar(SUBMIT_CALLBACK_IVAR);
                    if !submit_ptr.is_null() {
                        drop(Box::from_raw(submit_ptr as *mut Box<dyn Fn(String)>));
                    }
                    let _: () = msg_send![super(this, class!(NSObject)), dealloc];
                }
            }

            decl.add_method(
                sel!(controlTextDidChange:),
                control_text_did_change as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(performAction:),
                perform as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
            TEXT_TARGET_CLASS = decl.register();
        });

        TEXT_TARGET_CLASS
    }
}

unsafe fn create_void_target(callback: Box<dyn Fn()>) -> id {
    let cls = void_target_class();
    let target: id = msg_send![cls, alloc];
    let target: id = msg_send![target, init];
    let callback_ptr = Box::into_raw(Box::new(callback)) as *mut c_void;
    unsafe {
        (*target).set_ivar(CALLBACK_IVAR, callback_ptr);
    }
    target
}

unsafe fn create_bool_target(callback: Box<dyn Fn(bool)>) -> id {
    let cls = bool_target_class();
    let target: id = msg_send![cls, alloc];
    let target: id = msg_send![target, init];
    let callback_ptr = Box::into_raw(Box::new(callback)) as *mut c_void;
    unsafe {
        (*target).set_ivar(CALLBACK_IVAR, callback_ptr);
    }
    target
}

unsafe fn create_f64_target(callback: Box<dyn Fn(f64)>) -> id {
    let cls = f64_target_class();
    let target: id = msg_send![cls, alloc];
    let target: id = msg_send![target, init];
    let callback_ptr = Box::into_raw(Box::new(callback)) as *mut c_void;
    unsafe {
        (*target).set_ivar(CALLBACK_IVAR, callback_ptr);
    }
    target
}

unsafe fn create_text_target(
    on_change: Option<Box<dyn Fn(String)>>,
    on_submit: Option<Box<dyn Fn(String)>>,
) -> id {
    let cls = text_target_class();
    let target: id = msg_send![cls, alloc];
    let target: id = msg_send![target, init];
    let change_ptr = on_change
        .map(|callback| Box::into_raw(Box::new(callback)) as *mut c_void)
        .unwrap_or(ptr::null_mut());
    let submit_ptr = on_submit
        .map(|callback| Box::into_raw(Box::new(callback)) as *mut c_void)
        .unwrap_or(ptr::null_mut());
    unsafe {
        (*target).set_ivar(CHANGE_CALLBACK_IVAR, change_ptr);
        (*target).set_ivar(SUBMIT_CALLBACK_IVAR, submit_ptr);
    }
    target
}

fn bounds_to_ns_rect(bounds: Bounds<Pixels>, parent_height: f64) -> NSRect {
    let x: f64 = bounds.origin.x.into();
    let y: f64 = bounds.origin.y.into();
    let width: f64 = bounds.size.width.into();
    let height: f64 = bounds.size.height.into();
    let flipped_y = parent_height - y - height;
    NSRect::new(
        NSPoint::new(x, flipped_y),
        NSSize::new(width.max(1.0), height.max(1.0)),
    )
}

unsafe fn attach_and_position(parent: id, view: id, bounds: Bounds<Pixels>) {
    let parent_frame: NSRect = msg_send![parent, frame];
    let frame = bounds_to_ns_rect(bounds, parent_frame.size.height);
    let superview: id = msg_send![view, superview];
    if superview.is_null() {
        let _: () = msg_send![parent, addSubview: view];
    }
    let _: () = msg_send![view, setFrame: frame];
}

unsafe fn remove_from_parent(view: id) {
    let superview: id = msg_send![view, superview];
    if !superview.is_null() {
        let _: () = msg_send![view, removeFromSuperview];
    }
}

unsafe fn release_target(target: *mut c_void) {
    if !target.is_null() {
        let _: () = msg_send![target as id, release];
    }
}

unsafe fn cleanup_view_and_target(view: *mut c_void, target: *mut c_void) {
    if !view.is_null() {
        unsafe {
            remove_from_parent(view as id);
        }
    }
    unsafe {
        release_target(target);
    }
    if !view.is_null() {
        let _: () = msg_send![view as id, release];
    }
}

unsafe fn cleanup_view_only(view: *mut c_void, _target: *mut c_void) {
    if !view.is_null() {
        unsafe {
            remove_from_parent(view as id);
        }
        let _: () = msg_send![view as id, release];
    }
}

unsafe fn set_target_action(view: id, target: id) {
    let _: () = msg_send![view, setTarget: target];
    let _: () = msg_send![view, setAction: sel!(performAction:)];
}

unsafe fn clear_target_action(view: id) {
    let _: () = msg_send![view, setTarget: nil];
    let _: () = msg_send![view, setAction: nil];
}

unsafe fn set_delegate(view: id, delegate: id) {
    let _: () = msg_send![view, setDelegate: delegate];
}

unsafe fn set_enabled(view: id, enabled: bool) {
    let _: () = msg_send![view, setEnabled: enabled.to_objc()];
}

unsafe fn create_button(title: &str) -> id {
    let button: id = msg_send![class!(NSButton), alloc];
    let button: id = msg_send![button, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(80.0, 24.0))];
    let title = unsafe { ns_string(title) };
    let _: () = msg_send![button, setTitle: title];
    button
}

unsafe fn apply_button_style(view: id, style: ButtonStyle) {
    match style {
        ButtonStyle::Borderless => {
            let _: () = msg_send![view, setBordered: false];
        }
        ButtonStyle::Inline => {
            let _: () = msg_send![view, setBordered: true];
            let _: () = msg_send![view, setBezelStyle: 10isize];
        }
        ButtonStyle::Filled => {
            let _: () = msg_send![view, setBordered: true];
            let _: () = msg_send![view, setBezelStyle: 1isize];
        }
        ButtonStyle::Rounded => {
            let _: () = msg_send![view, setBordered: true];
        }
    }
}

unsafe fn create_switch() -> id {
    let view: id = msg_send![class!(NSSwitch), alloc];
    msg_send![view, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(40.0, 22.0))]
}

unsafe fn create_slider() -> id {
    let view: id = msg_send![class!(NSSlider), alloc];
    msg_send![view, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(140.0, 24.0))]
}

unsafe fn create_progress() -> id {
    let view: id = msg_send![class!(NSProgressIndicator), alloc];
    msg_send![view, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(140.0, 14.0))]
}

unsafe fn create_text_field(config: &TextFieldConfig<'_>) -> id {
    let view: id = match (config.style, config.secure) {
        (TextFieldStyle::Search, _) => {
            let search_field: id = msg_send![class!(NSSearchField), alloc];
            msg_send![
                search_field,
                initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, 24.0))
            ]
        }
        (_, true) => {
            let secure_field: id = msg_send![class!(NSSecureTextField), alloc];
            msg_send![
                secure_field,
                initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, 24.0))
            ]
        }
        _ => {
            let text_field: id = msg_send![class!(NSTextField), alloc];
            msg_send![
                text_field,
                initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, 24.0))
            ]
        }
    };
    unsafe {
        apply_text_field_config(view, config);
    }
    view
}

unsafe fn apply_text_field_config(view: id, config: &TextFieldConfig<'_>) {
    let placeholder = config.placeholder.map(|value| unsafe { ns_string(value) });
    let current_value = unsafe { current_string_value(view) };

    if current_value != config.value {
        let _: () = msg_send![view, setStringValue: unsafe { ns_string(config.value) }];
    }
    let _: () = msg_send![view, setEditable: config.editable.to_objc()];
    let _: () = msg_send![view, setSelectable: config.selectable.to_objc()];
    let _: () = msg_send![view, setBezeled: config.bezeled.to_objc()];
    let _: () = msg_send![view, setDrawsBackground: config.bezeled.to_objc()];
    unsafe {
        set_enabled(view, config.enabled);
    }

    if let Some(placeholder) = placeholder {
        let _: () = msg_send![view, setPlaceholderString: placeholder];
    } else {
        let _: () = msg_send![view, setPlaceholderString: nil];
    }

    if let Some(size) = config.font_size {
        let font: id = msg_send![class!(NSFont), systemFontOfSize: size];
        let _: () = msg_send![view, setFont: font];
    }
}

unsafe fn current_string_value(view: id) -> String {
    let value: id = msg_send![view, stringValue];
    let utf8: *const i8 = msg_send![value, UTF8String];
    if utf8.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(utf8) }
            .to_string_lossy()
            .into_owned()
    }
}

unsafe fn text_field_matches_config(view: id, config: &TextFieldConfig<'_>) -> bool {
    let expected_class = match (config.style, config.secure) {
        (TextFieldStyle::Search, _) => class!(NSSearchField),
        (_, true) => class!(NSSecureTextField),
        _ => class!(NSTextField),
    };
    let is_match: bool = msg_send![view, isKindOfClass: expected_class];
    is_match
}

impl PlatformNativeControls for MacNativeControls {
    fn update_button(
        &self,
        state: &mut NativeControlState,
        parent: *mut c_void,
        bounds: Bounds<Pixels>,
        _scale: f32,
        config: ButtonConfig<'_>,
    ) {
        unsafe {
            let view = if state.is_initialized() {
                state.view() as id
            } else {
                let view = create_button(config.title);
                *state = NativeControlState::new(
                    view as *mut c_void,
                    ptr::null_mut(),
                    cleanup_view_and_target,
                );
                view
            };

            let _: () = msg_send![view, setTitle: ns_string(config.title)];
            apply_button_style(view, config.style);
            set_enabled(view, config.enabled);

            release_target(state.target());
            state.set_target(ptr::null_mut());
            if let Some(callback) = config.on_click {
                let target = create_void_target(callback);
                set_target_action(view, target);
                state.set_target(target as *mut c_void);
            } else {
                let _: () = msg_send![view, setTarget: nil];
                let _: () = msg_send![view, setAction: nil];
            }

            attach_and_position(parent as id, view, bounds);
        }
    }

    fn update_switch(
        &self,
        state: &mut NativeControlState,
        parent: *mut c_void,
        bounds: Bounds<Pixels>,
        _scale: f32,
        config: SwitchConfig,
    ) {
        unsafe {
            let view = if state.is_initialized() {
                state.view() as id
            } else {
                let view = create_switch();
                *state = NativeControlState::new(
                    view as *mut c_void,
                    ptr::null_mut(),
                    cleanup_view_and_target,
                );
                view
            };

            let _: () = msg_send![view, setState: if config.checked { 1isize } else { 0isize }];
            set_enabled(view, config.enabled);

            release_target(state.target());
            state.set_target(ptr::null_mut());
            if let Some(callback) = config.on_change {
                let target = create_bool_target(callback);
                set_target_action(view, target);
                state.set_target(target as *mut c_void);
            } else {
                let _: () = msg_send![view, setTarget: nil];
                let _: () = msg_send![view, setAction: nil];
            }

            attach_and_position(parent as id, view, bounds);
        }
    }

    fn update_slider(
        &self,
        state: &mut NativeControlState,
        parent: *mut c_void,
        bounds: Bounds<Pixels>,
        _scale: f32,
        config: SliderConfig,
    ) {
        unsafe {
            let view = if state.is_initialized() {
                state.view() as id
            } else {
                let view = create_slider();
                *state = NativeControlState::new(
                    view as *mut c_void,
                    ptr::null_mut(),
                    cleanup_view_and_target,
                );
                view
            };

            let _: () = msg_send![view, setMinValue: config.min];
            let _: () = msg_send![view, setMaxValue: config.max];
            let _: () = msg_send![view, setDoubleValue: config.value];
            set_enabled(view, config.enabled);

            release_target(state.target());
            state.set_target(ptr::null_mut());
            if let Some(callback) = config.on_change {
                let target = create_f64_target(callback);
                set_target_action(view, target);
                state.set_target(target as *mut c_void);
            } else {
                let _: () = msg_send![view, setTarget: nil];
                let _: () = msg_send![view, setAction: nil];
            }

            attach_and_position(parent as id, view, bounds);
        }
    }

    fn update_progress(
        &self,
        state: &mut NativeControlState,
        parent: *mut c_void,
        bounds: Bounds<Pixels>,
        _scale: f32,
        config: ProgressConfig,
    ) {
        unsafe {
            let view = if state.is_initialized() {
                state.view() as id
            } else {
                let view = create_progress();
                *state = NativeControlState::new(
                    view as *mut c_void,
                    ptr::null_mut(),
                    cleanup_view_only,
                );
                view
            };

            let _: () = msg_send![
                view,
                setStyle: match config.style {
                    ProgressStyle::Bar => 0isize,
                    ProgressStyle::Spinning => 1isize,
                }
            ];
            let _: () = msg_send![view, setMinValue: config.min];
            let _: () = msg_send![view, setMaxValue: config.max];

            match config.value {
                Some(value) => {
                    let _: () = msg_send![view, setIndeterminate: false];
                    let _: () = msg_send![view, setDoubleValue: value];
                }
                None => {
                    let _: () = msg_send![view, setIndeterminate: true];
                    let _: () = msg_send![view, startAnimation: nil];
                }
            }

            attach_and_position(parent as id, view, bounds);
        }
    }

    fn update_text_field(
        &self,
        state: &mut NativeControlState,
        parent: *mut c_void,
        bounds: Bounds<Pixels>,
        _scale: f32,
        config: TextFieldConfig<'_>,
    ) {
        unsafe {
            if state.is_initialized() && !text_field_matches_config(state.view() as id, &config) {
                cleanup_view_and_target(state.view(), state.target());
                *state = NativeControlState::default();
            }

            let view = if state.is_initialized() {
                state.view() as id
            } else {
                let view = create_text_field(&config);
                *state = NativeControlState::new(
                    view as *mut c_void,
                    ptr::null_mut(),
                    cleanup_view_and_target,
                );
                view
            };

            apply_text_field_config(view, &config);

            release_target(state.target());
            state.set_target(ptr::null_mut());

            if config.on_change.is_some() || config.on_submit.is_some() {
                let target = create_text_target(config.on_change, config.on_submit);
                set_delegate(view, target);
                set_target_action(view, target);
                state.set_target(target as *mut c_void);
            } else {
                set_delegate(view, nil);
                clear_target_action(view);
            }

            attach_and_position(parent as id, view, bounds);
        }
    }
}
