use std::cell::RefCell;
use std::rc::Rc;

use mozui::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Size, Style, Window,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2::{AnyThread, ClassType, DefinedClass, MainThreadMarker, define_class, msg_send};
use objc2_app_kit::{NSBezelStyle, NSButton, NSControl};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};

use crate::native_view::{NativeViewState, parent_ns_view};

type ActionCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

struct ActionTargetIvars {
    callback: ActionCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = ActionTargetIvars]
    #[name = "MozuiActionTarget"]
    struct ActionTarget;

    impl ActionTarget {
        #[unsafe(method(performAction:))]
        fn __perform_action(&self, _sender: &AnyObject) {
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                f();
            }
        }
    }

    unsafe impl NSObjectProtocol for ActionTarget {}
);

impl ActionTarget {
    fn new(callback: ActionCallback, _mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ActionTargetIvars { callback });
        unsafe { msg_send![super(this), init] }
    }
}

/// The visual style of a [`NativeButton`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeButtonStyle {
    #[default]
    Push,
    FlexiblePush,
    Toolbar,
    AccessoryBar,
}

impl NativeButtonStyle {
    fn to_ns(self) -> NSBezelStyle {
        match self {
            Self::Push => NSBezelStyle::Push,
            Self::FlexiblePush => NSBezelStyle::FlexiblePush,
            Self::Toolbar => NSBezelStyle::Toolbar,
            Self::AccessoryBar => NSBezelStyle::AccessoryBarAction,
        }
    }
}

/// A native macOS `NSButton` element.
pub struct NativeButton {
    id: ElementId,
    title: String,
    style: NativeButtonStyle,
    enabled: bool,
    symbol: Option<String>,
    key_equivalent: Option<String>,
    on_click: Option<Box<dyn Fn()>>,
}

impl NativeButton {
    pub fn new(id: impl Into<ElementId>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            style: NativeButtonStyle::default(),
            enabled: true,
            symbol: None,
            key_equivalent: None,
            on_click: None,
        }
    }

    pub fn style(mut self, style: NativeButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set an SF Symbol image on the button.
    pub fn symbol(mut self, name: impl Into<String>) -> Self {
        self.symbol = Some(name.into());
        self
    }

    /// Set a keyboard shortcut (e.g., "\r" for Return, "q" for Cmd+Q).
    pub fn key_equivalent(mut self, key: impl Into<String>) -> Self {
        self.key_equivalent = Some(key.into());
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(callback));
        self
    }
}

/// Persistent state for a NativeButton across frames.
struct NativeButtonPersistentState {
    view_state: NativeViewState,
    _action_target: Retained<ActionTarget>,
}

impl IntoElement for NativeButton {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeButton {
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
        let title = self.title.clone();
        let button_style = self.style;
        let enabled = self.enabled;
        let symbol = self.symbol.clone();
        let key_equivalent = self.key_equivalent.clone();
        let on_click = self.on_click.take();

        window.with_element_state(
            global_id,
            |state: Option<NativeButtonPersistentState>, window| {
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };
                    let ns_title = NSString::from_str(&title);
                    let button = unsafe {
                        NSButton::buttonWithTitle_target_action(&ns_title, None, None, mtm)
                    };
                    button.setBezelStyle(button_style.to_ns());
                    button.setEnabled(enabled);

                    if let Some(ref sym) = symbol {
                        let ns_sym = NSString::from_str(sym);
                        unsafe {
                            let image: Option<Retained<objc2_app_kit::NSImage>> = msg_send![
                                objc2_app_kit::NSImage::class(),
                                imageWithSystemSymbolName: &*ns_sym,
                                accessibilityDescription: std::ptr::null::<NSString>()
                            ];
                            if let Some(img) = image {
                                button.setImage(Some(&img));
                                // NSCellImagePosition::ImageLeading = 8
                                let _: () = msg_send![&button, setImagePosition: 8_isize];
                            }
                        }
                    }

                    if let Some(ref key) = key_equivalent {
                        let ns_key = NSString::from_str(key);
                        button.setKeyEquivalent(&ns_key);
                    }

                    let callback: ActionCallback = Rc::new(RefCell::new(on_click));
                    let action_target = ActionTarget::new(callback, mtm);

                    unsafe {
                        let target_obj: &AnyObject = &action_target;
                        NSControl::setTarget(&button, Some(target_obj));
                        NSControl::setAction(&button, Some(sel!(performAction:)));
                    }

                    let view_state = NativeViewState::new(unsafe {
                        objc2::rc::Retained::cast_unchecked(button)
                    });

                    NativeButtonPersistentState {
                        view_state,
                        _action_target: action_target,
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
