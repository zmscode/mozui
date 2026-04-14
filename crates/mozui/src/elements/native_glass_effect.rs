use crate::Refineable as _;
use crate::platform::native_controls::{GlassEffectConfig, GlassEffectStyle, NativeControlState};
use crate::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Style, StyleRefinement, Styled, Window,
};

/// Platform-native glass effect view.
///
/// Uses `NSGlassEffectView` on macOS 26+ and falls back to `NSVisualEffectView` on older systems.
pub fn native_glass_effect(id: impl Into<ElementId>) -> NativeGlassEffect {
    NativeGlassEffect::new(id)
}

/// Platform-native glass effect view backed by `NSGlassEffectView` on macOS 26+.
pub struct NativeGlassEffect {
    id: ElementId,
    layout_style: StyleRefinement,
    glass_style: GlassEffectStyle,
    corner_radius: Option<f64>,
    tint_color: Option<(f64, f64, f64, f64)>,
}

impl NativeGlassEffect {
    /// Create a new `NativeGlassEffect` with the given stable element id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            layout_style: StyleRefinement::default(),
            glass_style: GlassEffectStyle::default(),
            corner_radius: None,
            tint_color: None,
        }
    }

    /// Set the glass style variant.
    pub fn style(mut self, style: GlassEffectStyle) -> Self {
        self.glass_style = style;
        self
    }

    /// Override the corner radius.
    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = Some(radius);
        self
    }

    /// RGBA tint color, each component 0.0–1.0.
    pub fn tint_color(mut self, r: f64, g: f64, b: f64, a: f64) -> Self {
        self.tint_color = Some((r, g, b, a));
        self
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
        let mut style = Style { flex_shrink: 1., ..Default::default() };
        style.refine(&self.layout_style);
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        if id.is_none() || window.raw_native_view_ptr().is_null() {
            return None;
        }
        if window.native_controls().is_none() {
            return None;
        }

        let style = self.glass_style;
        let corner_radius = self.corner_radius;
        let tint_color = self.tint_color;
        let parent = window.raw_native_view_ptr();

        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

        window.with_optional_element_state::<NativeControlState, _>(id, |prev, window| {
            let mut state = prev.flatten().unwrap_or_default();
            let nc = window.native_controls().expect("checked above");
            nc.update_glass_effect(
                &mut state,
                parent,
                bounds,
                window.scale_factor(),
                GlassEffectConfig { style, corner_radius, tint_color },
            );
            (Some(hitbox.clone()), Some(state))
        });

        Some(hitbox)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let bounds = hitbox.as_ref().map(|h| h.bounds).unwrap_or(bounds);
        window.with_content_mask(Some(ContentMask { bounds }), |_w| {});
    }
}

impl Styled for NativeGlassEffect {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.layout_style
    }
}
