mod accordion;
mod avatar;
mod badge;
mod breadcrumb;
mod button;
mod checkbox;
mod color_picker;
pub(crate) mod collapsible;
mod description_list;
mod dialog;
mod div;
mod divider;
mod group_box;
mod icon;
mod img;
mod kbd;
mod label;
mod link;
mod list;
mod menu;
mod notification;
mod pagination;
mod popover;
mod progress;
mod radio;
mod rating;
mod root;
mod select;
mod skeleton;
mod table;
mod slider;
mod stepper;
mod styled;
mod switch;
mod tab;
mod tag;
mod text;
mod text_input;
mod tooltip;
mod virtual_list;

pub use accordion::{Accordion, AccordionItem, accordion, accordion_item};
pub use avatar::{Avatar, AvatarStatus, avatar};
pub use badge::{Badge, badge};
pub use breadcrumb::{Breadcrumb, BreadcrumbItem, breadcrumb, breadcrumb_item};
pub use button::{Button, ButtonGroup, ButtonVariant, button, button_group, icon_button};
pub use checkbox::{Checkbox, checkbox};
pub use color_picker::{ColorPicker, color_picker};
pub use collapsible::{CollapsibleContainer, collapsible};
pub use description_list::{DescriptionItem, DescriptionList, description_item, description_list};
pub use dialog::{DIALOG_ANIM_MS, Dialog, dialog, dialog_anim};
pub use div::{Div, ScrollOffset, div};
pub use divider::{Divider, DividerDirection, DividerVariant, divider};
pub use group_box::{GroupBox, group_box};
pub use icon::{Icon, icon};
pub use img::{AnimatedImage, ImageSource, Img, decode_gif_frames, decode_image, decode_image_file, decode_svg, decode_svg_file, img, img_animated};
pub use kbd::{Kbd, kbd};
pub use label::{Label, LabelHighlight, LabelHighlightMode, label};
pub use link::{Link, link};
pub use list::{List, ListItem, list, list_item};
pub use menu::{Menu, MenuItem, menu, menu_item, menu_separator};
pub use notification::{NOTIFICATION_ANIM_MS, Notification, NotificationPlacement, NotificationType, STACK_GAP as NOTIFICATION_STACK_GAP, notification, notification_anim};
pub use pagination::{Pagination, pagination};
pub use popover::{FitMode, Popover};
pub use progress::{Progress, progress};
pub use radio::{Radio, radio};
pub use rating::{Rating, rating};
pub use root::Root;
pub use select::{Select, SelectOption, select, select_option};
pub use skeleton::{Skeleton, SkeletonShape, skeleton};
pub use table::{ColumnWidth, SortDirection, Table, TableColumn, TableRow, table, table_column, table_row};
pub use slider::{Slider, slider};
pub use stepper::{Stepper, StepperItem, stepper};
pub use styled::{Collapsible, ComponentSize, Disableable, Selectable, Sizable};
pub use switch::{Switch, switch};
pub use tab::{Tab, TabBar, TabBarVariant, tab, tab_bar};
pub use tag::{Tag, TagVariant, tag};
pub use text::{Text, text};
pub use text_input::{TextInput, TextInputState, text_input};
pub use tooltip::{Tooltip, tooltip};
pub use virtual_list::{VirtualList, VirtualListDirection};

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
/// Drag handler receives the current mouse position (x, y) in absolute coordinates.
type DragHandler = Box<dyn Fn(Point, &mut dyn std::any::Any)>;
/// Scroll handler receives (delta_x, delta_y) in pixels.
type ScrollHandler = Box<dyn Fn(f32, f32, &mut dyn std::any::Any)>;
/// Drop handler receives (source_id, mouse position) when an item is dropped.
type DropHandler = Box<dyn Fn(DragId, Point, &mut dyn std::any::Any)>;
/// Right-click handler receives the mouse position where the context menu should appear.
type RightClickHandler = Box<dyn Fn(Point, &mut dyn std::any::Any)>;

