use super::native_element_helpers::schedule_native_callback;
use crate::Refineable as _;
use crate::platform::native_controls::{NativeControlState, SliderConfig};
use crate::{
    AbsoluteLength, App, Bounds, DefiniteLength, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Length, Pixels, Style, StyleRefinement, Styled,
    Window, px,
};
use std::rc::Rc;

/// Event emitted when a native slider changes.
#[derive(Clone, Debug)]
pub struct SliderChangeEvent {
    /// The new slider value.
    pub value: f64,
}

/// Create a platform-native slider element.
pub fn native_slider(id: impl Into<ElementId>) -> NativeSlider {
    NativeSlider {
        id: id.into(),
        min: 0.0,
        max: 1.0,
        value: 0.0,
        disabled: false,
        on_change: None,
        style: StyleRefinement::default(),
    }
}

/// A semantic wrapper around a platform-native slider.
pub struct NativeSlider {
    id: ElementId,
    min: f64,
    max: f64,
    value: f64,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(&SliderChangeEvent, &mut Window, &mut App) + 'static>>,
    style: StyleRefinement,
}

impl NativeSlider {
    /// Set the slider range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Set the current slider value.
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Disable the slider.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Register a change listener.
    pub fn on_change(
        mut self,
        listener: impl Fn(&SliderChangeEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(listener));
        self
    }
}

impl IntoElement for NativeSlider {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeSlider {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);

        if matches!(style.size.width, Length::Auto) {
            style.size.width =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(140.0))));
        }
        if matches!(style.size.height, Length::Auto) {
            style.size.height =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(24.0))));
        }

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        bounds
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        if id.is_none() {
            debug_assert!(false, "native controls require a stable element id");
            return;
        }
        if window.raw_native_view_ptr().is_null() || window.native_controls().is_none() {
            return;
        }

        let dispatcher = window.native_callback_dispatcher();
        let min = self.min;
        let max = self.max;
        let value = self.value;
        let disabled = self.disabled;
        let on_change = self.on_change.clone();

        window.with_optional_element_state::<NativeControlState, _>(id, |prev_state, window| {
            let mut state = prev_state.flatten().unwrap_or_default();
            let parent = window.raw_native_view_ptr();
            let native_controls = window
                .native_controls()
                .expect("native controls availability checked before paint");

            let on_change_fn = on_change.map(|handler| {
                schedule_native_callback(
                    handler,
                    |value| SliderChangeEvent { value },
                    dispatcher.clone(),
                )
            });

            native_controls.update_slider(
                &mut state,
                parent,
                bounds,
                window.scale_factor(),
                SliderConfig {
                    min,
                    max,
                    value,
                    enabled: !disabled,
                    on_change: on_change_fn,
                },
            );

            ((), Some(state))
        });
    }
}

impl Styled for NativeSlider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
