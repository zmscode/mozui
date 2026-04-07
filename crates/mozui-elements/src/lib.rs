mod div;
mod text;

pub use div::{Div, div};
pub use text::{Text, text};

use mozui_layout::LayoutEngine;
use mozui_renderer::DrawList;
use mozui_style::{Point, Rect};
use mozui_text::FontSystem;
use taffy::NodeId;

/// A node in the UI element tree.
pub trait Element {
    /// Build this element's Taffy layout node and return it.
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> NodeId;

    /// Paint this element using the computed layout positions.
    /// `layouts` is a pre-order traversal of computed layouts; `index` is consumed as we go.
    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
    );
}

/// Stored click handler — captures signal setters etc.
type ClickHandler = Box<dyn Fn(&mut dyn std::any::Any)>;

/// An interactive region with its handler.
struct InteractionEntry {
    bounds: Rect,
    on_click: ClickHandler,
}

/// Collects interactive regions during paint, hit-tests on events.
pub struct InteractionMap {
    entries: Vec<InteractionEntry>,
}

impl InteractionMap {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Register a click handler for a region.
    pub fn register_click(&mut self, bounds: Rect, handler: ClickHandler) {
        self.entries.push(InteractionEntry {
            bounds,
            on_click: handler,
        });
    }

    /// Find the topmost handler at a point and invoke it.
    /// Returns true if a handler was found and invoked.
    pub fn dispatch_click(&self, position: Point, cx: &mut dyn std::any::Any) -> bool {
        for entry in self.entries.iter().rev() {
            if entry.bounds.contains(position) {
                (entry.on_click)(cx);
                return true;
            }
        }
        false
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if a point is over any interactive element.
    pub fn has_handler_at(&self, position: Point) -> bool {
        self.entries.iter().rev().any(|e| e.bounds.contains(position))
    }
}