/// Unique identifier for a drag source, used to match sources with compatible targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DragId(pub usize);

/// An interactive region with its handler.
struct InteractionEntry {
    bounds: Rect,
    on_click: ClickHandler,
}

/// A right-click region with its handler.
struct RightClickEntry {
    bounds: Rect,
    on_right_click: RightClickHandler,
}

/// A focusable region with key handler.
struct FocusableEntry {
    id: usize,
    bounds: Rect,
    on_focus: Box<dyn Fn(bool, &mut dyn std::any::Any)>,
    on_key: KeyHandler,
}

/// A draggable region — handler fires on mouse-down and mouse-move while pressed.
struct DragRegionEntry {
    id: u64,
    bounds: Rect,
    on_drag: DragHandler,
}

/// A scrollable region with its handler.
struct ScrollRegion {
    bounds: Rect,
    on_scroll: ScrollHandler,
}

/// A focus trap scope that constrains Tab navigation within a region.
/// Used by modals/dialogs to prevent focus from escaping.
struct FocusTrap {
    id: usize,
    /// The focusable IDs that belong to this trap.
    focusable_ids: Vec<usize>,
}

/// A draggable source — can be picked up and dropped onto a DropTarget.
struct DndSource {
    id: DragId,
    bounds: Rect,
}

/// A drop target — accepts items dropped onto it.
struct DndTarget {
    id: DragId,
    bounds: Rect,
    on_drop: DropHandler,
}

/// State of an active drag-and-drop operation.
struct ActiveDnd {
    source_id: DragId,
    source_bounds: Rect,
    /// Where the mouse was when the drag started.
    start_position: Point,
    /// Whether the drag has moved far enough from the origin to be considered active.
    activated: bool,
}

/// Collects interactive regions during paint, hit-tests on events.
pub struct InteractionMap {
    entries: Vec<InteractionEntry>,
    right_click_entries: Vec<RightClickEntry>,
    key_handlers: Vec<KeyHandler>,
    focusables: Vec<FocusableEntry>,
    focused_id: Option<usize>,
    next_focus_id: usize,
    drag_regions: Vec<Rect>,
    drag_handlers: Vec<DragRegionEntry>,
    next_drag_id: u64,
    /// The drag handler ID that was active when mouse-down started.
    active_drag_id: Option<u64>,
    scroll_regions: Vec<ScrollRegion>,
    focus_traps: Vec<FocusTrap>,
    next_trap_id: usize,
    /// The currently active trap (last pushed). Tab cycles within this trap only.
    active_trap_id: Option<usize>,
    /// Current mouse position (updated by the app event loop).
    mouse_position: Point,
    /// Whether the left mouse button is currently pressed.
    mouse_pressed: bool,
    /// The bounds of the element where mouse-down started (for active state).
    press_origin_bounds: Option<Rect>,
    /// Scroll offset transform stack. Registered bounds are adjusted by this offset
    /// so that layout coordinates are converted to screen coordinates for hit-testing.
    scroll_offset_stack: Vec<f32>,
    current_scroll_offset_y: f32,
    // ── Hover regions ──────────────────────────────────────────────
    /// Bounds that trigger re-render on hover (e.g. tooltip anchors).
    hover_regions: Vec<Rect>,
    // ── Drag-and-drop ─────────────────────────────────────────────
    dnd_sources: Vec<DndSource>,
    dnd_targets: Vec<DndTarget>,
    active_dnd: Option<ActiveDnd>,
}

