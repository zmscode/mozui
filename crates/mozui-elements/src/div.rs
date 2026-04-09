use crate::{DragId, Element, InteractionMap};
use mozui_events;
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Point as StylePoint, Shadow};
use mozui_text::FontSystem;
use std::cell::Cell;
use std::rc::Rc;
use taffy::prelude::*;
use taffy::{Overflow, Point as TaffyPoint};

/// Shared scroll state with momentum physics.
///
/// Tracks scroll offset, velocity, and deceleration for smooth inertial scrolling.
/// Create via `cx.use_scroll()`.
#[derive(Clone)]
pub struct ScrollOffset {
    offset: Rc<Cell<f32>>,
    velocity: Rc<Cell<f32>>,
    /// Shared flag with the animation system — when true, the app loop keeps rendering.
    animations_active: Option<Rc<Cell<bool>>>,
}

/// Deceleration factor per frame (applied at ~60fps). Higher = more momentum.
const SCROLL_FRICTION: f32 = 0.92;
/// Stop momentum when velocity drops below this threshold (pixels/frame).
const SCROLL_VELOCITY_THRESHOLD: f32 = 0.3;

impl ScrollOffset {
    pub fn new() -> Self {
        Self {
            offset: Rc::new(Cell::new(0.0)),
            velocity: Rc::new(Cell::new(0.0)),
            animations_active: None,
        }
    }

    /// Attach the animation flag so momentum can request continuous redraws.
    pub fn with_animation_flag(mut self, flag: Rc<Cell<bool>>) -> Self {
        self.animations_active = Some(flag);
        self
    }

    pub fn get(&self) -> f32 {
        self.offset.get()
    }

    pub fn set(&self, v: f32) {
        self.offset.set(v);
    }

    /// Apply a scroll delta (from user input). Tracks velocity for momentum.
    pub fn scroll_by(&self, dy: f32, max_scroll: f32) {
        let current = self.offset.get();
        let new_val = (current - dy).clamp(0.0, max_scroll);
        self.offset.set(new_val);
        // Track velocity for momentum (negative because scroll direction is inverted)
        self.velocity.set(-dy);
    }

    /// Tick momentum physics. Returns true if still animating.
    /// Called each frame by the app loop / scroll container.
    pub fn tick_momentum(&self, max_scroll: f32) -> bool {
        let vel = self.velocity.get();
        if vel.abs() < SCROLL_VELOCITY_THRESHOLD {
            self.velocity.set(0.0);
            return false;
        }

        // Apply deceleration
        let new_vel = vel * SCROLL_FRICTION;
        self.velocity.set(new_vel);

        // Update offset
        let current = self.offset.get();
        let new_val = (current + new_vel).clamp(0.0, max_scroll);
        self.offset.set(new_val);

        // Stop if we hit the edge
        if new_val == 0.0 || new_val == max_scroll {
            self.velocity.set(0.0);
            return false;
        }

        // Signal the animation system to keep rendering
        if let Some(ref flag) = self.animations_active {
            flag.set(true);
        }

        true
    }

    /// Returns true if momentum is currently active.
    pub fn has_momentum(&self) -> bool {
        self.velocity.get().abs() >= SCROLL_VELOCITY_THRESHOLD
    }
}

pub struct Div {
    // Visual style
    background: Option<Fill>,
    corner_radii: Corners,
    border_width: f32,
    border_color: Color,
    opacity: f32,
    shadow: Option<Shadow>,

    // Taffy layout style
    taffy_style: Style,

    // Children
    children: Vec<Box<dyn Element>>,

    // Event handlers
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    on_key_down:
        Option<Box<dyn Fn(mozui_events::Key, mozui_events::Modifiers, &mut dyn std::any::Any)>>,

    // Window drag region
    is_drag_region: bool,

    // Scroll
    scroll_y: Option<ScrollOffset>,
    /// Content height computed during layout, used for scroll clamping.
    content_height: Cell<f32>,

