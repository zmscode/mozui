use crate::Element;
use mozui_layout::LayoutEngine;
use mozui_renderer::DrawList;
use mozui_style::{Anchor, Placement, Point, Rect, Size};
use mozui_text::FontSystem;
use taffy::NodeId;

/// How to handle overflow when a popover doesn't fit in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitMode {
    /// Clamp position so the popover stays within the window bounds.
    #[default]
    SnapToWindow,
    /// Clamp with margin from window edges.
    SnapToWindowWithMargin(u32), // stored as u32 for Eq
    /// Flip the anchor to the opposite side if it overflows.
    SwitchAnchor,
}

/// A positioned floating element anchored to a specific point or region.
///
/// Use this to build tooltips, dropdown menus, popovers, etc.
pub struct Popover {
    child: Box<dyn Element>,
    anchor_rect: Rect,
    placement: Placement,
    anchor_corner: Anchor,
    offset: Point,
    fit_mode: FitMode,
    window_size: Size,
}

impl Popover {
    /// Create a popover positioned relative to `anchor_rect`.
    pub fn new(child: Box<dyn Element>, anchor_rect: Rect, window_size: Size) -> Self {
        Self {
            child,
            anchor_rect,
            placement: Placement::Bottom,
            anchor_corner: Anchor::TopLeft,
            offset: Point::ZERO,
            fit_mode: FitMode::SnapToWindow,
            window_size,
        }
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor_corner = anchor;
        self
    }

    pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = Point::new(x, y);
        self
    }

    pub fn fit_mode(mut self, mode: FitMode) -> Self {
        self.fit_mode = mode;
        self
    }

    /// Calculate the position for the popover content given its size.
    fn compute_position(&self, content_size: Size) -> Point {
        let ar = &self.anchor_rect;

        // Initial position based on placement
        let (mut x, mut y) = match self.placement {
            Placement::Bottom => (ar.origin.x, ar.origin.y + ar.size.height),
            Placement::Top => (ar.origin.x, ar.origin.y - content_size.height),
            Placement::Right => (ar.origin.x + ar.size.width, ar.origin.y),
            Placement::Left => (ar.origin.x - content_size.width, ar.origin.y),
        };

        // Apply anchor corner offset
        match self.anchor_corner {
            Anchor::TopLeft => {}
            Anchor::TopCenter => x += (ar.size.width - content_size.width) / 2.0,
            Anchor::TopRight => x += ar.size.width - content_size.width,
            Anchor::BottomLeft => y += ar.size.height - content_size.height,
            Anchor::BottomCenter => {
                x += (ar.size.width - content_size.width) / 2.0;
                y += ar.size.height - content_size.height;
            }
            Anchor::BottomRight => {
                x += ar.size.width - content_size.width;
                y += ar.size.height - content_size.height;
            }
        }

        // Apply manual offset
        x += self.offset.x;
        y += self.offset.y;

        // Apply fit mode
        match self.fit_mode {
            FitMode::SnapToWindow => {
                x = x.clamp(0.0, (self.window_size.width - content_size.width).max(0.0));
                y = y.clamp(
                    0.0,
                    (self.window_size.height - content_size.height).max(0.0),
                );
            }
            FitMode::SnapToWindowWithMargin(margin) => {
                let m = margin as f32;
                x = x.clamp(m, (self.window_size.width - content_size.width - m).max(m));
                y = y.clamp(
                    m,
                    (self.window_size.height - content_size.height - m).max(m),
                );
            }
            FitMode::SwitchAnchor => {
                // If overflowing, flip to opposite side
                let overflows = x < 0.0
                    || y < 0.0
                    || x + content_size.width > self.window_size.width
                    || y + content_size.height > self.window_size.height;

                if overflows {
                    let flipped = self.placement.opposite();
                    let (fx, fy) = match flipped {
                        Placement::Bottom => (ar.origin.x, ar.origin.y + ar.size.height),
                        Placement::Top => (ar.origin.x, ar.origin.y - content_size.height),
                        Placement::Right => (ar.origin.x + ar.size.width, ar.origin.y),
                        Placement::Left => (ar.origin.x - content_size.width, ar.origin.y),
                    };
                    x = fx + self.offset.x;
                    y = fy + self.offset.y;
                    // Final clamp
                    x = x.clamp(0.0, (self.window_size.width - content_size.width).max(0.0));
                    y = y.clamp(
                        0.0,
                        (self.window_size.height - content_size.height).max(0.0),
                    );
                }
            }
        }

        Point::new(x, y)
    }
}

impl Element for Popover {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> NodeId {
        self.child.layout(engine, font_system)
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut crate::InteractionMap,
        font_system: &FontSystem,
    ) {
        // Get the child's computed size from layout
        if *index < layouts.len() {
            let layout = layouts[*index];
            let content_size = Size::new(layout.width, layout.height);
            let _pos = self.compute_position(content_size);

            // We need to offset the child's paint by the computed position.
            // Since we can't modify layouts directly, we'll adjust via
            // a temporary offset approach — the child paints at its layout
            // position which should be absolute.
            // For now, delegate to the child's paint directly.
            self.child
                .paint(layouts, index, draw_list, interactions, font_system);
        }
    }
}
