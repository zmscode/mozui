use crate::Refineable as _;
use crate::platform::native_controls::{ImageViewConfig, NativeControlState, SymbolScale, SymbolWeight};
use crate::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, SharedString, Style, StyleRefinement,
    Styled, Window,
};

/// Platform-native SF Symbol image view backed by `NSImageView` on macOS.
pub fn native_image_view(
    id: impl Into<ElementId>,
    symbol_name: impl Into<SharedString>,
) -> NativeImageView {
    NativeImageView {
        id: id.into(),
        layout_style: StyleRefinement::default(),
        symbol_name: symbol_name.into(),
        weight: SymbolWeight::default(),
        scale: SymbolScale::default(),
        point_size: 0.0,
        tint_color: None,
    }
}

/// Platform-native SF Symbol image view backed by `NSImageView` on macOS.
pub struct NativeImageView {
    id: ElementId,
    layout_style: StyleRefinement,
    symbol_name: SharedString,
    weight: SymbolWeight,
    scale: SymbolScale,
    point_size: f64,
    tint_color: Option<(f64, f64, f64, f64)>,
}

impl NativeImageView {
    /// Set the symbol rendering weight.
    pub fn weight(mut self, weight: SymbolWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the symbol rendering scale.
    pub fn scale(mut self, scale: SymbolScale) -> Self {
        self.scale = scale;
        self
    }

    /// Set the point size (0.0 uses system default).
    pub fn point_size(mut self, size: f64) -> Self {
        self.point_size = size;
        self
    }

    /// RGBA tint color, each component 0.0–1.0.
    pub fn tint_color(mut self, r: f64, g: f64, b: f64, a: f64) -> Self {
        self.tint_color = Some((r, g, b, a));
        self
    }
}

impl IntoElement for NativeImageView {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeImageView {
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

        let symbol_name = self.symbol_name.clone();
        let weight = self.weight;
        let scale = self.scale;
        let point_size = self.point_size;
        let tint_color = self.tint_color;
        let parent = window.raw_native_view_ptr();

        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

        window.with_optional_element_state::<NativeControlState, _>(id, |prev, window| {
            let mut state = prev.flatten().unwrap_or_default();
            let nc = window.native_controls().expect("checked above");
            nc.update_image_view(
                &mut state,
                parent,
                bounds,
                window.scale_factor(),
                ImageViewConfig {
                    symbol_name: &symbol_name,
                    weight,
                    scale,
                    point_size,
                    tint_color,
                },
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

impl Styled for NativeImageView {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.layout_style
    }
}