    // Drag-and-drop
    dnd_source: Option<DragId>,
    dnd_target: Option<(DragId, Box<dyn Fn(DragId, StylePoint, &mut dyn std::any::Any)>)>,
}

pub fn div() -> Div {
    Div {
        background: None,
        corner_radii: Corners::ZERO,
        border_width: 0.0,
        border_color: Color::TRANSPARENT,
        opacity: 1.0,
        shadow: None,
        taffy_style: Style::default(),
        children: Vec::new(),
        on_click: None,
        on_key_down: None,
        is_drag_region: false,
        scroll_y: None,
        content_height: Cell::new(0.0),
        dnd_source: None,
        dnd_target: None,
    }
}

impl Div {
    // --- Sizing ---
    pub fn w(mut self, width: f32) -> Self {
        self.taffy_style.size.width = length(width);
        self
    }

    pub fn h(mut self, height: f32) -> Self {
        self.taffy_style.size.height = length(height);
        self
    }

    pub fn size(mut self, s: f32) -> Self {
        self.taffy_style.size = Size {
            width: length(s),
            height: length(s),
        };
        self
    }

    pub fn w_full(mut self) -> Self {
        self.taffy_style.size.width = percent(1.0);
        self
    }

    pub fn h_full(mut self) -> Self {
        self.taffy_style.size.height = percent(1.0);
        self
    }

    pub fn min_w(mut self, v: f32) -> Self {
        self.taffy_style.min_size.width = length(v);
        self
    }

    pub fn min_h(mut self, v: f32) -> Self {
        self.taffy_style.min_size.height = length(v);
        self
    }

    pub fn max_w(mut self, v: f32) -> Self {
        self.taffy_style.max_size.width = length(v);
        self
    }

    pub fn max_h(mut self, v: f32) -> Self {
        self.taffy_style.max_size.height = length(v);
        self
    }

    // --- Flex direction ---
    pub fn flex_row(mut self) -> Self {
        self.taffy_style.display = Display::Flex;
        self.taffy_style.flex_direction = FlexDirection::Row;
        self
    }

    pub fn flex_col(mut self) -> Self {
        self.taffy_style.display = Display::Flex;
        self.taffy_style.flex_direction = FlexDirection::Column;
        self
    }

    // --- Flex properties ---
    pub fn flex_grow(mut self, v: f32) -> Self {
        self.taffy_style.flex_grow = v;
        self
    }

    pub fn flex_shrink(mut self, v: f32) -> Self {
        self.taffy_style.flex_shrink = v;
        self
    }

    pub fn flex_1(mut self) -> Self {
        self.taffy_style.flex_grow = 1.0;
        self.taffy_style.flex_shrink = 1.0;
        self.taffy_style.flex_basis = percent(0.0);
        self
    }

    pub fn flex_wrap(mut self) -> Self {
        self.taffy_style.flex_wrap = FlexWrap::Wrap;
        self
    }

    // --- Gap ---
    pub fn gap(mut self, v: f32) -> Self {
        self.taffy_style.gap = Size {
            width: length(v),
            height: length(v),
        };
        self
    }

    pub fn gap_x(mut self, v: f32) -> Self {
        self.taffy_style.gap.width = length(v);
        self
    }

    pub fn gap_y(mut self, v: f32) -> Self {
        self.taffy_style.gap.height = length(v);
        self
    }

    // --- Padding ---
    pub fn p(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.padding = Rect {
            left: l,
            right: l,
            top: l,
            bottom: l,
        };
        self
    }

    pub fn px(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.padding.left = l;
        self.taffy_style.padding.right = l;
        self
    }

    pub fn py(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.padding.top = l;
        self.taffy_style.padding.bottom = l;
        self
    }

    pub fn pt(mut self, v: f32) -> Self {
        self.taffy_style.padding.top = length(v);
        self
    }

    pub fn pb(mut self, v: f32) -> Self {
        self.taffy_style.padding.bottom = length(v);
        self
    }

