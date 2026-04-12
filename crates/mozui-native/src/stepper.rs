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
use objc2_app_kit::{NSControl, NSStepper};
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::native_view::{NativeViewState, parent_ns_view};

type StepperCallback = Rc<RefCell<Option<Box<dyn Fn(f64)>>>>;

struct StepperTargetIvars {
    callback: StepperCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = StepperTargetIvars]
    #[name = "MozuiStepperTarget"]
    struct StepperTarget;

    impl StepperTarget {
        #[unsafe(method(stepperChanged:))]
        fn __stepper_changed(&self, sender: &AnyObject) {
            let value: f64 = unsafe { msg_send![sender, doubleValue] };
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                f(value);
            }
        }
    }

    unsafe impl NSObjectProtocol for StepperTarget {}
);

impl StepperTarget {
    fn new(callback: StepperCallback, _mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(StepperTargetIvars { callback });
        unsafe { msg_send![super(this), init] }
    }
}

/// A native macOS `NSStepper` element.
///
/// Maps to SwiftUI's `Stepper`.
pub struct NativeStepper {
    id: ElementId,
    min: f64,
    max: f64,
    value: f64,
    increment: f64,
    on_change: Option<Box<dyn Fn(f64)>>,
}

impl NativeStepper {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            min: 0.0,
            max: 100.0,
            value: 0.0,
            increment: 1.0,
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

    pub fn increment(mut self, increment: f64) -> Self {
        self.increment = increment;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(f64) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }
}

struct NativeStepperPersistentState {
    view_state: NativeViewState,
    _target: Retained<StepperTarget>,
}

impl IntoElement for NativeStepper {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for NativeStepper {
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
        let increment = self.increment;
        let on_change = self.on_change.take();

        window.with_element_state(
            global_id,
            |state: Option<NativeStepperPersistentState>, window| {
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };

                    let stepper = {
                        let stepper = NSStepper::initWithFrame(
                            mtm.alloc(),
                            objc2_foundation::NSRect::ZERO,
                        );
                        stepper.setMinValue(min);
                        stepper.setMaxValue(max);
                        stepper.setDoubleValue(value);
                        stepper.setIncrement(increment);
                        stepper
                    };

                    let callback: StepperCallback = Rc::new(RefCell::new(on_change));
                    let target = StepperTarget::new(callback, mtm);

                    unsafe {
                        let target_obj: &AnyObject = &target;
                        NSControl::setTarget(&stepper, Some(target_obj));
                        NSControl::setAction(&stepper, Some(sel!(stepperChanged:)));
                    }

                    let view_state = NativeViewState::new(unsafe {
                        objc2::rc::Retained::cast_unchecked(stepper)
                    });

                    NativeStepperPersistentState {
                        view_state,
                        _target: target,
                    }
                });

                state.view_state.attach_and_position(parent, bounds);
                let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                (Some(hitbox), state)
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
