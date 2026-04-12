use mozui::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Size, Style, Window,
};
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
};

use crate::native_view::{NativeViewState, parent_ns_view};

/// The material (blur style) for a [`NativeVisualEffect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualEffectMaterial {
    Titlebar,
    #[default]
    Selection,
    Menu,
    Popover,
    Sidebar,
    HeaderView,
    Sheet,
    WindowBackground,
    HudWindow,
    FullScreenUI,
    ToolTip,
    ContentBackground,
    UnderWindowBackground,
    UnderPageBackground,
}

impl VisualEffectMaterial {
    fn to_ns(self) -> NSVisualEffectMaterial {
        match self {
            Self::Titlebar => NSVisualEffectMaterial::Titlebar,
            Self::Selection => NSVisualEffectMaterial::Selection,
            Self::Menu => NSVisualEffectMaterial::Menu,
            Self::Popover => NSVisualEffectMaterial::Popover,
            Self::Sidebar => NSVisualEffectMaterial::Sidebar,
            Self::HeaderView => NSVisualEffectMaterial::HeaderView,
            Self::Sheet => NSVisualEffectMaterial::Sheet,
            Self::WindowBackground => NSVisualEffectMaterial::WindowBackground,
            Self::HudWindow => NSVisualEffectMaterial::HUDWindow,
            Self::FullScreenUI => NSVisualEffectMaterial::FullScreenUI,
            Self::ToolTip => NSVisualEffectMaterial::ToolTip,
            Self::ContentBackground => NSVisualEffectMaterial::ContentBackground,
            Self::UnderWindowBackground => NSVisualEffectMaterial::UnderWindowBackground,
            Self::UnderPageBackground => NSVisualEffectMaterial::UnderPageBackground,
        }
    }
}

/// The blending mode for a [`NativeVisualEffect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualEffectBlending {
    #[default]
    BehindWindow,
    WithinWindow,
}

impl VisualEffectBlending {
    fn to_ns(self) -> NSVisualEffectBlendingMode {
        match self {
            Self::BehindWindow => NSVisualEffectBlendingMode::BehindWindow,
            Self::WithinWindow => NSVisualEffectBlendingMode::WithinWindow,
        }
    }
}

/// The active state for a [`NativeVisualEffect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualEffectActiveState {
    /// Follows window active state.
    FollowsWindowActiveState,
    /// Always active (blur visible even when window is inactive).
    #[default]
    Active,
    /// Inactive.
    Inactive,
}

impl VisualEffectActiveState {
    fn to_ns(self) -> NSVisualEffectState {
        match self {
            Self::FollowsWindowActiveState => NSVisualEffectState::FollowsWindowActiveState,
            Self::Active => NSVisualEffectState::Active,
            Self::Inactive => NSVisualEffectState::Inactive,
        }
    }
}

/// A native `NSVisualEffectView` element for blur and vibrancy effects.
pub struct NativeVisualEffect {
    id: ElementId,
    material: VisualEffectMaterial,
    blending: VisualEffectBlending,
    state: VisualEffectActiveState,
    is_emphasized: bool,
}

impl NativeVisualEffect {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            material: VisualEffectMaterial::default(),
            blending: VisualEffectBlending::default(),
            state: VisualEffectActiveState::default(),
            is_emphasized: false,
        }
    }

    pub fn material(mut self, material: VisualEffectMaterial) -> Self {
        self.material = material;
        self
    }

    pub fn blending(mut self, blending: VisualEffectBlending) -> Self {
        self.blending = blending;
        self
    }

    pub fn state(mut self, state: VisualEffectActiveState) -> Self {
        self.state = state;
        self
    }

    /// When true, emphasizes the visual effect (e.g., for a selected source list row).
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
        let material = self.material;
        let blending = self.blending;
        let active_state = self.state;
        let is_emphasized = self.is_emphasized;

        window.with_element_state(global_id, |state: Option<NativeViewState>, window| {
            let parent = parent_ns_view(window);

            let mut state = state.unwrap_or_else(|| {
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let effect_view = NSVisualEffectView::new(mtm);
                effect_view.setMaterial(material.to_ns());
                effect_view.setBlendingMode(blending.to_ns());
                effect_view.setState(active_state.to_ns());
                effect_view.setEmphasized(is_emphasized);
                NativeViewState::new(unsafe { objc2::rc::Retained::cast_unchecked(effect_view) })
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
