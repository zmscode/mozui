use super::native_element_helpers::schedule_native_callback;
use crate::Refineable as _;
use crate::platform::native_controls::{NativeControlState, TextFieldConfig, TextFieldStyle};
use crate::{
    AbsoluteLength, App, Bounds, DefiniteLength, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Length, Pixels, SharedString, Style,
    StyleRefinement, Styled, Window, px,
};
use std::rc::Rc;

/// Event emitted when a native text field changes.
#[derive(Clone, Debug)]
pub struct TextFieldChangeEvent {
    /// The current text.
    pub text: SharedString,
}

/// Event emitted when a native text field submits.
#[derive(Clone, Debug)]
pub struct TextFieldSubmitEvent {
    /// The current text.
    pub text: SharedString,
}

/// Native text field styling shared by platform backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeTextFieldStyle {
    /// Standard editable text field.
    #[default]
    Plain,
    /// Native search field appearance.
    Search,
}

impl From<NativeTextFieldStyle> for TextFieldStyle {
    fn from(value: NativeTextFieldStyle) -> Self {
        match value {
            NativeTextFieldStyle::Plain => TextFieldStyle::Plain,
            NativeTextFieldStyle::Search => TextFieldStyle::Search,
        }
    }
}

/// Create a platform-native text field element.
pub fn native_text_field(id: impl Into<ElementId>) -> NativeTextField {
    NativeTextField::new(id)
}

/// Create a platform-native search field element.
pub fn native_search_field(id: impl Into<ElementId>) -> NativeTextField {
    NativeTextField::new(id).field_style(NativeTextFieldStyle::Search)
}

/// A semantic wrapper around a platform-native text/search field.
pub struct NativeTextField {
    id: ElementId,
    placeholder: Option<SharedString>,
    value: SharedString,
    disabled: bool,
    editable: bool,
    selectable: bool,
    bezeled: bool,
    font_size: Option<f64>,
    secure: bool,
    field_style: NativeTextFieldStyle,
    on_change: Option<Rc<dyn Fn(&TextFieldChangeEvent, &mut Window, &mut App) + 'static>>,
    on_submit: Option<Rc<dyn Fn(&TextFieldSubmitEvent, &mut Window, &mut App) + 'static>>,
    style: StyleRefinement,
}

impl NativeTextField {
    /// Create a native text field.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            placeholder: None,
            value: SharedString::default(),
            disabled: false,
            editable: true,
            selectable: true,
            bezeled: true,
            font_size: None,
            secure: false,
            field_style: NativeTextFieldStyle::default(),
            on_change: None,
            on_submit: None,
            style: StyleRefinement::default(),
        }
    }

    /// Create a non-editable label-like text field.
    pub fn label(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self::new(id)
            .value(text)
            .editable(false)
            .selectable(false)
            .bezeled(false)
    }

    /// Set the placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the current text value.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Disable the field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Control whether the field is editable.
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Control whether the field allows text selection.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Control whether the field draws a bezel/border.
    pub fn bezeled(mut self, bezeled: bool) -> Self {
        self.bezeled = bezeled;
        self
    }

    /// Override the system font size.
    pub fn font_size(mut self, font_size: f64) -> Self {
        self.font_size = Some(font_size);
        self
    }

    /// Render a secure/password field.
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Override the platform field style.
    pub fn field_style(mut self, field_style: NativeTextFieldStyle) -> Self {
        self.field_style = field_style;
        self
    }

    /// Register a change listener.
    pub fn on_change(
        mut self,
        listener: impl Fn(&TextFieldChangeEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(listener));
        self
    }

    /// Register a submit listener.
    pub fn on_submit(
        mut self,
        listener: impl Fn(&TextFieldSubmitEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_submit = Some(Rc::new(listener));
        self
    }
}

impl IntoElement for NativeTextField {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeTextField {
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
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(180.0))));
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
        let placeholder = self.placeholder.clone();
        let value = self.value.clone();
        let disabled = self.disabled;
        let editable = self.editable;
        let selectable = self.selectable;
        let bezeled = self.bezeled;
        let font_size = self.font_size;
        let secure = self.secure;
        let field_style = self.field_style;
        let on_change = self.on_change.clone();
        let on_submit = self.on_submit.clone();

        window.with_optional_element_state::<NativeControlState, _>(id, |prev_state, window| {
            let mut state = prev_state.flatten().unwrap_or_default();
            let parent = window.raw_native_view_ptr();
            let native_controls = window
                .native_controls()
                .expect("native controls availability checked before paint");

            let on_change_fn = on_change.map(|handler| {
                schedule_native_callback(
                    handler,
                    |text| TextFieldChangeEvent {
                        text: SharedString::from(text),
                    },
                    dispatcher.clone(),
                )
            });
            let on_submit_fn = on_submit.map(|handler| {
                schedule_native_callback(
                    handler,
                    |text| TextFieldSubmitEvent {
                        text: SharedString::from(text),
                    },
                    dispatcher.clone(),
                )
            });

            native_controls.update_text_field(
                &mut state,
                parent,
                bounds,
                window.scale_factor(),
                TextFieldConfig {
                    value: value.as_ref(),
                    placeholder: placeholder.as_ref().map(|value| value.as_ref()),
                    enabled: !disabled,
                    editable,
                    selectable,
                    bezeled,
                    font_size,
                    secure,
                    style: field_style.into(),
                    on_change: on_change_fn,
                    on_submit: on_submit_fn,
                },
            );

            ((), Some(state))
        });
    }
}

impl Styled for NativeTextField {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
