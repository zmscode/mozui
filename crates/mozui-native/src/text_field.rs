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
use objc2_app_kit::{NSControl, NSTextField};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};

use crate::native_view::{NativeViewState, parent_ns_view};

type TextCallback = Rc<RefCell<Option<Box<dyn Fn(String)>>>>;

struct TextFieldTargetIvars {
    on_change: TextCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = TextFieldTargetIvars]
    #[name = "MozuiTextFieldTarget"]
    struct TextFieldTarget;

    impl TextFieldTarget {
        #[unsafe(method(textDidChange:))]
        fn __text_did_change(&self, sender: &AnyObject) {
            let value: Retained<NSString> = unsafe { msg_send![sender, stringValue] };
            let text = value.to_string();
            let cb = self.ivars().on_change.borrow();
            if let Some(ref f) = *cb {
                f(text);
            }
        }
    }

    unsafe impl NSObjectProtocol for TextFieldTarget {}
);

impl TextFieldTarget {
    fn new(on_change: TextCallback, _mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(TextFieldTargetIvars { on_change });
        unsafe { msg_send![super(this), init] }
    }
}

/// A native macOS `NSTextField` element.
pub struct NativeTextField {
    id: ElementId,
    placeholder: Option<String>,
    value: Option<String>,
    on_change: Option<Box<dyn Fn(String)>>,
}

impl NativeTextField {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            placeholder: None,
            value: None,
            on_change: None,
        }
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn value(mut self, text: impl Into<String>) -> Self {
        self.value = Some(text.into());
        self
    }

    pub fn on_change(mut self, callback: impl Fn(String) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }
}

struct NativeTextFieldPersistentState {
    view_state: NativeViewState,
    _target: Retained<TextFieldTarget>,
}

impl IntoElement for NativeTextField {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeTextField {
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
        let placeholder = self.placeholder.clone();
        let value = self.value.clone();
        let on_change = self.on_change.take();

        window.with_element_state(
            global_id,
            |state: Option<NativeTextFieldPersistentState>, window| {
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };
                    let text_field = NSTextField::new(mtm);

                    if let Some(ref placeholder) = placeholder {
                        let ns_placeholder = NSString::from_str(placeholder);
                        text_field.setPlaceholderString(Some(&ns_placeholder));
                    }

                    if let Some(ref value) = value {
                        let ns_value = NSString::from_str(value);
                        text_field.setStringValue(&ns_value);
                    }

                    let on_change_cb: TextCallback = Rc::new(RefCell::new(on_change));
                    let target = TextFieldTarget::new(on_change_cb, mtm);

                    unsafe {
                        let target_obj: &AnyObject = &target;
                        NSControl::setTarget(&text_field, Some(target_obj));
                        NSControl::setAction(&text_field, Some(sel!(textDidChange:)));
                    }

                    let view_state = NativeViewState::new(unsafe {
                        objc2::rc::Retained::cast_unchecked(text_field)
                    });

                    NativeTextFieldPersistentState {
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
