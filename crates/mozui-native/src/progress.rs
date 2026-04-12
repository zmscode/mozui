use mozui::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Size, Style, Window,
};
use objc2::msg_send;
use objc2_app_kit::NSProgressIndicator;
use objc2_foundation::NSRect;

use crate::native_view::{NativeViewState, parent_ns_view};

/// Progress indicator style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressStyle {
    /// Horizontal bar (determinate or indeterminate).
    #[default]
    Bar,
    /// Spinning indicator (indeterminate).
    Spinning,
}

/// A native macOS `NSProgressIndicator` element.
///
/// Maps to SwiftUI's `ProgressView`.
pub struct NativeProgress {
    id: ElementId,
    value: Option<f64>,
    min: f64,
    max: f64,
    style: ProgressStyle,
}

impl NativeProgress {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: None,
            min: 0.0,
            max: 100.0,
            style: ProgressStyle::default(),
        }
    }

    /// Set determinate value. If not set, the indicator is indeterminate.
    pub fn value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn style(mut self, style: ProgressStyle) -> Self {
        self.style = style;
        self
    }
}

struct NativeProgressPersistentState {
    view_state: NativeViewState,
}

impl IntoElement for NativeProgress {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for NativeProgress {
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
        let value = self.value;
        let min = self.min;
        let max = self.max;
        let style = self.style;

        window.with_element_state(
            global_id,
            |state: Option<NativeProgressPersistentState>, window| {
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    let mtm = unsafe { objc2::MainThreadMarker::new_unchecked() };
                    let indicator = unsafe {
                        let indicator =
                            NSProgressIndicator::initWithFrame(mtm.alloc(), NSRect::ZERO);
                        indicator.setMinValue(min);
                        indicator.setMaxValue(max);

                        // NSProgressIndicatorStyleBar = 0, NSProgressIndicatorStyleSpinning = 1
                        let ns_style: isize = match style {
                            ProgressStyle::Bar => 0,
                            ProgressStyle::Spinning => 1,
                        };
                        let _: () = msg_send![&indicator, setStyle: ns_style];

                        match value {
                            Some(v) => {
                                indicator.setIndeterminate(false);
                                indicator.setDoubleValue(v);
                            }
                            None => {
                                indicator.setIndeterminate(true);
                                indicator.startAnimation(None);
                            }
                        }

                        indicator
                    };

                    let view_state = NativeViewState::new(unsafe {
                        objc2::rc::Retained::cast_unchecked(indicator)
                    });

                    NativeProgressPersistentState { view_state }
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
