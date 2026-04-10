use crate::{DeferredPosition, Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::animation::{Animated, Transition};
use mozui_style::{Color, Corners, Fill, Point, Rect, Shadow, Theme};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;
use taffy::prelude::*;

/// Animation duration in ms for sheet entrance/exit.
/// Use this when scheduling removal after exit animation:
/// `cx.set_timeout(Duration::from_millis(SHEET_ANIM_MS), ...)`
pub const SHEET_ANIM_MS: u64 = 150;
const SLIDE_PX: f32 = 100.0;
const DEFAULT_SIZE: f32 = 350.0;
const PAD: f32 = 16.0;

/// Which edge the sheet slides in from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetPlacement {
    Left,
    Right,
    Top,
    Bottom,
}

/// A slide-in panel overlay (drawer / side sheet).
///
/// Uses the deferred element system so the sheet always paints on top
/// of the main tree.
pub struct Sheet {
    placement: SheetPlacement,
    size: f32,
    children: Vec<Box<dyn Element>>,
    title: Option<Box<dyn Element>>,
    footer: Option<Box<dyn Element>>,
    overlay: bool,
    on_close: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
    bg: Color,
    border_color: Color,
    overlay_color: Color,
    shadow: Shadow,
    anim: Option<Animated<f32>>,
}

/// Create an entrance animation for a sheet.
pub fn sheet_anim(animation_flag: Rc<Cell<bool>>) -> Animated<f32> {
    let transition =
        Transition::new(Duration::from_millis(SHEET_ANIM_MS)).custom_bezier(0.4, 0.0, 0.2, 1.0);
    let anim = Animated::new(0.0, transition, animation_flag);
    anim.set(1.0);
    anim
}

pub fn sheet(placement: SheetPlacement, theme: &Theme) -> Sheet {
    Sheet {
        placement,
        size: DEFAULT_SIZE,
        children: Vec::new(),
        title: None,
        footer: None,
        overlay: true,
        on_close: None,
        bg: theme.popover,
        border_color: theme.border,
        overlay_color: Color::rgba(0, 0, 0, 0.5),
        shadow: theme.shadow_lg,
        anim: None,
    }
}

impl Sheet {
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn child(mut self, element: impl Element + 'static) -> Self {
        self.children.push(Box::new(element));
        self
    }

    pub fn title(mut self, element: impl Element + 'static) -> Self {
        self.title = Some(Box::new(element));
        self
    }

    pub fn footer(mut self, element: impl Element + 'static) -> Self {
        self.footer = Some(Box::new(element));
        self
    }

    pub fn overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }

    pub fn on_close(mut self, f: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }

    pub fn anim(mut self, anim: Animated<f32>) -> Self {
        self.anim = Some(anim);
        self
    }
}

impl Element for Sheet {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Sheet",
            layout_id: LayoutId::NONE,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Defer the entire sheet overlay for paint-on-top z-ordering
        cx.defer(
            Box::new(SheetOverlay {
                placement: self.placement,
                size: self.size,
                children: std::mem::take(&mut self.children),
                title: self.title.take(),
                footer: self.footer.take(),
                overlay: self.overlay,
                on_close: self.on_close.take(),
                bg: self.bg,
                border_color: self.border_color,
                overlay_color: self.overlay_color,
                shadow: self.shadow,
                anim: self.anim.clone(),
                layout_id: LayoutId::NONE,
                panel_id: LayoutId::NONE,
                title_id: LayoutId::NONE,
                body_id: LayoutId::NONE,
                child_ids: Vec::new(),
                footer_id: LayoutId::NONE,
            }),
            DeferredPosition::Overlay,
        );

        // Return a zero-size placeholder
        cx.new_leaf(taffy::Style::default())
    }

    fn paint(&mut self, _bounds: Rect, _cx: &mut PaintContext) {
        // Nothing — painted by the deferred system
    }
}

// ── Deferred sheet overlay ────────────────────────────────────────

struct SheetOverlay {
    placement: SheetPlacement,
    size: f32,
    children: Vec<Box<dyn Element>>,
    title: Option<Box<dyn Element>>,
    footer: Option<Box<dyn Element>>,
    overlay: bool,
    on_close: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
    bg: Color,
    border_color: Color,
    overlay_color: Color,
    shadow: Shadow,
    anim: Option<Animated<f32>>,
    // Layout IDs
    layout_id: LayoutId,
    panel_id: LayoutId,
    title_id: LayoutId,
    body_id: LayoutId,
    child_ids: Vec<LayoutId>,
    footer_id: LayoutId,
}

impl SheetOverlay {
    fn is_horizontal(&self) -> bool {
        matches!(self.placement, SheetPlacement::Left | SheetPlacement::Right)
    }
}

impl Element for SheetOverlay {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "SheetOverlay",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.child_ids.clear();

        // Build content children
        let mut content_children = Vec::new();

        if let Some(ref mut title) = self.title {
            self.title_id = title.layout(cx);
            content_children.push(self.title_id);
        }

