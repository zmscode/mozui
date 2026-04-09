use crate::{DeferredPosition, Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_style::{Placement, Rect};

/// A positioned floating element anchored to a specific point or region.
///
/// Uses the deferred element system: the child is laid out in an independent
/// tree and painted on top of the main tree, positioned relative to
/// `anchor_id` using `placement` and `gap`.
pub struct Popover {
    child: Box<dyn Element>,
    anchor_id: LayoutId,
    placement: Placement,
    gap: f32,
}

impl Popover {
    /// Create a popover positioned relative to the element identified by `anchor_id`.
    ///
    /// The `anchor_id` must be a `LayoutId` returned by a previous `layout()` call
    /// in the same frame (i.e., the anchor element must be laid out before the popover).
    pub fn new(child: Box<dyn Element>, anchor_id: LayoutId) -> Self {
        Self {
            child,
            anchor_id,
            placement: Placement::Bottom,
            gap: 4.0,
        }
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

impl Element for Popover {
    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Defer the child for independent layout + paint-on-top.
        // We need to take the child out since it moves into the DeferredEntry.
        let child = std::mem::replace(
            &mut self.child,
            Box::new(crate::div::EmptyPlaceholder),
        );

        cx.defer(
            child,
            DeferredPosition::Anchored {
                anchor_id: self.anchor_id,
                placement: self.placement,
                gap: self.gap,
            },
        );

        // Return a zero-size placeholder — the popover doesn't participate in parent layout
        cx.new_leaf(taffy::Style::default())
    }

    fn paint(&mut self, _bounds: Rect, _cx: &mut PaintContext) {
        // Nothing — child is painted by the deferred system
    }
}