    pub fn pl(mut self, v: f32) -> Self {
        self.taffy_style.padding.left = length(v);
        self
    }

    pub fn pr(mut self, v: f32) -> Self {
        self.taffy_style.padding.right = length(v);
        self
    }

    // --- Margin ---
    pub fn m(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.margin = Rect {
            left: l,
            right: l,
            top: l,
            bottom: l,
        };
        self
    }

    pub fn mx(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.margin.left = l;
        self.taffy_style.margin.right = l;
        self
    }

    pub fn my(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.margin.top = l;
        self.taffy_style.margin.bottom = l;
        self
    }

    pub fn mx_auto(mut self) -> Self {
        self.taffy_style.margin.left = LengthPercentageAuto::Auto;
        self.taffy_style.margin.right = LengthPercentageAuto::Auto;
        self
    }

    // --- Alignment ---
    pub fn items_start(mut self) -> Self {
        self.taffy_style.align_items = Some(AlignItems::FlexStart);
        self
    }

    pub fn items_center(mut self) -> Self {
        self.taffy_style.align_items = Some(AlignItems::Center);
        self
    }

    pub fn items_end(mut self) -> Self {
        self.taffy_style.align_items = Some(AlignItems::FlexEnd);
        self
    }

    pub fn items_stretch(mut self) -> Self {
        self.taffy_style.align_items = Some(AlignItems::Stretch);
        self
    }

