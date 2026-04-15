use crate::Refineable as _;
use crate::platform::native_controls::{
    NativeControlState, VisualEffectActiveState, VisualEffectBlending, VisualEffectConfig,
    VisualEffectMaterial,
};
use crate::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Style, StyleRefinement, Styled, Window,
};

/// A platform-native blur / vibrancy view backed by `NSVisualEffectView` on macOS.
pub fn native_visual_effect(id: impl Into<ElementId>) -> NativeVisualEffect {
    NativeVisualEffect::new(id)
}

/// A platform-native blur / vibrancy view backed by `NSVisualEffectView` on macOS.
pub struct NativeVisualEffect {
    id: ElementId,
    style: StyleRefinement,
    material: VisualEffectMaterial,
    blending: VisualEffectBlending,
    active_state: VisualEffectActiveState,
    is_emphasized: bool,
}

impl NativeVisualEffect {
    /// Create a new `NativeVisualEffect` with the given stable element id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            material: VisualEffectMaterial::default(),
            blending: VisualEffectBlending::default(),
            active_state: VisualEffectActiveState::default(),
            is_emphasized: false,
        }
    }

    /// Set the blur material.
    pub fn material(mut self, material: VisualEffectMaterial) -> Self {
        self.material = material;
        self
    }

    /// Set the blending mode.
    pub fn blending(mut self, blending: VisualEffectBlending) -> Self {
        self.blending = blending;
        self
    }

    /// Set the active state.
    pub fn active_state(mut self, active_state: VisualEffectActiveState) -> Self {
        self.active_state = active_state;
        self
    }

    /// Set whether the view renders in emphasized (selected) state.
    pub fn is_emphasized(mut self, emphasized: bool) -> Self {
        self.is_emphasized = emphasized;
        self
    }
}

impl IntoElement for NativeVisualEffect {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeVisualEffect {
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
        let mut style = Style {
            flex_shrink: 1.,
            ..Default::default()
        };
        style.refine(&self.style);
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

        let material = self.material;
        let blending = self.blending;
        let active_state = self.active_state;
        let is_emphasized = self.is_emphasized;
        let parent = window.raw_native_view_ptr();

        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

        window.with_optional_element_state::<NativeControlState, _>(id, |prev, window| {
            let mut state = prev.flatten().unwrap_or_default();
            let nc = window.native_controls().expect("checked above");
            nc.update_visual_effect(
                &mut state,
                parent,
                bounds,
                window.scale_factor(),
                VisualEffectConfig {
                    material,
                    blending,
                    active_state,
                    is_emphasized,
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

impl Styled for NativeVisualEffect {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
