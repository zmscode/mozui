use mozui::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Size, Style, Window,
};
use objc2::rc::Retained;
use objc2::runtime::AnyClass;
use objc2::{MainThreadMarker, msg_send};
use objc2_app_kit::{
    NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
    NSVisualEffectView,
};

use crate::native_view::{NativeViewState, parent_ns_view};

/// The style of glass effect to apply (macOS 26+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlassEffectStyle {
    /// Standard glass effect (NSGlassEffectViewStyleRegular = 0).
    #[default]
    Regular,
    /// Clear glass effect with less blur (NSGlassEffectViewStyleClear = 1).
    Clear,
}

/// A native glass effect element that uses `NSGlassEffectView` on macOS 26+
/// and falls back to `NSVisualEffectView` on older versions.
pub struct NativeGlassEffect {
    id: ElementId,
    style: GlassEffectStyle,
    corner_radius: Option<f64>,
}

impl NativeGlassEffect {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: GlassEffectStyle::default(),
            corner_radius: None,
        }
    }

    pub fn style(mut self, style: GlassEffectStyle) -> Self {
        self.style = style;
        self
    }

    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = Some(radius);
        self
    }
}

fn glass_effect_class() -> Option<&'static AnyClass> {
    AnyClass::get(c"NSGlassEffectView")
}

fn create_glass_view(
    style: GlassEffectStyle,
    corner_radius: Option<f64>,
    mtm: MainThreadMarker,
) -> Retained<NSView> {
    if let Some(cls) = glass_effect_class() {
        unsafe {
            let view: Retained<NSView> = msg_send![cls, new];

            let style_value: isize = match style {
                GlassEffectStyle::Regular => 0,
                GlassEffectStyle::Clear => 1,
            };
            let _: () = msg_send![&view, setStyle: style_value];

            if let Some(radius) = corner_radius {
                let _: () = msg_send![&view, setCornerRadius: radius];
            }

            view
        }
    } else {
        let effect_view = NSVisualEffectView::new(mtm);
        effect_view.setMaterial(NSVisualEffectMaterial::HUDWindow);
        effect_view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        effect_view.setState(NSVisualEffectState::Active);

        unsafe { objc2::rc::Retained::cast_unchecked(effect_view) }
    }
}

impl IntoElement for NativeGlassEffect {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeGlassEffect {
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
        let glass_style = self.style;
        let corner_radius = self.corner_radius;

        window.with_element_state(global_id, |state: Option<NativeViewState>, window| {
            let parent = parent_ns_view(window);

            let mut state = state.unwrap_or_else(|| {
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let view = create_glass_view(glass_style, corner_radius, mtm);
                NativeViewState::new(view)
            });

            state.attach_and_position(parent, bounds);
            let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
            (Some(hitbox), state)
        })
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
