use mozui::{
    App, Bounds, ContentMask, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Size, Style, Window,
};
#[cfg(target_os = "ios")]
use objc2::MainThreadOnly;
#[cfg(target_os = "macos")]
use objc2::msg_send;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSProgressIndicator;
#[cfg(target_os = "macos")]
use objc2_foundation::NSRect;
#[cfg(target_os = "ios")]
use objc2_ui_kit::{UIProgressView, UIProgressViewStyle};

use crate::native_view::NativeViewState;
#[cfg(target_os = "macos")]
use crate::native_view::parent_ns_view;
#[cfg(target_os = "ios")]
use crate::native_view::parent_ui_view;

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
        #[cfg(target_os = "macos")]
        let style = self.style;

        window.with_element_state(
            global_id,
            |state: Option<NativeProgressPersistentState>, window| {
                #[cfg(target_os = "ios")]
                let parent = parent_ui_view(window);
                #[cfg(target_os = "macos")]
                let parent = parent_ns_view(window);

                let mut state = state.unwrap_or_else(|| {
                    #[cfg(target_os = "ios")]
                    let indicator = {
                        let mtm = unsafe { objc2::MainThreadMarker::new_unchecked() };
                        UIProgressView::initWithProgressViewStyle(
                            UIProgressView::alloc(mtm),
                            UIProgressViewStyle::Default,
                        )
                    };
                    #[cfg(target_os = "macos")]
                    let mtm = unsafe { objc2::MainThreadMarker::new_unchecked() };
                    #[cfg(target_os = "macos")]
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

                #[cfg(target_os = "ios")]
                {
                    let progress_view: &UIProgressView =
                        unsafe { &*(state.view_state.view() as *const _ as *const UIProgressView) };
                    let normalized = match value {
                        Some(v) if max > min => ((v - min) / (max - min)).clamp(0.0, 1.0) as f32,
                        _ => 0.0,
                    };
                    progress_view.setProgress_animated(normalized, false);
                }
                #[cfg(target_os = "macos")]
                {
                    let indicator: &NSProgressIndicator = unsafe {
                        &*(state.view_state.view() as *const _ as *const NSProgressIndicator)
                    };
                    match value {
                        Some(v) => {
                            indicator.setIndeterminate(false);
                            indicator.setDoubleValue(v);
                        }
                        None => {
                            indicator.setIndeterminate(true);
                            unsafe {
                                indicator.startAnimation(None);
                            }
                        }
                    }
                }

                #[cfg(target_os = "ios")]
                let hitbox = if let Some(parent) = parent {
                    state.view_state.attach_and_position(parent, bounds);
                    Some(window.insert_hitbox(bounds, HitboxBehavior::Normal))
                } else {
                    None
                };
                #[cfg(target_os = "macos")]
                let hitbox = {
                    state.view_state.attach_and_position(parent, bounds);
                    Some(window.insert_hitbox(bounds, HitboxBehavior::Normal))
                };
                (hitbox, state)
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
