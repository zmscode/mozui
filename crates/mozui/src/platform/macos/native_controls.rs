//! macOS implementation of PlatformNativeControls using objc2.
//!
//! All target delegate classes use `define_class!` with typed ivars and
//! automatic dealloc. Views and targets are stored as owned raw pointers in
//! `NativeControlState`; ownership is recovered via `Retained::from_raw` in
//! cleanup functions.

use crate::platform::native_controls::{
    ButtonConfig, ButtonStyle, GlassEffectConfig, GlassEffectStyle, ImageViewConfig,
    NativeControlState, PlatformNativeControls, ProgressConfig, ProgressStyle, SliderConfig,
    SwitchConfig, TextFieldConfig, TextFieldStyle, VisualEffectActiveState, VisualEffectBlending,
    VisualEffectConfig, VisualEffectMaterial,
};
use crate::{Bounds, Pixels};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AllocAnyThread, DefinedClass, MainThreadMarker, define_class, msg_send, sel};
use objc2_app_kit::{
    NSButton, NSControl, NSImageSymbolConfiguration, NSImageView, NSProgressIndicator,
    NSSearchField, NSSecureTextField, NSSlider, NSSwitch, NSTextField, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
};
use objc2_foundation::{NSNotification, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;

pub struct MacNativeControls;
pub static MAC_NATIVE_CONTROLS: MacNativeControls = MacNativeControls;

// ---------------------------------------------------------------------------
// Callback types
// ---------------------------------------------------------------------------

type VoidCb = RefCell<Option<Box<dyn Fn()>>>;
type BoolCb = RefCell<Option<Box<dyn Fn(bool)>>>;
type F64Cb = RefCell<Option<Box<dyn Fn(f64)>>>;

// ---------------------------------------------------------------------------
// VoidTarget – button / generic no-arg action
// ---------------------------------------------------------------------------

struct VoidTargetIvars {
    callback: VoidCb,
}

impl Default for VoidTargetIvars {
    fn default() -> Self {
        Self {
            callback: RefCell::new(None),
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = VoidTargetIvars]
    #[name = "MozuiNativeVoidTarget"]
    struct VoidTarget;

    impl VoidTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, _sender: &AnyObject) {
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                f();
            }
        }
    }

    unsafe impl NSObjectProtocol for VoidTarget {}
);

