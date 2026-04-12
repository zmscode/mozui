use mozui::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Size, Style, Window,
};
use objc2::rc::Retained;
use objc2::{ClassType, MainThreadMarker, msg_send};
use objc2_app_kit::{NSColor, NSImage, NSImageSymbolConfiguration, NSImageView, NSView};
use objc2_foundation::NSString;

use crate::native_view::{NativeViewState, parent_ns_view};

/// SF Symbol weight variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymbolWeight {
    UltraLight,
    Thin,
    Light,
    #[default]
    Regular,
    Medium,
    Semibold,
    Bold,
    Heavy,
    Black,
}

impl SymbolWeight {
    fn to_ns(self) -> f64 {
        // NSFontWeight values
        match self {
            Self::UltraLight => -0.8,
            Self::Thin => -0.6,
            Self::Light => -0.4,
            Self::Regular => 0.0,
            Self::Medium => 0.23,
            Self::Semibold => 0.3,
            Self::Bold => 0.4,
            Self::Heavy => 0.56,
            Self::Black => 0.62,
        }
    }
}

/// SF Symbol scale variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymbolScale {
    Small,
    #[default]
    Medium,
    Large,
}

impl SymbolScale {
    fn to_ns(self) -> isize {
        // NSImageSymbolScale values
        match self {
            Self::Small => 1,
            Self::Medium => 2,
            Self::Large => 3,
        }
    }
}

/// A native SF Symbol image element using `NSImageView`.
///
/// Renders an SF Symbol from Apple's system symbol library. Available on macOS 11+.
///
/// ```rust,no_run
/// NativeSymbol::new("sidebar-icon", "sidebar.left")
///     .weight(SymbolWeight::Medium)
///     .scale(SymbolScale::Large)
///     .tint(0.6, 0.6, 0.6, 1.0)
/// ```
pub struct NativeSymbol {
    id: ElementId,
    name: String,
    weight: SymbolWeight,
    scale: SymbolScale,
    tint: Option<(f64, f64, f64, f64)>, // r, g, b, a
}

impl NativeSymbol {
    pub fn new(id: impl Into<ElementId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            weight: SymbolWeight::default(),
            scale: SymbolScale::default(),
            tint: None,
        }
    }

    pub fn weight(mut self, weight: SymbolWeight) -> Self {
        self.weight = weight;
        self
    }

    pub fn scale(mut self, scale: SymbolScale) -> Self {
        self.scale = scale;
        self
    }

    /// Set tint color as RGBA (0.0 to 1.0).
    pub fn tint(mut self, r: f64, g: f64, b: f64, a: f64) -> Self {
        self.tint = Some((r, g, b, a));
        self
    }
}

fn create_symbol_image_view(
    name: &str,
    weight: SymbolWeight,
    scale: SymbolScale,
    tint: Option<(f64, f64, f64, f64)>,
    mtm: MainThreadMarker,
) -> Option<Retained<NSView>> {
    let ns_name = NSString::from_str(name);

    // NSImage.imageWithSystemSymbolName:accessibilityDescription:
    let image: Option<Retained<NSImage>> = unsafe {
        msg_send![
            NSImage::class(),
            imageWithSystemSymbolName: &*ns_name,
            accessibilityDescription: std::ptr::null::<NSString>()
        ]
    };

    let image = image?;

    // Create symbol configuration with weight and scale
    let weight_value = weight.to_ns();
    let scale_value = scale.to_ns();

    let config: Retained<NSImageSymbolConfiguration> = unsafe {
        msg_send![
            NSImageSymbolConfiguration::class(),
            configurationWithPointSize: 0.0_f64,
            weight: weight_value,
            scale: scale_value
        ]
    };

    let configured_image: Retained<NSImage> =
        unsafe { msg_send![&image, imageWithSymbolConfiguration: &*config] };

    let image_view = NSImageView::imageViewWithImage(&configured_image, mtm);

    // Apply tint via contentTintColor
    if let Some((r, g, b, a)) = tint {
        unsafe {
            let color: Retained<NSColor> = msg_send![
                NSColor::class(),
                colorWithRed: r,
                green: g,
                blue: b,
                alpha: a
            ];
            let _: () = msg_send![&image_view, setContentTintColor: &*color];
        }
    }

    let view: Retained<NSView> = unsafe { Retained::cast_unchecked(image_view) };
    Some(view)
}

impl IntoElement for NativeSymbol {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeSymbol {
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
        let name = self.name.clone();
        let weight = self.weight;
        let scale = self.scale;
        let tint = self.tint;

        window.with_element_state(global_id, |state: Option<NativeViewState>, window| {
            let parent = parent_ns_view(window);

            let mut state = state.unwrap_or_else(|| {
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let view = create_symbol_image_view(&name, weight, scale, tint, mtm)
                    .expect("SF Symbol not found — check the symbol name");
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
