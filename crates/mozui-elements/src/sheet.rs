use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::animation::{Animated, Transition};
use mozui_style::{Color, Corners, Fill, Shadow, Theme};
use mozui_text::FontSystem;
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
pub struct Sheet {
    placement: SheetPlacement,
    size: f32,
    children: Vec<Box<dyn Element>>,
    title: Option<Box<dyn Element>>,
    footer: Option<Box<dyn Element>>,
    overlay: bool,
    on_close: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
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
        self.on_close = Some(Box::new(f));
        self
    }

    pub fn anim(mut self, anim: Animated<f32>) -> Self {
        self.anim = Some(anim);
        self
    }

    fn is_horizontal(&self) -> bool {
        matches!(self.placement, SheetPlacement::Left | SheetPlacement::Right)
    }
}

impl Element for Sheet {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        // Build content children
        let mut content_children = Vec::new();

        if let Some(ref title) = self.title {
            content_children.push(title.layout(engine, font_system));
        }

        // Body wrapper
        let body_children: Vec<_> = self
            .children
            .iter()
            .map(|c| c.layout(engine, font_system))
            .collect();
        content_children.push(engine.new_with_children(
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
            &body_children,
        ));

        if let Some(ref footer) = self.footer {
            content_children.push(footer.layout(engine, font_system));
        }

        // Panel — sized along placement axis
        let (panel_w, panel_h) = if self.is_horizontal() {
            (length(self.size), percent(1.0))
        } else {
            (percent(1.0), length(self.size))
        };

        let panel = engine.new_with_children(
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

        // Full-screen overlay container (absolute positioned)
        engine.new_with_children(
            Style {
                display: Display::Flex,
                position: Position::Absolute,
                inset: taffy::Rect {
                    left: length(0.0),
                    right: length(0.0),
                    top: length(0.0),
                    bottom: length(0.0),
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
            &[panel],
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &FontSystem,
    ) {
        let progress = self.anim.as_ref().map(|a| a.get()).unwrap_or(1.0);

        // Overlay container
        let overlay_layout = layouts[*index];
        *index += 1;
        let overlay_bounds = mozui_style::Rect::new(
            overlay_layout.x,
            overlay_layout.y,
            overlay_layout.width,
            overlay_layout.height,
        );

        // Draw overlay backdrop
        if self.overlay {
            draw_list.push(DrawCommand::Rect {
                bounds: overlay_bounds,
                background: Fill::Solid(self.overlay_color.with_alpha(self.overlay_color.a * progress)),
                corner_radii: Corners::ZERO,
                border: None,
                shadow: None,
            });

            // Block all interaction behind the overlay
            interactions.register_hover_region(overlay_bounds);
            interactions.register_drag_handler(overlay_bounds, Box::new(|_pos, _cx| {}));

            // Click overlay to close
            if let Some(ref on_close) = self.on_close {
                let ptr = on_close.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                interactions
                    .register_click(overlay_bounds, Box::new(move |cx| unsafe { (*ptr)(cx) }));
            }
        }

        // Panel
        let panel_layout = layouts[*index];
        *index += 1;

        // Slide offset based on animation progress
        let slide_offset = (1.0 - progress) * SLIDE_PX;
        let (dx, dy) = match self.placement {
            SheetPlacement::Left => (-slide_offset, 0.0),
            SheetPlacement::Right => (slide_offset, 0.0),
            SheetPlacement::Top => (0.0, -slide_offset),
            SheetPlacement::Bottom => (0.0, slide_offset),
        };

        let panel_bounds = mozui_style::Rect::new(
            panel_layout.x + dx,
            panel_layout.y + dy,
            panel_layout.width,
            panel_layout.height,
        );

        // Border on the inner edge
        let border_side = match self.placement {
            SheetPlacement::Left => Some(mozui_style::Rect::new(
                panel_bounds.origin.x + panel_bounds.size.width - 1.0,
                panel_bounds.origin.y,
                1.0,
                panel_bounds.size.height,
            )),
            SheetPlacement::Right => Some(mozui_style::Rect::new(
                panel_bounds.origin.x,
                panel_bounds.origin.y,
                1.0,
                panel_bounds.size.height,
            )),
            SheetPlacement::Top => Some(mozui_style::Rect::new(
                panel_bounds.origin.x,
                panel_bounds.origin.y + panel_bounds.size.height - 1.0,
                panel_bounds.size.width,
                1.0,
            )),
            SheetPlacement::Bottom => Some(mozui_style::Rect::new(
                panel_bounds.origin.x,
                panel_bounds.origin.y,
                panel_bounds.size.width,
                1.0,
            )),
        };

        // Prevent overlay click from firing when clicking inside panel
        interactions.register_click(panel_bounds, Box::new(|_| {}));

        // Clip and fade entire panel uniformly
        draw_list.push_clip(panel_bounds);
        draw_list.push_opacity(progress);

        // Panel background with shadow
        let shadow = if progress > 0.5 {
            Some(self.shadow)
        } else {
            None
        };
        draw_list.push(DrawCommand::Rect {
            bounds: panel_bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::ZERO,
            border: None,
            shadow,
        });

        // Border line
        if let Some(border_rect) = border_side {
            draw_list.push(DrawCommand::Rect {
                bounds: border_rect,
                background: Fill::Solid(self.border_color),
                corner_radii: Corners::ZERO,
                border: None,
                shadow: None,
            });
        }

        // Title
        if let Some(ref title) = self.title {
            title.paint(layouts, index, draw_list, interactions, font_system);
        }

        // Body
        let _body = layouts[*index];
        *index += 1;
        for child in &self.children {
            child.paint(layouts, index, draw_list, interactions, font_system);
        }

        // Footer
        if let Some(ref footer) = self.footer {
            footer.paint(layouts, index, draw_list, interactions, font_system);
        }

        draw_list.pop_opacity();
        draw_list.pop_clip();
    }
}