impl VoidTarget {
    fn new(callback: Box<dyn Fn()>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(VoidTargetIvars {
            callback: RefCell::new(Some(callback)),
        });
        unsafe { msg_send![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// BoolTarget – switch / toggle
// ---------------------------------------------------------------------------

struct BoolTargetIvars {
    callback: BoolCb,
}

impl Default for BoolTargetIvars {
    fn default() -> Self {
        Self {
            callback: RefCell::new(None),
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = BoolTargetIvars]
    #[name = "MozuiNativeBoolTarget"]
    struct BoolTarget;

    impl BoolTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &AnyObject) {
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                let state: isize = unsafe { msg_send![sender, state] };
                f(state != 0);
            }
        }
    }

    unsafe impl NSObjectProtocol for BoolTarget {}
);

impl BoolTarget {
    fn new(callback: Box<dyn Fn(bool)>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(BoolTargetIvars {
            callback: RefCell::new(Some(callback)),
        });
        unsafe { msg_send![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// F64Target – slider / stepper
// ---------------------------------------------------------------------------

struct F64TargetIvars {
    callback: F64Cb,
}

impl Default for F64TargetIvars {
    fn default() -> Self {
        Self {
            callback: RefCell::new(None),
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = F64TargetIvars]
    #[name = "MozuiNativeF64Target"]
    struct F64Target;

    impl F64Target {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &AnyObject) {
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                let value: f64 = unsafe { msg_send![sender, doubleValue] };
                f(value);
            }
        }
    }

    unsafe impl NSObjectProtocol for F64Target {}
);

impl F64Target {
    fn new(callback: Box<dyn Fn(f64)>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(F64TargetIvars {
            callback: RefCell::new(Some(callback)),
        });
        unsafe { msg_send![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// TextTarget – text field change + submit delegate
// ---------------------------------------------------------------------------

struct TextTargetIvars {
    on_change: RefCell<Option<Box<dyn Fn(String)>>>,
    on_submit: RefCell<Option<Box<dyn Fn(String)>>>,
}

impl Default for TextTargetIvars {
    fn default() -> Self {
        Self {
            on_change: RefCell::new(None),
            on_submit: RefCell::new(None),
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = TextTargetIvars]
    #[name = "MozuiNativeTextTarget"]
    struct TextTarget;

    impl TextTarget {
        /// NSTextFieldDelegate: called on every keystroke.
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, notification: &NSNotification) {
            let cb = self.ivars().on_change.borrow();
            if let Some(ref f) = *cb {
                let value = unsafe { string_value_from_notification(notification) };
                f(value);
            }
        }

        /// Target/action: called on Enter / field commit.
        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &AnyObject) {
            let cb = self.ivars().on_submit.borrow();
            if let Some(ref f) = *cb {
                let value = unsafe { string_value_of_object(sender) };
                f(value);
            }
        }
    }

    unsafe impl NSObjectProtocol for TextTarget {}
);

impl TextTarget {
    fn new(
        on_change: Option<Box<dyn Fn(String)>>,
        on_submit: Option<Box<dyn Fn(String)>>,
    ) -> Retained<Self> {
        let this = Self::alloc().set_ivars(TextTargetIvars {
            on_change: RefCell::new(on_change),
            on_submit: RefCell::new(on_submit),
        });
        unsafe { msg_send![super(this), init] }
    }
}

// ---------------------------------------------------------------------------
// String extraction helpers
// ---------------------------------------------------------------------------

unsafe fn string_value_from_notification(notification: &NSNotification) -> String {
    let object: *mut AnyObject = msg_send![notification, object];
    if object.is_null() {
        return String::new();
    }
    unsafe { string_value_of_object(&*object) }
}

unsafe fn string_value_of_object(obj: &AnyObject) -> String {
    let ns_str_ptr: *mut NSString = msg_send![obj, stringValue];
    if ns_str_ptr.is_null() {
        return String::new();
    }
    unsafe { (*ns_str_ptr).to_string() }
}

// ---------------------------------------------------------------------------
// Coordinate conversion (AppKit flipped Y)
// ---------------------------------------------------------------------------

fn bounds_to_ns_rect(bounds: Bounds<Pixels>, parent_height: f64) -> NSRect {
    let x: f64 = bounds.origin.x.into();
    let y: f64 = bounds.origin.y.into();
    let w: f64 = bounds.size.width.into();
    let h: f64 = bounds.size.height.into();
    let flipped_y = parent_height - y - h;
    NSRect::new(
        NSPoint::new(x, flipped_y),
        NSSize::new(w.max(1.0), h.max(1.0)),
    )
}

// ---------------------------------------------------------------------------
// View lifecycle helpers
// ---------------------------------------------------------------------------

unsafe fn attach_and_position(parent: *mut c_void, view: *mut c_void, bounds: Bounds<Pixels>) {
    if parent.is_null() || view.is_null() {
        return;
    }
    let parent_ref: &NSView = unsafe { &*(parent as *const NSView) };
    let view_ref: &NSView = unsafe { &*(view as *const NSView) };
    let parent_frame = parent_ref.frame();
    let frame = bounds_to_ns_rect(bounds, parent_frame.size.height);
    if unsafe { view_ref.superview() }.is_none() {
        parent_ref.addSubview(view_ref);
    }
    view_ref.setFrame(frame);
}

/// Cleanup: remove from superview and drop both view and target.
unsafe fn cleanup_view_and_target(view: *mut c_void, target: *mut c_void) {
    if !target.is_null() {
        drop(unsafe { Retained::from_raw(target as *mut NSObject) });
    }
    if !view.is_null() {
        if let Some(v) = unsafe { Retained::from_raw(view as *mut NSView) } {
            v.removeFromSuperview();
        }
    }
}

/// Cleanup: remove from superview and drop view; no target.
unsafe fn cleanup_view_only(view: *mut c_void, _target: *mut c_void) {
    if !view.is_null() {
        if let Some(v) = unsafe { Retained::from_raw(view as *mut NSView) } {
            v.removeFromSuperview();
        }
    }
}

// ---------------------------------------------------------------------------
// Wire target/action helpers
// ---------------------------------------------------------------------------

unsafe fn set_target_action(view: *mut c_void, target: &AnyObject) {
    let control: &NSControl = unsafe { &*(view as *const NSControl) };
    unsafe {
        NSControl::setTarget(control, Some(target));
        NSControl::setAction(control, Some(sel!(performAction:)));
    }
}

unsafe fn clear_target_action(view: *mut c_void) {
    let control: &NSControl = unsafe { &*(view as *const NSControl) };
    unsafe {
        NSControl::setTarget(control, None);
        NSControl::setAction(control, None);
    }
}

unsafe fn set_delegate(view: *mut c_void, delegate: *mut AnyObject) {
    let _: () = unsafe { msg_send![view as *mut AnyObject, setDelegate: delegate] };
}

unsafe fn set_enabled(view: *mut c_void, enabled: bool) {
    let control: &NSControl = unsafe { &*(view as *const NSControl) };
    NSControl::setEnabled(control, enabled);
}

/// Release and clear the old target stored in `state`, returning null.
unsafe fn release_old_target(state: &mut NativeControlState) {
    let ptr = state.target();
    if !ptr.is_null() {
        drop(unsafe { Retained::from_raw(ptr as *mut NSObject) });
    }
    state.set_target(ptr::null_mut());
}

// ---------------------------------------------------------------------------
// Control creation helpers
// ---------------------------------------------------------------------------

unsafe fn create_button(title: &str, mtm: MainThreadMarker) -> *mut c_void {
    let ns_title = NSString::from_str(title);
    let button = unsafe { NSButton::buttonWithTitle_target_action(&ns_title, None, None, mtm) };
    Retained::into_raw(unsafe { Retained::cast_unchecked::<NSView>(button) }) as *mut c_void
}

unsafe fn apply_button_style(view: *mut c_void, style: ButtonStyle) {
    let view_obj = view as *mut AnyObject;
    match style {
        ButtonStyle::Borderless => {
            let _: () = unsafe { msg_send![view_obj, setBordered: false] };
        }
        ButtonStyle::Inline => {
            let _: () = unsafe { msg_send![view_obj, setBordered: true] };
            let _: () = unsafe { msg_send![view_obj, setBezelStyle: 10isize] };
        }
        ButtonStyle::Filled | ButtonStyle::Rounded => {
            let _: () = unsafe { msg_send![view_obj, setBordered: true] };
            let _: () = unsafe { msg_send![view_obj, setBezelStyle: 1isize] };
        }
    }
}

unsafe fn create_switch(mtm: MainThreadMarker) -> *mut c_void {
    let sw = NSSwitch::new(mtm);
    Retained::into_raw(unsafe { Retained::cast_unchecked::<NSView>(sw) }) as *mut c_void
}

unsafe fn create_slider(mtm: MainThreadMarker) -> *mut c_void {
    let zero = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(140.0, 24.0));
    let slider: Retained<NSSlider> =
        unsafe { msg_send![mtm.alloc::<NSSlider>(), initWithFrame: zero] };
    Retained::into_raw(unsafe { Retained::cast_unchecked::<NSView>(slider) }) as *mut c_void
}

unsafe fn create_progress(mtm: MainThreadMarker) -> *mut c_void {
    let zero = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(140.0, 14.0));
    let prog: Retained<NSProgressIndicator> =
        unsafe { msg_send![mtm.alloc::<NSProgressIndicator>(), initWithFrame: zero] };
    Retained::into_raw(unsafe { Retained::cast_unchecked::<NSView>(prog) }) as *mut c_void
}

unsafe fn create_text_field(config: &TextFieldConfig<'_>, mtm: MainThreadMarker) -> *mut c_void {
    let zero = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, 24.0));
    let view: Retained<NSView> = match (config.style, config.secure) {
        (TextFieldStyle::Search, _) => {
            let sf: Retained<NSSearchField> =
                unsafe { msg_send![mtm.alloc::<NSSearchField>(), initWithFrame: zero] };
            unsafe { Retained::cast_unchecked(sf) }
        }
        (_, true) => {
            let sf: Retained<NSSecureTextField> =
                unsafe { msg_send![mtm.alloc::<NSSecureTextField>(), initWithFrame: zero] };
            unsafe { Retained::cast_unchecked(sf) }
        }
        _ => {
            let tf: Retained<NSTextField> =
                unsafe { msg_send![mtm.alloc::<NSTextField>(), initWithFrame: zero] };
            unsafe { Retained::cast_unchecked(tf) }
        }
    };
    Retained::into_raw(view) as *mut c_void
}

unsafe fn apply_text_field_config(view: *mut c_void, config: &TextFieldConfig<'_>) {
    let obj = view as *mut AnyObject;

    // Only update stringValue if it changed (avoids fighting with user edits).
    let current_ns_str: *mut NSString = unsafe { msg_send![obj, stringValue] };
    let current_str = if current_ns_str.is_null() {
        String::new()
    } else {
        unsafe { (*current_ns_str).to_string() }
    };
    if current_str != config.value {
        let ns_val = NSString::from_str(config.value);
        let _: () = unsafe { msg_send![obj, setStringValue: &*ns_val] };
    }

    let _: () = unsafe { msg_send![obj, setEditable: config.editable] };
    let _: () = unsafe { msg_send![obj, setSelectable: config.selectable] };
    let _: () = unsafe { msg_send![obj, setBezeled: config.bezeled] };
    let _: () = unsafe { msg_send![obj, setDrawsBackground: config.bezeled] };
    unsafe { set_enabled(view, config.enabled) };

    if let Some(placeholder) = config.placeholder {
        let ns_ph = NSString::from_str(placeholder);
        let _: () = unsafe { msg_send![obj, setPlaceholderString: &*ns_ph] };
    } else {
        let _: () = unsafe { msg_send![obj, setPlaceholderString: ptr::null::<NSString>()] };
    }

    if let Some(size) = config.font_size {
        let font: *mut AnyObject = unsafe {
            msg_send![
                objc2::class!(NSFont),
                systemFontOfSize: size
            ]
        };
        if !font.is_null() {
            let _: () = unsafe { msg_send![obj, setFont: font] };
        }
    }
}

unsafe fn text_field_class_matches(view: *mut c_void, config: &TextFieldConfig<'_>) -> bool {
    let obj = view as *mut AnyObject;
    let expected_class: *const objc2::runtime::AnyClass = match (config.style, config.secure) {
        (TextFieldStyle::Search, _) => unsafe { msg_send![objc2::class!(NSSearchField), class] },
        (_, true) => unsafe { msg_send![objc2::class!(NSSecureTextField), class] },
        _ => unsafe { msg_send![objc2::class!(NSTextField), class] },
    };
    let is_match: bool = unsafe { msg_send![obj, isKindOfClass: expected_class] };
    is_match
}

// ---------------------------------------------------------------------------
// NSVisualEffectMaterial / blending / state mapping
// ---------------------------------------------------------------------------

fn material_to_ns(m: VisualEffectMaterial) -> NSVisualEffectMaterial {
    match m {
        VisualEffectMaterial::Titlebar => NSVisualEffectMaterial::Titlebar,
        VisualEffectMaterial::Selection => NSVisualEffectMaterial::Selection,
        VisualEffectMaterial::Menu => NSVisualEffectMaterial::Menu,
        VisualEffectMaterial::Popover => NSVisualEffectMaterial::Popover,
        VisualEffectMaterial::Sidebar => NSVisualEffectMaterial::Sidebar,
        VisualEffectMaterial::HeaderView => NSVisualEffectMaterial::HeaderView,
        VisualEffectMaterial::Sheet => NSVisualEffectMaterial::Sheet,
        VisualEffectMaterial::WindowBackground => NSVisualEffectMaterial::WindowBackground,
        VisualEffectMaterial::HudWindow => NSVisualEffectMaterial::HUDWindow,
        VisualEffectMaterial::FullScreenUI => NSVisualEffectMaterial::FullScreenUI,
        VisualEffectMaterial::ToolTip => NSVisualEffectMaterial::ToolTip,
        VisualEffectMaterial::ContentBackground => NSVisualEffectMaterial::ContentBackground,
        VisualEffectMaterial::UnderWindowBackground => {
            NSVisualEffectMaterial::UnderWindowBackground
        }
        VisualEffectMaterial::UnderPageBackground => NSVisualEffectMaterial::UnderPageBackground,
    }
}

fn blending_to_ns(b: VisualEffectBlending) -> NSVisualEffectBlendingMode {
    match b {
        VisualEffectBlending::BehindWindow => NSVisualEffectBlendingMode::BehindWindow,
        VisualEffectBlending::WithinWindow => NSVisualEffectBlendingMode::WithinWindow,
    }
}

fn active_state_to_ns(s: VisualEffectActiveState) -> NSVisualEffectState {
    match s {
        VisualEffectActiveState::FollowsWindowActiveState => {
            NSVisualEffectState::FollowsWindowActiveState
        }
        VisualEffectActiveState::Active => NSVisualEffectState::Active,
        VisualEffectActiveState::Inactive => NSVisualEffectState::Inactive,
    }
}

// ---------------------------------------------------------------------------
// PlatformNativeControls implementation
// ---------------------------------------------------------------------------

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
            let mtm = MainThreadMarker::new_unchecked();

            if !state.is_initialized() {
                let view = create_button(config.title, mtm);
                *state = NativeControlState::new(view, ptr::null_mut(), cleanup_view_and_target);
            } else {
                // Update title.
                let ns_title = NSString::from_str(config.title);
                let _: () = msg_send![state.view() as *mut AnyObject, setTitle: &*ns_title];
            }

            apply_button_style(state.view(), config.style);
            set_enabled(state.view(), config.enabled);

            release_old_target(state);
            if let Some(callback) = config.on_click {
                let target = VoidTarget::new(callback);
                set_target_action(state.view(), &target);
                state.set_target(Retained::into_raw(target) as *mut c_void);
            } else {
                clear_target_action(state.view());
            }

            attach_and_position(parent, state.view(), bounds);
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
            let mtm = MainThreadMarker::new_unchecked();

            if !state.is_initialized() {
                let view = create_switch(mtm);
                *state = NativeControlState::new(view, ptr::null_mut(), cleanup_view_and_target);
            }

            let obj = state.view() as *mut AnyObject;
            let ns_state: isize = if config.checked { 1 } else { 0 };
            let _: () = msg_send![obj, setState: ns_state];
            set_enabled(state.view(), config.enabled);

            release_old_target(state);
            if let Some(callback) = config.on_change {
                let target = BoolTarget::new(callback);
                set_target_action(state.view(), &target);
                state.set_target(Retained::into_raw(target) as *mut c_void);
            } else {
                clear_target_action(state.view());
            }

            attach_and_position(parent, state.view(), bounds);
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
            let mtm = MainThreadMarker::new_unchecked();

            if !state.is_initialized() {
                let view = create_slider(mtm);
                *state = NativeControlState::new(view, ptr::null_mut(), cleanup_view_and_target);
            }

            let obj = state.view() as *mut AnyObject;
            let _: () = msg_send![obj, setMinValue: config.min];
            let _: () = msg_send![obj, setMaxValue: config.max];
            let _: () = msg_send![obj, setDoubleValue: config.value];
            set_enabled(state.view(), config.enabled);

            release_old_target(state);
            if let Some(callback) = config.on_change {
                let target = F64Target::new(callback);
                set_target_action(state.view(), &target);
                state.set_target(Retained::into_raw(target) as *mut c_void);
            } else {
                clear_target_action(state.view());
            }

            attach_and_position(parent, state.view(), bounds);
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
            let mtm = MainThreadMarker::new_unchecked();

            if !state.is_initialized() {
                let view = create_progress(mtm);
                *state = NativeControlState::new(view, ptr::null_mut(), cleanup_view_only);
            }

            let obj = state.view() as *mut AnyObject;
            let style_val: isize = match config.style {
                ProgressStyle::Bar => 0,
                ProgressStyle::Spinning => 1,
            };
            let _: () = msg_send![obj, setStyle: style_val];
            let _: () = msg_send![obj, setMinValue: config.min];
            let _: () = msg_send![obj, setMaxValue: config.max];

            match config.value {
                Some(value) => {
                    let _: () = msg_send![obj, setIndeterminate: false];
                    let _: () = msg_send![obj, setDoubleValue: value];
                }
                None => {
                    let _: () = msg_send![obj, setIndeterminate: true];
                    let _: () = msg_send![obj, startAnimation: ptr::null::<AnyObject>()];
                }
            }

            attach_and_position(parent, state.view(), bounds);
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
            let mtm = MainThreadMarker::new_unchecked();

            // Recreate if field type changed (e.g. plain → search).
            if state.is_initialized() && !text_field_class_matches(state.view(), &config) {
                cleanup_view_and_target(state.view(), state.target());
                *state = NativeControlState::default();
            }

            if !state.is_initialized() {
                let view = create_text_field(&config, mtm);
                *state = NativeControlState::new(view, ptr::null_mut(), cleanup_view_and_target);
            }

            apply_text_field_config(state.view(), &config);

            release_old_target(state);
            if config.on_change.is_some() || config.on_submit.is_some() {
                let target = TextTarget::new(config.on_change, config.on_submit);
                let target_ptr = Retained::into_raw(target);
                set_delegate(state.view(), target_ptr as *mut AnyObject);
                set_target_action(state.view(), &*(target_ptr as *const AnyObject));
                state.set_target(target_ptr as *mut c_void);
            } else {
                set_delegate(state.view(), ptr::null_mut());
                clear_target_action(state.view());
            }

            attach_and_position(parent, state.view(), bounds);
        }
    }

    fn update_visual_effect(
        &self,
        state: &mut NativeControlState,
        parent: *mut c_void,
        bounds: Bounds<Pixels>,
        _scale: f32,
        config: VisualEffectConfig,
    ) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();

            if !state.is_initialized() {
                let view = NSVisualEffectView::new(mtm);
                let view_ptr =
                    Retained::into_raw(Retained::cast_unchecked::<NSView>(view)) as *mut c_void;
                *state = NativeControlState::new(view_ptr, ptr::null_mut(), cleanup_view_only);
            }

            let view: &NSVisualEffectView = &*(state.view() as *const NSVisualEffectView);
            view.setMaterial(material_to_ns(config.material));
            view.setBlendingMode(blending_to_ns(config.blending));
            view.setState(active_state_to_ns(config.active_state));
            view.setEmphasized(config.is_emphasized);

            attach_and_position(parent, state.view(), bounds);
        }
    }

    fn update_glass_effect(
        &self,
        state: &mut NativeControlState,
        parent: *mut c_void,
        bounds: Bounds<Pixels>,
        _scale: f32,
        config: GlassEffectConfig,
    ) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();

            if !state.is_initialized() {
                let view = create_glass_view(&config, mtm);
                let view_ptr = Retained::into_raw(view) as *mut c_void;
                *state = NativeControlState::new(view_ptr, ptr::null_mut(), cleanup_view_only);
            }

            attach_and_position(parent, state.view(), bounds);
        }
    }

    fn update_image_view(
        &self,
        state: &mut NativeControlState,
        parent: *mut c_void,
        bounds: Bounds<Pixels>,
        _scale: f32,
        config: ImageViewConfig<'_>,
    ) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();

            if !state.is_initialized() {
                if let Some(view) = create_symbol_image_view(&config, mtm) {
                    let view_ptr = Retained::into_raw(view) as *mut c_void;
                    *state = NativeControlState::new(view_ptr, ptr::null_mut(), cleanup_view_only);
                } else {
                    return; // Symbol not found — skip.
                }
            } else {
                // Update image in place.
                update_symbol_image(state.view(), &config);
            }

            attach_and_position(parent, state.view(), bounds);
        }
    }
}

