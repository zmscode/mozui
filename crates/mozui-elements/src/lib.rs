mod div;
mod text;
mod text_input;

pub use div::{Div, div};
pub use text::{Text, text};
pub use text_input::{TextInput, TextInputState, text_input};

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
        font_system: &FontSystem,
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

/// A focusable region with key handler.
struct FocusableEntry {
    id: usize,
    bounds: Rect,
    on_focus: Box<dyn Fn(bool, &mut dyn std::any::Any)>,
    on_key: KeyHandler,
}

/// Collects interactive regions during paint, hit-tests on events.
pub struct InteractionMap {
    entries: Vec<InteractionEntry>,
    key_handlers: Vec<KeyHandler>,
    focusables: Vec<FocusableEntry>,
    focused_id: Option<usize>,
    next_focus_id: usize,
    drag_regions: Vec<Rect>,
}

impl InteractionMap {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            key_handlers: Vec::new(),
            focusables: Vec::new(),
            focused_id: None,
            next_focus_id: 0,
            drag_regions: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.key_handlers.clear();
        self.focusables.clear();
        self.next_focus_id = 0;
        self.drag_regions.clear();
    }

    /// Register a drag region (for window title bar dragging).
    pub fn register_drag_region(&mut self, bounds: Rect) {
        self.drag_regions.push(bounds);
    }

    /// Check if a point is in a drag region (and not over a clickable/focusable element).
    pub fn is_drag_region(&self, position: Point) -> bool {
        let in_drag = self.drag_regions.iter().any(|r| r.contains(position));
        if !in_drag {
            return false;
        }
        // Don't drag if clicking on interactive elements within the title bar
        let on_interactive = self.entries.iter().any(|e| e.bounds.contains(position))
            || self.focusables.iter().any(|e| e.bounds.contains(position));
        !on_interactive
    }

    /// Register a key handler (global — always receives events).
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

    /// Register a focusable element. Returns its focus ID.
    /// on_focus is called with true/false when focus changes.
    /// on_key is only dispatched when this element is focused.
    pub fn register_focusable(
        &mut self,
        bounds: Rect,
        on_focus: Box<dyn Fn(bool, &mut dyn std::any::Any)>,
        on_key: KeyHandler,
    ) -> usize {
        let id = self.next_focus_id;
        self.next_focus_id += 1;
        self.focusables.push(FocusableEntry {
            id,
            bounds,
            on_focus,
            on_key,
        });
        id
    }

    /// Find the topmost handler at a point and invoke it.
    /// Returns true if a handler was found and invoked.
    pub fn dispatch_click(&mut self, position: Point, cx: &mut dyn std::any::Any) -> bool {
        // Check focusables first (they're painted later, so on top)
        for entry in self.focusables.iter().rev() {
            if entry.bounds.contains(position) {
                let new_id = entry.id;
                if self.focused_id != Some(new_id) {
                    // Blur old
                    if let Some(old_id) = self.focused_id {
                        if let Some(old) = self.focusables.iter().find(|e| e.id == old_id) {
                            (old.on_focus)(false, cx);
                        }
                    }
                    // Focus new
                    (entry.on_focus)(true, cx);
                    self.focused_id = Some(new_id);
                }
                return true;
            }
        }

        // Blur any focused element when clicking elsewhere
        if let Some(old_id) = self.focused_id.take() {
            if let Some(old) = self.focusables.iter().find(|e| e.id == old_id) {
                (old.on_focus)(false, cx);
            }
        }

        // Regular click handlers
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

    /// Dispatch a key event. If an element is focused, dispatch to it first,
    /// then always dispatch to global key handlers too.
    pub fn dispatch_key(
        &self,
        key: mozui_events::Key,
        modifiers: mozui_events::Modifiers,
        cx: &mut dyn std::any::Any,
    ) -> bool {
        let mut handled = false;

        if let Some(focused_id) = self.focused_id {
            if let Some(entry) = self.focusables.iter().find(|e| e.id == focused_id) {
                (entry.on_key)(key, modifiers, cx);
                handled = true;
            }
        }

        // Always dispatch to global key handlers (e.g. Escape to quit)
        for handler in &self.key_handlers {
            handler(key, modifiers, cx);
            handled = true;
        }

        handled
    }

    /// Check if a point is over any interactive element (including focusables).
    pub fn has_handler_at(&self, position: Point) -> bool {
        self.focusables
            .iter()
            .rev()
            .any(|e| e.bounds.contains(position))
            || self
                .entries
                .iter()
                .rev()
                .any(|e| e.bounds.contains(position))
    }

    /// Get the appropriate cursor style for a position.
    pub fn cursor_at(&self, position: Point) -> mozui_events::CursorStyle {
        // Focusables (text inputs) get text cursor
        if self
            .focusables
            .iter()
            .rev()
            .any(|e| e.bounds.contains(position))
        {
            return mozui_events::CursorStyle::Text;
        }
        // Click handlers get hand cursor
        if self
            .entries
            .iter()
            .rev()
            .any(|e| e.bounds.contains(position))
        {
            return mozui_events::CursorStyle::Hand;
        }
        mozui_events::CursorStyle::Arrow
    }

    /// Tab to next/previous focusable element. Returns true if focus changed.
    pub fn cycle_focus(&mut self, reverse: bool, cx: &mut dyn std::any::Any) -> bool {
        if self.focusables.is_empty() {
            return false;
        }
        let current_idx = self
            .focused_id
            .and_then(|id| self.focusables.iter().position(|e| e.id == id));
        let len = self.focusables.len();
        let next_idx = if reverse {
            match current_idx {
                Some(idx) => {
                    if idx == 0 {
                        len - 1
                    } else {
                        idx - 1
                    }
                }
                None => len - 1,
            }
        } else {
            match current_idx {
                Some(idx) => (idx + 1) % len,
                None => 0,
            }
        };

        // Blur old
        if let Some(old_id) = self.focused_id {
            if let Some(old) = self.focusables.iter().find(|e| e.id == old_id) {
                (old.on_focus)(false, cx);
            }
        }

        let next = &self.focusables[next_idx];
        (next.on_focus)(true, cx);
        self.focused_id = Some(next.id);
        true
    }
}
