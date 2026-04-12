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
use objc2_app_kit::{NSControl, NSPopUpButton};
use objc2_foundation::{NSInteger, NSObject, NSObjectProtocol, NSString};

use crate::native_view::{NativeViewState, parent_ns_view};

type SelectCallback = Rc<RefCell<Option<Box<dyn Fn(usize)>>>>;

struct PickerTargetIvars {
    callback: SelectCallback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = PickerTargetIvars]
    #[name = "MozuiPickerTarget"]
    struct PickerTarget;

    impl PickerTarget {
        #[unsafe(method(selectionChanged:))]
        fn __selection_changed(&self, sender: &AnyObject) {
            let index: NSInteger = unsafe { msg_send![sender, indexOfSelectedItem] };
            let cb = self.ivars().callback.borrow();
            if let Some(ref f) = *cb {
                f(index as usize);
            }
        }
    }

    unsafe impl NSObjectProtocol for PickerTarget {}
);

impl PickerTarget {
    fn new(callback: SelectCallback, _mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(PickerTargetIvars { callback });
        unsafe { msg_send![super(this), init] }
    }
}

/// A native macOS dropdown/popup button rendered as a mozui element.
///
/// Maps to SwiftUI's `Picker` with `.menu` style.
pub struct NativePicker {
    id: ElementId,
    items: Vec<String>,
    selected: usize,
    on_change: Option<Box<dyn Fn(usize)>>,
}

impl NativePicker {
    pub fn new(id: impl Into<ElementId>, items: Vec<String>) -> Self {
        Self {
            id: id.into(),
            items,
            selected: 0,
            on_change: None,
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }
}

struct NativePickerPersistentState {
    view_state: NativeViewState,
    _target: Retained<PickerTarget>,
}

impl IntoElement for NativePicker {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for NativePicker {
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
        let items = self.items.clone();
        let selected = self.selected;
        let on_change = self.on_change.take();

        window.with_element_state(
            global_id,
            |state: Option<NativePickerPersistentState>, window| {
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };

                    let popup = NSPopUpButton::initWithFrame_pullsDown(
                        mtm.alloc(),
                        objc2_foundation::NSRect::ZERO,
                        false,
                    );

                    for item_title in &items {
                        let ns_title = NSString::from_str(item_title);
                        popup.addItemWithTitle(&ns_title);
                    }

                    if selected < items.len() {
                        popup.selectItemAtIndex(selected as NSInteger);
                    }

                    let callback: SelectCallback = Rc::new(RefCell::new(on_change));
                    let target = PickerTarget::new(callback, mtm);

                    unsafe {
                        let target_obj: &AnyObject = &target;
                        NSControl::setTarget(&popup, Some(target_obj));
                        NSControl::setAction(&popup, Some(sel!(selectionChanged:)));
                    }

                    let view_state =
                        NativeViewState::new(unsafe { objc2::rc::Retained::cast_unchecked(popup) });

                    NativePickerPersistentState {
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