// ---------------------------------------------------------------------------
// Glass effect factory
// ---------------------------------------------------------------------------

unsafe fn create_glass_view(config: &GlassEffectConfig, mtm: MainThreadMarker) -> Retained<NSView> {
    use objc2::runtime::AnyClass;

    if let Some(cls) = AnyClass::get(c"NSGlassEffectView") {
        let view: Retained<NSView> = unsafe { msg_send![cls, new] };
        let style_val: isize = match config.style {
            GlassEffectStyle::Regular => 0,
            GlassEffectStyle::Clear => 1,
        };
        let _: () = unsafe { msg_send![&view, setStyle: style_val] };
        if let Some(radius) = config.corner_radius {
            let _: () = unsafe { msg_send![&view, setCornerRadius: radius] };
        }
        if let Some((r, g, b, a)) = config.tint_color {
            let color: *mut AnyObject = unsafe {
                msg_send![
                    objc2::class!(NSColor),
                    colorWithRed: r,
                    green: g,
                    blue: b,
                    alpha: a
                ]
            };
            if !color.is_null() {
                let _: () = unsafe { msg_send![&view, setTintColor: color] };
            }
        }
        view
    } else {
        // macOS < 26: fall back to NSVisualEffectView.
        let vev = NSVisualEffectView::new(mtm);
        vev.setMaterial(NSVisualEffectMaterial::HUDWindow);
        vev.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        vev.setState(NSVisualEffectState::Active);
        unsafe { Retained::cast_unchecked(vev) }
    }
}

