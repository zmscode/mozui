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
use objc2_app_kit::{NSControl, NSSlider};
use objc2_foundation::{NSObject, NSObjectProtocol};
#[cfg(target_os = "ios")]
use objc2_ui_kit::{UIControl, UIControlEvents, UISlider};

use crate::native_view::NativeViewState;
#[cfg(target_os = "macos")]
use crate::native_view::parent_ns_view;
#[cfg(target_os = "ios")]
use crate::native_view::parent_ui_view;

type SliderCallback = Rc<RefCell<Option<Box<dyn Fn(f64)>>>>;

struct SliderTargetIvars {
    callback: SliderCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = SliderTargetIvars]
    #[name = "MozuiSliderTarget"]
    struct SliderTarget;

    impl SliderTarget {
        #[unsafe(method(sliderChanged:))]
        fn __slider_changed(&self, sender: &AnyObject) {
            #[cfg(target_os = "ios")]
            let value: f32 = unsafe { msg_send![sender, value] };
            #[cfg(target_os = "ios")]
            let value: f64 = value as f64;
            #[cfg(target_os = "macos")]
            let value: f64 = unsafe { msg_send![sender, doubleValue] };
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                f(value);
            }
        }
    }

    unsafe impl NSObjectProtocol for SliderTarget {}
);

impl SliderTarget {
    fn new(callback: SliderCallback, _mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(SliderTargetIvars { callback });
        unsafe { msg_send![super(this), init] }
    }
}

/// A native macOS `NSSlider` element.
///
/// Maps to SwiftUI's `Slider`.
pub struct NativeSlider {
    id: ElementId,
    min: f64,
    max: f64,
    value: f64,
    on_change: Option<Box<dyn Fn(f64)>>,
}

impl NativeSlider {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            min: 0.0,
            max: 1.0,
            value: 0.0,
            on_change: None,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(f64) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }
}

struct NativeSliderPersistentState {
    view_state: NativeViewState,
    _target: Retained<SliderTarget>,
}

impl IntoElement for NativeSlider {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for NativeSlider {
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
        let min = self.min;
        let max = self.max;
        let value = self.value;
        let on_change = self.on_change.take();

        window.with_element_state(
            global_id,
            |state: Option<NativeSliderPersistentState>, window| {
                #[cfg(target_os = "ios")]
                let parent = parent_ui_view(window);
                #[cfg(target_os = "macos")]
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };

                    #[cfg(target_os = "ios")]
                    let slider = {
                        let slider = UISlider::new(mtm);
                        slider.setMinimumValue(min as f32);
                        slider.setMaximumValue(max as f32);
                        slider.setValue(value as f32);
                        slider
                    };
                    #[cfg(target_os = "macos")]
                    let slider = {
                        let slider =
                            NSSlider::initWithFrame(mtm.alloc(), objc2_foundation::NSRect::ZERO);
                        slider.setMinValue(min);
                        slider.setMaxValue(max);
                        slider.setDoubleValue(value);
                        slider
                    };

                    let callback: SliderCallback = Rc::new(RefCell::new(on_change));
                    let target = SliderTarget::new(callback, mtm);

                    #[cfg(target_os = "ios")]
                    unsafe {
                        let target_obj: &AnyObject = &target;
                        UIControl::addTarget_action_forControlEvents(
                            &slider,
                            Some(target_obj),
                            sel!(sliderChanged:),
                            UIControlEvents::ValueChanged,
                        );
                    }
                    #[cfg(target_os = "macos")]
                    unsafe {
                        let target_obj: &AnyObject = &target;
                        NSControl::setTarget(&slider, Some(target_obj));
                        NSControl::setAction(&slider, Some(sel!(sliderChanged:)));
                    }

                    let view_state = NativeViewState::new(unsafe {
                        objc2::rc::Retained::cast_unchecked(slider)
                    });

                    NativeSliderPersistentState {
                        view_state,
                        _target: target,
                    }
                });

                #[cfg(target_os = "ios")]
                {
                    let platform_slider: &UISlider =
                        unsafe { &*(state.view_state.view() as *const _ as *const UISlider) };
                    platform_slider.setMinimumValue(min as f32);
                    platform_slider.setMaximumValue(max as f32);
                    platform_slider.setValue(value as f32);
                }
                #[cfg(target_os = "macos")]
                {
                    let platform_slider: &NSSlider =
                        unsafe { &*(state.view_state.view() as *const _ as *const NSSlider) };
                    platform_slider.setMinValue(min);
                    platform_slider.setMaxValue(max);
                    platform_slider.setDoubleValue(value);
                }

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
