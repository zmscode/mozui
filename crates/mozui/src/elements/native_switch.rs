use super::native_element_helpers::schedule_native_callback;
use crate::Refineable as _;
use crate::platform::native_controls::{NativeControlState, SwitchConfig};
use crate::{
    AbsoluteLength, App, Bounds, DefiniteLength, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Length, Pixels, Style, StyleRefinement, Styled,
    Window, px,
};
use std::rc::Rc;

/// Event emitted when a native switch toggles.
#[derive(Clone, Debug)]
pub struct SwitchChangeEvent {
    /// The new checked state.
    pub checked: bool,
}

/// Create a platform-native switch element.
pub fn native_switch(id: impl Into<ElementId>) -> NativeSwitch {
    NativeSwitch {
        id: id.into(),
        checked: false,
        disabled: false,
        on_change: None,
        style: StyleRefinement::default(),
    }
}

/// A semantic wrapper around a platform-native switch.
pub struct NativeSwitch {
    id: ElementId,
    checked: bool,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(&SwitchChangeEvent, &mut Window, &mut App) + 'static>>,
    style: StyleRefinement,
}

impl NativeSwitch {
    /// Set the current checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Disable the switch.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Register a change listener.
    pub fn on_change(
        mut self,
        listener: impl Fn(&SwitchChangeEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(listener));
        self
    }
}

impl IntoElement for NativeSwitch {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeSwitch {
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
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(40.0))));
        }
        if matches!(style.size.height, Length::Auto) {
            style.size.height =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(22.0))));
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
        let checked = self.checked;
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
                    |checked| SwitchChangeEvent { checked },
                    dispatcher.clone(),
                )
            });

            native_controls.update_switch(
                &mut state,
                parent,
                bounds,
                window.scale_factor(),
                SwitchConfig {
                    checked,
                    enabled: !disabled,
                    on_change: on_change_fn,
                },
            );

            ((), Some(state))
        });
    }
}

impl Styled for NativeSwitch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