// ---------------------------------------------------------------------------
// SF Symbol image view factory
// ---------------------------------------------------------------------------

unsafe fn create_symbol_image_view(
    config: &ImageViewConfig<'_>,
    mtm: MainThreadMarker,
) -> Option<Retained<NSView>> {
    let ns_name = NSString::from_str(config.symbol_name);

    let image: Option<Retained<objc2_app_kit::NSImage>> = unsafe {
        msg_send![
            objc2::class!(NSImage),
            imageWithSystemSymbolName: &*ns_name,
            accessibilityDescription: ptr::null::<NSString>()
        ]
    };
    let image = image?;

    let conf: Retained<NSImageSymbolConfiguration> = unsafe {
        msg_send![
            objc2::class!(NSImageSymbolConfiguration),
            configurationWithPointSize: config.point_size,
            weight: config.weight.to_ns_weight(),
            scale: config.scale.to_ns_scale()
        ]
    };

    let configured_image: Retained<objc2_app_kit::NSImage> =
        unsafe { msg_send![&image, imageWithSymbolConfiguration: &*conf] };

    let image_view = NSImageView::imageViewWithImage(&configured_image, mtm);

    if let Some((r, g, b, a)) = config.tint_color {
        let color: *mut AnyObject = unsafe {
            msg_send![
                objc2::class!(NSColor),
                colorWithRed: r,
                green: g,
                blue: b,
                alpha: a
            ]
        };
        if !color.is_null() {
            let _: () = unsafe { msg_send![&image_view, setContentTintColor: color] };
        }
    }

    Some(unsafe { Retained::cast_unchecked(image_view) })
}

