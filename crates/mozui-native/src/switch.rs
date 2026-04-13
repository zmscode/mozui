use std::cell::RefCell;
use std::rc::Rc;

use mozui::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Size, Style, Window,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2::{AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSControl, NSControlStateValueOff, NSControlStateValueOn, NSSwitch};
#[cfg(target_os = "macos")]
use objc2_foundation::NSInteger;
use objc2_foundation::{NSObject, NSObjectProtocol};
#[cfg(target_os = "ios")]
use objc2_ui_kit::{UIControl, UIControlEvents, UISwitch};

use crate::native_view::NativeViewState;
#[cfg(target_os = "macos")]
use crate::native_view::parent_ns_view;
#[cfg(target_os = "ios")]
use crate::native_view::parent_ui_view;

#[cfg(target_os = "ios")]
type PlatformSwitch = UISwitch;
#[cfg(target_os = "macos")]
type PlatformSwitch = NSSwitch;

type ToggleCallback = Rc<RefCell<Option<Box<dyn Fn(bool)>>>>;

struct SwitchTargetIvars {
    callback: ToggleCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = SwitchTargetIvars]
    #[name = "MozuiSwitchTarget"]
    struct SwitchTarget;

    impl SwitchTarget {
        #[unsafe(method(switchToggled:))]
        fn __switch_toggled(&self, sender: &AnyObject) {
            #[cfg(target_os = "ios")]
            let is_on: bool = unsafe { msg_send![sender, isOn] };
            #[cfg(target_os = "macos")]
            let state: NSInteger = unsafe { msg_send![sender, state] };
            #[cfg(target_os = "macos")]
            let is_on = state == NSControlStateValueOn;
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                f(is_on);
            }
        }
    }

    unsafe impl NSObjectProtocol for SwitchTarget {}
);

impl SwitchTarget {
    fn new(callback: ToggleCallback, _mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(SwitchTargetIvars { callback });
        unsafe { msg_send![super(this), init] }
    }
}

/// A native macOS `NSSwitch` toggle element.
pub struct NativeSwitch {
    id: ElementId,
    is_on: bool,
    enabled: bool,
    on_toggle: Option<Box<dyn Fn(bool)>>,
}

impl NativeSwitch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            is_on: false,
            enabled: true,
            on_toggle: None,
        }
    }

    pub fn is_on(mut self, on: bool) -> Self {
        self.is_on = on;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn on_toggle(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_toggle = Some(Box::new(callback));
        self
    }
}

struct NativeSwitchPersistentState {
    view_state: NativeViewState,
    _target: Retained<SwitchTarget>,
}

fn platform_switch(view_state: &NativeViewState) -> &PlatformSwitch {
    unsafe { &*(view_state.view() as *const _ as *const PlatformSwitch) }
}

impl IntoElement for NativeSwitch {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeSwitch {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            flex_shrink: 1.,
            ..Default::default()
        };
        let layout_id = window.request_layout(style, [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let global_id = id.unwrap();
        let is_on = self.is_on;
        let enabled = self.enabled;
        let on_toggle = self.on_toggle.take();

        window.with_element_state(
            global_id,
            |state: Option<NativeSwitchPersistentState>, window| {
                #[cfg(target_os = "ios")]
                let parent = parent_ui_view(window);
                #[cfg(target_os = "macos")]
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };
                    let switch = PlatformSwitch::new(mtm);

                    #[cfg(target_os = "ios")]
                    switch.setOn(is_on);
                    #[cfg(target_os = "macos")]
                    switch.setState(if is_on {
                        NSControlStateValueOn
                    } else {
                        NSControlStateValueOff
                    });
                    switch.setEnabled(enabled);

                    let callback: ToggleCallback = Rc::new(RefCell::new(on_toggle));
                    let target = SwitchTarget::new(callback, mtm);

                    #[cfg(target_os = "ios")]
                    unsafe {
                        let target_obj: &AnyObject = &target;
                        UIControl::addTarget_action_forControlEvents(
                            &switch,
                            Some(target_obj),
                            sel!(switchToggled:),
                            UIControlEvents::ValueChanged,
                        );
                    }
                    #[cfg(target_os = "macos")]
                    unsafe {
                        let target_obj: &AnyObject = &target;
                        NSControl::setTarget(&switch, Some(target_obj));
                        NSControl::setAction(&switch, Some(sel!(switchToggled:)));
                    }

                    let view_state = NativeViewState::new(unsafe {
                        objc2::rc::Retained::cast_unchecked(switch)
                    });

                    NativeSwitchPersistentState {
                        view_state,
                        _target: target,
                    }
                });

                #[cfg(target_os = "ios")]
                platform_switch(&state.view_state).setOn_animated(is_on, false);
                #[cfg(target_os = "macos")]
                platform_switch(&state.view_state).setState(if is_on {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
                platform_switch(&state.view_state).setEnabled(enabled);
                #[cfg(target_os = "ios")]
                let hitbox = if let Some(parent) = parent {
                    state.view_state.attach_and_position(parent, bounds);
                    Some(window.insert_hitbox(bounds, HitboxBehavior::Normal))
                } else {
                    None
                };
                #[cfg(target_os = "macos")]
                let hitbox = {
                    state.view_state.attach_and_position(parent, bounds);
                    Some(window.insert_hitbox(bounds, HitboxBehavior::Normal))
                };
                (hitbox, state)
            },
        )
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let bounds = hitbox.as_ref().map(|h| h.bounds).unwrap_or(bounds);
        window.with_content_mask(Some(ContentMask { bounds }), |_window| {});
    }
}
