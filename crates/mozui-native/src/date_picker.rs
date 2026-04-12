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
use objc2_app_kit::{NSControl, NSDatePicker, NSDatePickerElementFlags, NSDatePickerStyle};
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::native_view::{NativeViewState, parent_ns_view};

/// Date picker display style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DatePickerDisplayStyle {
    /// Text field with stepper.
    #[default]
    TextFieldAndStepper,
    /// Clock and calendar.
    ClockAndCalendar,
    /// Text field only.
    TextField,
}

/// Which date/time elements to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePickerElements {
    /// Year, month, day.
    DateOnly,
    /// Hour, minute, second.
    TimeOnly,
    /// Both date and time.
    DateAndTime,
}

impl Default for DatePickerElements {
    fn default() -> Self {
        Self::DateAndTime
    }
}

type DateChangeCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

struct DatePickerTargetIvars {
    callback: DateChangeCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = DatePickerTargetIvars]
    #[name = "MozuiDatePickerTarget"]
    struct DatePickerTargetObj;

    impl DatePickerTargetObj {
        #[unsafe(method(dateChanged:))]
        fn __date_changed(&self, _sender: &AnyObject) {
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                f();
            }
        }
    }

    unsafe impl NSObjectProtocol for DatePickerTargetObj {}
);

impl DatePickerTargetObj {
    fn new(callback: DateChangeCallback, _mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(DatePickerTargetIvars { callback });
        unsafe { msg_send![super(this), init] }
    }
}

/// A native macOS `NSDatePicker` element.
///
/// Maps to SwiftUI's `DatePicker`.
pub struct NativeDatePicker {
    id: ElementId,
    display_style: DatePickerDisplayStyle,
    elements: DatePickerElements,
    on_change: Option<Box<dyn Fn()>>,
}

impl NativeDatePicker {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            display_style: DatePickerDisplayStyle::default(),
            elements: DatePickerElements::default(),
            on_change: None,
        }
    }

    pub fn display_style(mut self, style: DatePickerDisplayStyle) -> Self {
        self.display_style = style;
        self
    }

    pub fn elements(mut self, elements: DatePickerElements) -> Self {
        self.elements = elements;
        self
    }

    pub fn on_change(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }
}

struct NativeDatePickerPersistentState {
    view_state: NativeViewState,
    _target: Retained<DatePickerTargetObj>,
}

impl IntoElement for NativeDatePicker {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for NativeDatePicker {
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
        let display_style = self.display_style;
        let elements = self.elements;
        let on_change = self.on_change.take();

        window.with_element_state(
            global_id,
            |state: Option<NativeDatePickerPersistentState>, window| {
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };

                    let picker = {
                        let picker = NSDatePicker::initWithFrame(
                            mtm.alloc(),
                            objc2_foundation::NSRect::ZERO,
                        );

                        let ns_style = match display_style {
                            DatePickerDisplayStyle::TextFieldAndStepper => {
                                NSDatePickerStyle::TextFieldAndStepper
                            }
                            DatePickerDisplayStyle::ClockAndCalendar => {
                                NSDatePickerStyle::ClockAndCalendar
                            }
                            DatePickerDisplayStyle::TextField => NSDatePickerStyle::TextField,
                        };
                        picker.setDatePickerStyle(ns_style);

                        let ns_elements = match elements {
                            DatePickerElements::DateOnly => NSDatePickerElementFlags::YearMonthDay,
                            DatePickerElements::TimeOnly => {
                                NSDatePickerElementFlags::HourMinuteSecond
                            }
                            DatePickerElements::DateAndTime => NSDatePickerElementFlags(
                                NSDatePickerElementFlags::YearMonthDay.0
                                    | NSDatePickerElementFlags::HourMinuteSecond.0,
                            ),
                        };
                        picker.setDatePickerElements(ns_elements);

                        picker
                    };

                    let callback: DateChangeCallback = Rc::new(RefCell::new(on_change));
                    let target = DatePickerTargetObj::new(callback, mtm);

                    unsafe {
                        let target_obj: &AnyObject = &target;
                        NSControl::setTarget(&picker, Some(target_obj));
                        NSControl::setAction(&picker, Some(sel!(dateChanged:)));
                    }

                    let view_state = NativeViewState::new(unsafe {
                        objc2::rc::Retained::cast_unchecked(picker)
                    });

                    NativeDatePickerPersistentState {
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