unsafe fn update_symbol_image(view: *mut c_void, config: &ImageViewConfig<'_>) {
    let ns_name = NSString::from_str(config.symbol_name);

    let image: Option<Retained<objc2_app_kit::NSImage>> = unsafe {
        msg_send![
            objc2::class!(NSImage),
            imageWithSystemSymbolName: &*ns_name,
            accessibilityDescription: ptr::null::<NSString>()
        ]
    };
    if let Some(image) = image {
        let conf: Retained<NSImageSymbolConfiguration> = unsafe {
            msg_send![
                objc2::class!(NSImageSymbolConfiguration),
                configurationWithPointSize: config.point_size,
                weight: config.weight.to_ns_weight(),
                scale: config.scale.to_ns_scale()
            ]
        };
        let configured_image: Retained<objc2_app_kit::NSImage> =
            unsafe { msg_send![&image, imageWithSymbolConfiguration: &*conf] };
        let _: () = unsafe { msg_send![view as *mut AnyObject, setImage: &*configured_image] };
    }

    if let Some((r, g, b, a)) = config.tint_color {
        let color: *mut AnyObject = unsafe {
            msg_send![
                objc2::class!(NSColor),
                colorWithRed: r,
                green: g,
                blue: b,
                alpha: a
            ]
        };
        if !color.is_null() {
            let _: () = unsafe { msg_send![view as *mut AnyObject, setContentTintColor: color] };
        }
    }
}
