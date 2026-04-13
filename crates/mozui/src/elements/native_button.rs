use super::native_element_helpers::schedule_native_callback_no_args;
use crate::Refineable as _;
use crate::platform::native_controls::{ButtonConfig, ButtonStyle, NativeControlState};
use crate::{
    AbsoluteLength, App, Bounds, ClickEvent, DefiniteLength, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Length, Pixels, SharedString, Style,
    StyleRefinement, Styled, Window, px,
};
use std::rc::Rc;

/// Native button styling shared by platform backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeButtonStyle {
    /// Standard rounded push button.
    #[default]
    Rounded,
    /// Filled / accented button.
    Filled,
    /// Inline toolbar-style button.
    Inline,
    /// Borderless icon button.
    Borderless,
}

impl From<NativeButtonStyle> for ButtonStyle {
    fn from(value: NativeButtonStyle) -> Self {
        match value {
            NativeButtonStyle::Rounded => ButtonStyle::Rounded,
            NativeButtonStyle::Filled => ButtonStyle::Filled,
            NativeButtonStyle::Inline => ButtonStyle::Inline,
            NativeButtonStyle::Borderless => ButtonStyle::Borderless,
        }
    }
}

/// Create a platform-native button element.
pub fn native_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> NativeButton {
    NativeButton {
        id: id.into(),
        label: label.into(),
        on_click: None,
        style: StyleRefinement::default(),
        button_style: NativeButtonStyle::default(),
        disabled: false,
    }
}

/// A semantic wrapper around a platform-native push button.
pub struct NativeButton {
    id: ElementId,
    label: SharedString,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    style: StyleRefinement,
    button_style: NativeButtonStyle,
    disabled: bool,
}

impl NativeButton {
    /// Register a click listener.
    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(listener));
        self
    }

    /// Override the platform button style.
    pub fn button_style(mut self, style: NativeButtonStyle) -> Self {
        self.button_style = style;
        self
    }

    /// Disable the button.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl IntoElement for NativeButton {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeButton {
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
            let width = (self.label.len() as f32 * 8.0 + 24.0).max(80.0);
            style.size.width =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(width))));
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
        let label = self.label.clone();
        let disabled = self.disabled;
        let button_style = self.button_style;
        let on_click = self.on_click.clone();

        window.with_optional_element_state::<NativeControlState, _>(id, |prev_state, window| {
            let mut state = prev_state.flatten().unwrap_or_default();
            let parent = window.raw_native_view_ptr();
            let native_controls = window
                .native_controls()
                .expect("native controls availability checked before paint");

            let on_click_fn = on_click.map(|handler| {
                schedule_native_callback_no_args(handler, ClickEvent::default, dispatcher.clone())
            });

            native_controls.update_button(
                &mut state,
                parent,
                bounds,
                window.scale_factor(),
                ButtonConfig {
                    title: &label,
                    enabled: !disabled,
                    style: button_style.into(),
                    on_click: on_click_fn,
                },
            );

            ((), Some(state))
        });
    }
}

impl Styled for NativeButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