impl InteractionMap {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            right_click_entries: Vec::new(),
            key_handlers: Vec::new(),
            focusables: Vec::new(),
            focused_id: None,
            next_focus_id: 0,
            drag_regions: Vec::new(),
            drag_handlers: Vec::new(),
            next_drag_id: 0,
            active_drag_id: None,
            scroll_regions: Vec::new(),
            focus_traps: Vec::new(),
            next_trap_id: 0,
            active_trap_id: None,
            mouse_position: Point::ZERO,
            mouse_pressed: false,
            press_origin_bounds: None,
            scroll_offset_stack: Vec::new(),
            current_scroll_offset_y: 0.0,
            hover_regions: Vec::new(),
            dnd_sources: Vec::new(),
            dnd_targets: Vec::new(),
            active_dnd: None,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.right_click_entries.clear();
        self.key_handlers.clear();
        self.focusables.clear();
        self.next_focus_id = 0;
        self.drag_regions.clear();
        self.drag_handlers.clear();
        self.next_drag_id = 0;
        // Note: active_drag_id persists across clears (drag may span rebuilds)
        self.scroll_regions.clear();
        self.focus_traps.clear();
        self.next_trap_id = 0;
        self.active_trap_id = None;
        self.scroll_offset_stack.clear();
        self.current_scroll_offset_y = 0.0;
        self.hover_regions.clear();
        self.dnd_sources.clear();
        self.dnd_targets.clear();
        // Note: active_dnd persists across clears (drag spans rebuilds)
        // Note: mouse_position, mouse_pressed, press_origin_bounds persist across clears
    }

    // ── Mouse state ─────────────────────────────────────────────

    /// Update the current mouse position. Called by the app event loop on MouseMove.
    pub fn set_mouse_position(&mut self, position: Point) {
        self.mouse_position = position;
    }

    /// Get the current mouse position.
    pub fn mouse_position(&self) -> Point {
        self.mouse_position
    }

    /// Mark the left mouse button as pressed at the given position.
    pub fn set_mouse_pressed(&mut self, position: Point) {
        self.mouse_pressed = true;
        self.mouse_position = position;
        // Record which clickable/interactive region was under the press
        // (last = topmost, check focusables then click entries)
        let mut found = None;
        for entry in self.focusables.iter().rev() {
            if entry.bounds.contains(position) {
                found = Some(entry.bounds);
                break;
            }
        }
        if found.is_none() {
            for entry in self.entries.iter().rev() {
                if entry.bounds.contains(position) {
                    found = Some(entry.bounds);
                    break;
                }
            }
        }
        self.press_origin_bounds = found;
    }

    /// Check if the mouse is currently pressed.
    pub fn is_mouse_pressed(&self) -> bool {
        self.mouse_pressed
    }

    /// Mark the left mouse button as released.
    pub fn set_mouse_released(&mut self) {
        self.mouse_pressed = false;
        self.press_origin_bounds = None;
    }

    /// Check if a given bounds region is currently hovered by the mouse.
    /// Call this during `paint` to determine visual hover state.
    /// Bounds are in layout coordinates and will be adjusted for scroll offset.
    pub fn is_hovered(&self, bounds: Rect) -> bool {
        self.adjust_bounds(bounds).contains(self.mouse_position)
    }

    /// Check if a given bounds region is in the "active" (pressed) state.
    /// True when the mouse is pressed AND the cursor is still over the element
    /// AND the press originated on this element.
    /// Bounds are in layout coordinates and will be adjusted for scroll offset.
    pub fn is_active(&self, bounds: Rect) -> bool {
        let adjusted = self.adjust_bounds(bounds);
        self.mouse_pressed
            && adjusted.contains(self.mouse_position)
            && self.press_origin_bounds.map_or(false, |origin| {
                origin == adjusted
            })
    }

    /// Register a hover-sensitive region. When the mouse is over any hover region,
    /// the app loop will trigger a re-render (needed for tooltips, etc.).
    pub fn register_hover_region(&mut self, bounds: Rect) {
        self.hover_regions.push(self.adjust_bounds(bounds));
    }

    // ── Scroll offset transform ───────────────────────────────

    /// Push a scroll offset. All bounds registered after this call (until pop)
    /// will be adjusted by this offset, converting layout coordinates to screen
    /// coordinates for correct hit-testing inside scroll containers.
    pub fn push_scroll_offset(&mut self, offset_y: f32) {
        self.scroll_offset_stack.push(self.current_scroll_offset_y);
        self.current_scroll_offset_y += offset_y;
    }

    /// Pop the most recent scroll offset, restoring the previous one.
    pub fn pop_scroll_offset(&mut self) {
        if let Some(prev) = self.scroll_offset_stack.pop() {
            self.current_scroll_offset_y = prev;
        }
    }

    /// Adjust bounds from layout coordinates to screen coordinates using
    /// the current scroll offset. Useful for drag handlers that need to
    /// capture screen-space coordinates for position calculations.
    pub fn adjust_bounds(&self, bounds: Rect) -> Rect {
        if self.current_scroll_offset_y == 0.0 {
            bounds
        } else {
            Rect::new(
                bounds.origin.x,
                bounds.origin.y + self.current_scroll_offset_y,
                bounds.size.width,
                bounds.size.height,
            )
        }
    }

    /// Register a drag region (for window title bar dragging).
    pub fn register_drag_region(&mut self, bounds: Rect) {
        self.drag_regions.push(self.adjust_bounds(bounds));
    }

    /// Register an element drag handler. The handler fires on mouse-down within
    /// the bounds and on every mouse-move while the button is held.
    /// Handlers are matched by registration order (stable ID) across rebuilds,
    /// so bounds can shift without breaking active drags.
    pub fn register_drag_handler(&mut self, bounds: Rect, handler: DragHandler) {
        let id = self.next_drag_id;
        self.next_drag_id += 1;
        self.drag_handlers.push(DragRegionEntry {
            id,
            bounds: self.adjust_bounds(bounds),
            on_drag: handler,
        });
    }

    /// Dispatch a drag-start event (called on MouseDown). Returns true if a drag handler
    /// was found and invoked.
    pub fn dispatch_drag_start(&mut self, position: Point, cx: &mut dyn std::any::Any) -> bool {
        for entry in self.drag_handlers.iter().rev() {
            if entry.bounds.contains(position) {
                self.active_drag_id = Some(entry.id);
                (entry.on_drag)(position, cx);
                return true;
            }
        }
        false
    }

    /// Dispatch a drag-move event (called on MouseMove while pressed). Returns true if handled.
    /// Matches by registration-order ID so that layout shifts during a drag don't break it.
    pub fn dispatch_drag_move(&self, position: Point, cx: &mut dyn std::any::Any) -> bool {
        if let Some(active_id) = self.active_drag_id {
            for entry in self.drag_handlers.iter().rev() {
                if entry.id == active_id {
                    (entry.on_drag)(position, cx);
                    return true;
                }
            }
        }
        false
    }

    /// Clear the active drag state (called on MouseUp).
    pub fn clear_active_drag(&mut self) {
        self.active_drag_id = None;
    }

    /// Returns true if there's an active drag handler.
    pub fn has_active_drag(&self) -> bool {
        self.active_drag_id.is_some()
    }

    // ── Drag-and-drop (DnD) ──────────────────────────────────────

    /// Register a drag source. When the user presses and drags from this region,
    /// a DnD session starts with the given `id`. Drop targets with the same `id`
    /// will accept the drop.
    pub fn register_dnd_source(&mut self, id: DragId, bounds: Rect) {
        self.dnd_sources.push(DndSource {
            id,
            bounds: self.adjust_bounds(bounds),
        });
    }

    /// Register a drop target. When a drag source with a matching `id` is released
    /// over this region, `on_drop` fires with `(source_id, mouse_position, cx)`.
    pub fn register_drop_target(
        &mut self,
        id: DragId,
        bounds: Rect,
        on_drop: impl Fn(DragId, Point, &mut dyn std::any::Any) + 'static,
    ) {
        self.dnd_targets.push(DndTarget {
            id,
            bounds: self.adjust_bounds(bounds),
            on_drop: Box::new(on_drop),
        });
    }

    /// Called on MouseDown — checks if the press is on a DnD source and begins
    /// a pending drag session. The drag doesn't activate until the mouse moves
    /// beyond a small threshold (to distinguish from clicks).
    /// Returns true if a DnD source was found.
    pub fn dnd_mouse_down(&mut self, position: Point) -> bool {
        for source in self.dnd_sources.iter().rev() {
            if source.bounds.contains(position) {
                self.active_dnd = Some(ActiveDnd {
                    source_id: source.id,
                    source_bounds: source.bounds,
                    start_position: position,
                    activated: false,
                });
                return true;
            }
        }
        false
    }

    /// Called on MouseMove while pressed — activates the drag if the mouse has
    /// moved far enough from the origin. Returns true if a DnD is active
    /// (caller should re-render for visual feedback).
    pub fn dnd_mouse_move(&mut self, position: Point) -> bool {
        if let Some(ref mut dnd) = self.active_dnd {
            if !dnd.activated {
                let dx = position.x - dnd.start_position.x;
                let dy = position.y - dnd.start_position.y;
                if dx * dx + dy * dy > 25.0 {
                    // 5px threshold
                    dnd.activated = true;
                }
            }
            return dnd.activated;
        }
        false
    }

    /// Called on MouseUp — if a DnD is active and the mouse is over a compatible
    /// drop target, fires the on_drop handler. Returns true if a drop was dispatched.
    pub fn dnd_mouse_up(&mut self, position: Point, cx: &mut dyn std::any::Any) -> bool {
        let dnd = match self.active_dnd.take() {
            Some(d) if d.activated => d,
            other => {
                self.active_dnd = other;
                return false;
            }
        };

        for target in self.dnd_targets.iter().rev() {
            if target.id == dnd.source_id && target.bounds.contains(position) {
                (target.on_drop)(dnd.source_id, position, cx);
                return true;
            }
        }
        false
    }

    /// Cancel any active DnD session without dropping.
    pub fn dnd_cancel(&mut self) {
        self.active_dnd = None;
    }

    /// Returns true if a DnD drag is in progress (mouse has moved past threshold).
    pub fn is_dnd_active(&self) -> bool {
        self.active_dnd
            .as_ref()
            .map_or(false, |d| d.activated)
    }

    /// Get the source ID of the active DnD session, if any.
    pub fn dnd_source_id(&self) -> Option<DragId> {
        self.active_dnd
            .as_ref()
            .filter(|d| d.activated)
            .map(|d| d.source_id)
    }

    /// Get the source bounds of the active DnD session (for ghost rendering).
    pub fn dnd_source_bounds(&self) -> Option<Rect> {
        self.active_dnd
            .as_ref()
            .filter(|d| d.activated)
            .map(|d| d.source_bounds)
    }

    /// Check if a drop target region is currently being hovered by an active DnD
    /// with the given ID. Call during paint for visual feedback (highlight drop zones).
    pub fn is_drop_hovered(&self, id: DragId, bounds: Rect) -> bool {
        match &self.active_dnd {
            Some(dnd) if dnd.activated && dnd.source_id == id => {
                self.adjust_bounds(bounds).contains(self.mouse_position)
            }
            _ => false,
        }
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

    /// Register a scrollable region. The handler receives (dx, dy) in pixels.
    pub fn register_scroll_region(&mut self, bounds: Rect, handler: ScrollHandler) {
        self.scroll_regions.push(ScrollRegion {
            bounds: self.adjust_bounds(bounds),
            on_scroll: handler,
        });
    }

    /// Dispatch a scroll event. Hits the deepest (last registered) scroll region
    /// containing the position. Returns true if handled.
    pub fn dispatch_scroll(
        &self,
        position: Point,
        delta_x: f32,
        delta_y: f32,
        cx: &mut dyn std::any::Any,
    ) -> bool {
        // Last registered = deepest in tree (painted last = on top)
        for region in self.scroll_regions.iter().rev() {
            if region.bounds.contains(position) {
                (region.on_scroll)(delta_x, delta_y, cx);
                return true;
            }
        }
        false
    }

    /// Register a key handler (global — always receives events).
    pub fn register_key_handler(&mut self, handler: KeyHandler) {
        self.key_handlers.push(handler);
    }

    /// Register a click handler for a region.
    pub fn register_click(&mut self, bounds: Rect, handler: ClickHandler) {
        self.entries.push(InteractionEntry {
            bounds: self.adjust_bounds(bounds),
            on_click: handler,
        });
    }

    /// Register a right-click (context menu) handler for a region.
    pub fn register_right_click(&mut self, bounds: Rect, handler: RightClickHandler) {
        self.right_click_entries.push(RightClickEntry {
            bounds: self.adjust_bounds(bounds),
            on_right_click: handler,
        });
    }

    /// Dispatch a right-click event. Returns true if a handler was found and invoked.
    pub fn dispatch_right_click(&self, position: Point, cx: &mut dyn std::any::Any) -> bool {
        for entry in self.right_click_entries.iter().rev() {
            if entry.bounds.contains(position) {
                (entry.on_right_click)(position, cx);
                return true;
            }
        }
        false
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
            bounds: self.adjust_bounds(bounds),
            on_focus,
            on_key,
        });
        self.add_to_active_trap(id);
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

    /// Check if a point is over any interactive element (including focusables and hover regions).
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
            || self
                .hover_regions
                .iter()
                .any(|r| r.contains(position))
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

    // ── Focus trap management ──────────────────────────────────────

    /// Push a focus trap. All focusable elements registered after this call
    /// (until `pop_focus_trap`) will belong to this trap. Tab navigation
    /// will be constrained to these elements when this trap is active.
    /// Returns a trap ID.
    pub fn push_focus_trap(&mut self) -> usize {
        let id = self.next_trap_id;
        self.next_trap_id += 1;
        self.focus_traps.push(FocusTrap {
            id,
            focusable_ids: Vec::new(),
        });
        self.active_trap_id = Some(id);
        id
    }

    /// Pop the most recent focus trap and restore the previous one (if any).
    pub fn pop_focus_trap(&mut self) {
        if let Some(active_id) = self.active_trap_id {
            self.focus_traps.retain(|t| t.id != active_id);
            self.active_trap_id = self.focus_traps.last().map(|t| t.id);
        }
    }

    /// Add a focusable ID to the currently active trap (called internally
    /// by `register_focusable` when a trap is active).
    fn add_to_active_trap(&mut self, focusable_id: usize) {
        if let Some(trap_id) = self.active_trap_id {
            if let Some(trap) = self.focus_traps.iter_mut().find(|t| t.id == trap_id) {
                trap.focusable_ids.push(focusable_id);
            }
        }
    }

    /// Tab to next/previous focusable element. Returns true if focus changed.
    /// Respects the active focus trap — if one is set, only cycles within
    /// the trapped focusable elements.
    pub fn cycle_focus(&mut self, reverse: bool, cx: &mut dyn std::any::Any) -> bool {
        // Determine which focusables to cycle through
        let eligible_ids: Option<Vec<usize>> = self.active_trap_id.and_then(|trap_id| {
            self.focus_traps
                .iter()
                .find(|t| t.id == trap_id)
                .map(|t| t.focusable_ids.clone())
        });

        let eligible: Vec<&FocusableEntry> = match &eligible_ids {
            Some(ids) => self
                .focusables
                .iter()
                .filter(|e| ids.contains(&e.id))
                .collect(),
            None => self.focusables.iter().collect(),
        };

        if eligible.is_empty() {
            return false;
        }

        let current_idx = self
            .focused_id
            .and_then(|id| eligible.iter().position(|e| e.id == id));
        let len = eligible.len();
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

        let next = eligible[next_idx];
        (next.on_focus)(true, cx);
        self.focused_id = Some(next.id);
        true
    }
}
