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
use objc2_app_kit::{NSColor, NSColorWell, NSControl};
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::native_view::{NativeViewState, parent_ns_view};

/// RGBA color representation for the color picker.
#[derive(Debug, Clone, Copy)]
pub struct NativeColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl NativeColor {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}

type ColorCallback = Rc<RefCell<Option<Box<dyn Fn(NativeColor)>>>>;

struct ColorTargetIvars {
    callback: ColorCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = ColorTargetIvars]
    #[name = "MozuiColorTarget"]
    struct ColorTarget;

    impl ColorTarget {
        #[unsafe(method(colorChanged:))]
        fn __color_changed(&self, sender: &AnyObject) {
            let color: *const AnyObject = unsafe { msg_send![sender, color] };
            let mut r: f64 = 0.0;
            let mut g: f64 = 0.0;
            let mut b: f64 = 0.0;
            let mut a: f64 = 0.0;
            unsafe {
                let _: () = msg_send![color, getRed: &mut r, green: &mut g, blue: &mut b, alpha: &mut a];
            }
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                f(NativeColor::new(r, g, b, a));
            }
        }
    }

    unsafe impl NSObjectProtocol for ColorTarget {}
);

impl ColorTarget {
    fn new(callback: ColorCallback, _mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ColorTargetIvars { callback });
        unsafe { msg_send![super(this), init] }
    }
}

/// A native macOS `NSColorWell` element.
///
/// Maps to SwiftUI's `ColorPicker`.
pub struct NativeColorPicker {
    id: ElementId,
    color: NativeColor,
    on_change: Option<Box<dyn Fn(NativeColor)>>,
}

impl NativeColorPicker {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            color: NativeColor::white(),
            on_change: None,
        }
    }

    pub fn color(mut self, color: NativeColor) -> Self {
        self.color = color;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(NativeColor) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }
}

struct NativeColorPickerPersistentState {
    view_state: NativeViewState,
    _target: Retained<ColorTarget>,
}

impl IntoElement for NativeColorPicker {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for NativeColorPicker {
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
        let color = self.color;
        let on_change = self.on_change.take();

        window.with_element_state(
            global_id,
            |state: Option<NativeColorPickerPersistentState>, window| {
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };

                    let well = unsafe {
                        let well =
                            NSColorWell::initWithFrame(mtm.alloc(), objc2_foundation::NSRect::ZERO);
                        let ns_color: Retained<NSColor> = msg_send![
                            NSColor::class(),
                            colorWithRed: color.r,
                            green: color.g,
                            blue: color.b,
                            alpha: color.a
                        ];
                        well.setColor(&ns_color);
                        well
                    };

                    let callback: ColorCallback = Rc::new(RefCell::new(on_change));
                    let target = ColorTarget::new(callback, mtm);

                    unsafe {
                        let target_obj: &AnyObject = &target;
                        NSControl::setTarget(&well, Some(target_obj));
                        NSControl::setAction(&well, Some(sel!(colorChanged:)));
                    }

                    let view_state =
                        NativeViewState::new(unsafe { objc2::rc::Retained::cast_unchecked(well) });

                    NativeColorPickerPersistentState {
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
