use crate::Refineable as _;
use crate::platform::native_controls::{NativeControlState, ProgressConfig, ProgressStyle};
use crate::{
    AbsoluteLength, App, Bounds, DefiniteLength, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Length, Pixels, Style, StyleRefinement, Styled,
    Window, px,
};

/// Create a platform-native progress indicator element.
pub fn native_progress(id: impl Into<ElementId>) -> NativeProgress {
    NativeProgress {
        id: id.into(),
        value: None,
        min: 0.0,
        max: 100.0,
        style: ProgressStyle::default(),
        style_refinement: StyleRefinement::default(),
    }
}

/// A semantic wrapper around a platform-native progress indicator.
pub struct NativeProgress {
    id: ElementId,
    value: Option<f64>,
    min: f64,
    max: f64,
    style: ProgressStyle,
    style_refinement: StyleRefinement,
}

impl NativeProgress {
    /// Set the determinate value. Leave unset for an indeterminate indicator.
    pub fn value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    /// Set the value range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Set the indicator style.
    pub fn progress_style(mut self, style: ProgressStyle) -> Self {
        self.style = style;
        self
    }
}

impl IntoElement for NativeProgress {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NativeProgress {
    type RequestLayoutState = ();
    type PrepaintState = Bounds<Pixels>;

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
        let mut style = Style::default();
        style.refine(&self.style_refinement);

        if matches!(style.size.width, Length::Auto) {
            style.size.width =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(140.0))));
        }
        if matches!(style.size.height, Length::Auto) {
            let height = match self.style {
                ProgressStyle::Bar => 14.0,
                ProgressStyle::Spinning => 20.0,
            };
            style.size.height =
                Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(height))));
        }

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        bounds
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        if id.is_none() {
            debug_assert!(false, "native controls require a stable element id");
            return;
        }
        if window.raw_native_view_ptr().is_null() || window.native_controls().is_none() {
            return;
        }

        let value = self.value;
        let min = self.min;
        let max = self.max;
        let style = self.style;

        window.with_optional_element_state::<NativeControlState, _>(id, |prev_state, window| {
            let mut state = prev_state.flatten().unwrap_or_default();
            let parent = window.raw_native_view_ptr();
            let native_controls = window
                .native_controls()
                .expect("native controls availability checked before paint");

            native_controls.update_progress(
                &mut state,
                parent,
                bounds,
                window.scale_factor(),
                ProgressConfig {
                    value,
                    min,
                    max,
                    style,
                },
            );

            ((), Some(state))
        });
    }
}

impl Styled for NativeProgress {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style_refinement
    }
}