        // Body wrapper
        for i in 0..self.children.len() {
            let id = self.children[i].layout(cx);
            self.child_ids.push(id);
        }
        self.body_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                padding: taffy::Rect {
                    left: length(PAD),
                    right: length(PAD),
                    top: length(PAD),
                    bottom: length(PAD),
                },
                overflow: taffy::Point {
                    x: taffy::Overflow::Hidden,
                    y: taffy::Overflow::Hidden,
                },
                ..Default::default()
            },
            &self.child_ids,
        );
        content_children.push(self.body_id);

        if let Some(ref mut footer) = self.footer {
            self.footer_id = footer.layout(cx);
            content_children.push(self.footer_id);
        }

        // Panel -- sized along placement axis
        let (panel_w, panel_h) = if self.is_horizontal() {
            (length(self.size), percent(1.0))
        } else {
            (percent(1.0), length(self.size))
        };

        self.panel_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: panel_w,
                    height: panel_h,
                },
                ..Default::default()
            },
            &content_children,
        );

        // Full-screen overlay container.
        // Uses percent(1.0) to fill the sub-engine's available space
        // (Position::Absolute + inset doesn't work at sub-engine root).
        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                // Align panel to the correct edge
                justify_content: Some(match self.placement {
                    SheetPlacement::Left => JustifyContent::FlexStart,
                    SheetPlacement::Right => JustifyContent::FlexEnd,
                    SheetPlacement::Top | SheetPlacement::Bottom => JustifyContent::Center,
                }),
                align_items: Some(match self.placement {
                    SheetPlacement::Top => AlignItems::FlexStart,
                    SheetPlacement::Bottom => AlignItems::FlexEnd,
                    SheetPlacement::Left | SheetPlacement::Right => AlignItems::Stretch,
                }),
                ..Default::default()
            },
            &[self.panel_id],
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let progress = self.anim.as_ref().map(|a| a.get()).unwrap_or(1.0);

        // Overlay container
        let overlay_bounds = bounds;

        // Draw overlay backdrop
        if self.overlay {
            cx.draw_list.push(DrawCommand::Rect {
                bounds: overlay_bounds,
                background: Fill::Solid(
                    self.overlay_color
                        .with_alpha(self.overlay_color.a * progress),
                ),
                corner_radii: Corners::ZERO,
                border: None,
                shadow: None, shadows: vec![],
            });

            // Block all interaction behind the overlay
            cx.interactions.register_hover_region(overlay_bounds);
            cx.interactions
                .register_drag_handler(overlay_bounds, Rc::new(|_pos: Point, _cx: &mut dyn std::any::Any| {}));

            // Click overlay to close
            if let Some(ref on_close) = self.on_close {
                cx.interactions
                    .register_click(overlay_bounds, on_close.clone());
            }
        }

        // Panel
        let panel_layout = cx.engine.bounds(self.panel_id);

        // Slide offset based on animation progress
        let slide_offset = (1.0 - progress) * SLIDE_PX;
        let (dx, dy) = match self.placement {
            SheetPlacement::Left => (-slide_offset, 0.0),
            SheetPlacement::Right => (slide_offset, 0.0),
            SheetPlacement::Top => (0.0, -slide_offset),
            SheetPlacement::Bottom => (0.0, slide_offset),
        };

        let panel_bounds = Rect::new(
            panel_layout.x + dx,
            panel_layout.y + dy,
            panel_layout.width,
            panel_layout.height,
        );

        // Border on the inner edge
        let border_side = match self.placement {
            SheetPlacement::Left => Some(Rect::new(
                panel_bounds.origin.x + panel_bounds.size.width - 1.0,
                panel_bounds.origin.y,
                1.0,
                panel_bounds.size.height,
            )),
            SheetPlacement::Right => Some(Rect::new(
                panel_bounds.origin.x,
                panel_bounds.origin.y,
                1.0,
                panel_bounds.size.height,
            )),
            SheetPlacement::Top => Some(Rect::new(
                panel_bounds.origin.x,
                panel_bounds.origin.y + panel_bounds.size.height - 1.0,
                panel_bounds.size.width,
                1.0,
            )),
            SheetPlacement::Bottom => Some(Rect::new(
                panel_bounds.origin.x,
                panel_bounds.origin.y,
                panel_bounds.size.width,
                1.0,
            )),
        };

        // Prevent overlay click from firing when clicking inside panel
        cx.interactions
            .register_click(panel_bounds, Rc::new(|_: &mut dyn std::any::Any| {}));

        // Clip and fade entire panel uniformly
        cx.draw_list.push_clip(panel_bounds);
        cx.draw_list.push_opacity(progress);

        // Panel background with shadow
        let shadow = if progress > 0.5 {
            Some(self.shadow)
        } else {
            None
        };
        cx.draw_list.push(DrawCommand::Rect {
            bounds: panel_bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::ZERO,
            border: None,
            shadow,
            shadows: vec![],
        });

        // Border line
        if let Some(border_rect) = border_side {
            cx.draw_list.push(DrawCommand::Rect {
                bounds: border_rect,
                background: Fill::Solid(self.border_color),
                corner_radii: Corners::ZERO,
                border: None,
                shadow: None, shadows: vec![],
            });
        }

        // Title
        if let Some(ref mut title) = self.title {
            let title_bounds = cx.bounds(self.title_id);
            title.paint(title_bounds, cx);
        }

        // Body
        for i in 0..self.children.len() {
            let child_bounds = cx.bounds(self.child_ids[i]);
            self.children[i].paint(child_bounds, cx);
        }

        // Footer
        if let Some(ref mut footer) = self.footer {
            let footer_bounds = cx.bounds(self.footer_id);
            footer.paint(footer_bounds, cx);
        }

        cx.draw_list.pop_opacity();
        cx.draw_list.pop_clip();
    }
}
