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
type KeyHandler = Box<dyn Fn(mozui_events::Key, mozui_events::Modifiers, &mut dyn std::any::Any)>;

/// An interactive region with its handler.
struct InteractionEntry {
    bounds: Rect,
    on_click: ClickHandler,
}

/// Collects interactive regions during paint, hit-tests on events.
pub struct InteractionMap {
    entries: Vec<InteractionEntry>,
    key_handlers: Vec<KeyHandler>,
}

impl InteractionMap {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            key_handlers: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.key_handlers.clear();
    }

    /// Register a key handler.
    pub fn register_key_handler(&mut self, handler: KeyHandler) {
        self.key_handlers.push(handler);
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

    /// Dispatch a key event to all registered key handlers.
    pub fn dispatch_key(
        &self,
        key: mozui_events::Key,
        modifiers: mozui_events::Modifiers,
        cx: &mut dyn std::any::Any,
    ) -> bool {
        for handler in &self.key_handlers {
            handler(key, modifiers, cx);
        }
        !self.key_handlers.is_empty()
    }

    /// Check if a point is over any interactive element.
    pub fn has_handler_at(&self, position: Point) -> bool {
        self.entries.iter().rev().any(|e| e.bounds.contains(position))
    }
}