    pub fn justify_start(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::FlexStart);
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::Center);
        self
    }

    pub fn justify_end(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::FlexEnd);
        self
    }

    pub fn justify_between(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::SpaceBetween);
        self
    }

    pub fn justify_around(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::SpaceAround);
        self
    }

    pub fn justify_evenly(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::SpaceEvenly);
        self
    }

    // --- Overflow ---
    pub fn overflow_hidden(mut self) -> Self {
        self.taffy_style.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Hidden,
        };
        self
    }

    /// Enable vertical scrolling. Pass a `ScrollOffset` that persists
    /// across rebuilds (create it once in your app state or with `use_scroll`).
    pub fn overflow_y_scroll(mut self, offset: ScrollOffset) -> Self {
        self.scroll_y = Some(offset);
        self.taffy_style.overflow.y = Overflow::Scroll;
        self
    }

    // --- Visual ---
    pub fn bg(mut self, fill: impl Into<Fill>) -> Self {
        self.background = Some(fill.into());
        self
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.corner_radii = Corners::uniform(radius);
        self
    }

    pub fn rounded_t(mut self, radius: f32) -> Self {
        self.corner_radii.top_left = radius;
        self.corner_radii.top_right = radius;
        self
    }

    pub fn rounded_b(mut self, radius: f32) -> Self {
        self.corner_radii.bottom_left = radius;
        self.corner_radii.bottom_right = radius;
        self
    }

    pub fn rounded_full(mut self) -> Self {
        self.corner_radii = Corners::uniform(9999.0);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    // --- Window integration ---
    /// Mark this element as a window drag region (for custom title bars).
    pub fn drag_region(mut self) -> Self {
        self.is_drag_region = true;
        self
    }

    // --- Events ---
    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn on_key_down(
        mut self,
        handler: impl Fn(mozui_events::Key, mozui_events::Modifiers, &mut dyn std::any::Any) + 'static,
    ) -> Self {
        self.on_key_down = Some(Box::new(handler));
        self
    }

    // --- Drag-and-drop ---

    /// Make this div a drag source. When the user drags from this element,
    /// compatible drop targets (registered with the same `DragId`) will accept it.
    pub fn draggable(mut self, id: DragId) -> Self {
        self.dnd_source = Some(id);
        self
    }

    /// Make this div a drop target. When a draggable with `id` is released over
    /// this element, the handler fires with `(source_id, mouse_position, cx)`.
    pub fn drop_target(
        mut self,
        id: DragId,
        handler: impl Fn(DragId, StylePoint, &mut dyn std::any::Any) + 'static,
    ) -> Self {
        self.dnd_target = Some((id, Box::new(handler)));
        self
    }

    // --- Children ---
    pub fn child(mut self, child: impl Element + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    pub fn children<I, E>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Element + 'static,
    {
        for child in children {
            self.children.push(Box::new(child));
        }
        self
    }
}

impl Element for Div {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let child_nodes: Vec<taffy::NodeId> = self
            .children
            .iter()
            .map(|c| c.layout(engine, font_system))
            .collect();

        engine.new_with_children(self.taffy_style.clone(), &child_nodes)
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &mozui_text::FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);

        // Paint self
        if let Some(ref bg) = self.background {
            let border = if self.border_width > 0.0 {
                Some(Border {
                    width: self.border_width,
                    color: self.border_color,
                })
            } else {
                None
            };

            draw_list.push(DrawCommand::Rect {
                bounds,
                background: bg.clone(),
                corner_radii: self.corner_radii,
                border,
                shadow: self.shadow,
            });
        }

        // Register click handler if present
        if let Some(ref handler) = self.on_click {
            let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            interactions.register_click(bounds, Box::new(move |cx| unsafe { (*handler_ptr)(cx) }));
        }

        // Register drag region if marked
        if self.is_drag_region {
            interactions.register_drag_region(bounds);
        }

        // Register key handler if present
        if let Some(ref handler) = self.on_key_down {
            let handler_ptr = handler.as_ref()
                as *const dyn Fn(
                    mozui_events::Key,
                    mozui_events::Modifiers,
                    &mut dyn std::any::Any,
                );
            interactions.register_key_handler(Box::new(move |key, mods, cx| unsafe {
                (*handler_ptr)(key, mods, cx)
            }));
        }

        // Register DnD source
        if let Some(id) = self.dnd_source {
            interactions.register_dnd_source(id, bounds);
        }

        // Register DnD target
        if let Some((id, ref handler)) = self.dnd_target {
            let handler_ptr = handler.as_ref()
                as *const dyn Fn(DragId, StylePoint, &mut dyn std::any::Any);
            interactions.register_drop_target(id, bounds, move |source_id, pos, cx| unsafe {
                (*handler_ptr)(source_id, pos, cx)
            });
        }

        // Scroll handling
        let scroll_offset_y = self.scroll_y.as_ref().map(|so| so.get()).unwrap_or(0.0);
        let is_scrollable = self.scroll_y.is_some();

        if is_scrollable {
            draw_list.push_scroll_offset(-scroll_offset_y);
            interactions.push_scroll_offset(-scroll_offset_y);
        }

        // Paint children — track index range to compute content bounds
        let children_start = *index;
        for child in &self.children {
            child.paint(layouts, index, draw_list, interactions, font_system);
        }
        let children_end = *index;

        if is_scrollable {
            draw_list.pop_scroll_offset();
            interactions.pop_scroll_offset();
        }

        // Register scroll region after painting children (so we know content height)
        if let Some(ref scroll) = self.scroll_y {
            let viewport_height = bounds.size.height;

            // Find the bottom of the deepest child to determine content height
            let mut content_bottom = 0.0_f32;
            for i in children_start..children_end {
                let cl = layouts[i];
                let bot = cl.y + cl.height - bounds.origin.y;
                content_bottom = content_bottom.max(bot);
            }

            let max_scroll = (content_bottom - viewport_height).max(0.0);
            self.content_height.set(content_bottom);

            // Clamp current offset
            let clamped = scroll_offset_y.clamp(0.0, max_scroll);
            if clamped != scroll_offset_y {
                scroll.set(clamped);
            }

            // Tick momentum physics each frame
            scroll.tick_momentum(max_scroll);

            let scroll_clone = scroll.clone();
            interactions.register_scroll_region(
                bounds,
                Box::new(move |_dx, dy, _cx| {
                    scroll_clone.scroll_by(dy, max_scroll);
                }),
            );
        }
    }
}
