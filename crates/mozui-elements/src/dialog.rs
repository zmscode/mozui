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

/// Animation duration in ms for dialog entrance/exit.
/// Use this when scheduling removal after exit animation:
/// `cx.set_timeout(Duration::from_millis(DIALOG_ANIM_MS), ...)`
pub const DIALOG_ANIM_MS: u64 = 200;

/// A modal dialog overlay with backdrop, centered content, focus trap,
/// and optional escape/backdrop-click to dismiss.
///
/// ```rust,ignore
/// dialog(&theme)
///     .on_dismiss(move |cx| { /* close dialog */ })
///     .child(
///         div().flex_col().gap(12.0).p(24.0)
///             .child(label("Are you sure?").size(ComponentSize::Large))
///             .child(button("Confirm").primary().on_click(|cx| { /* ... */ }))
///     )
/// ```
pub struct Dialog {
    children: Vec<Box<dyn Element>>,
    on_dismiss: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    dismiss_on_backdrop: bool,
    dismiss_on_escape: bool,
    backdrop_color: Color,
    bg: Color,
    _fg: Color,
    border_color: Color,
    shadow: Shadow,
    corner_radius: f32,
    max_width: f32,
    /// Baked-in animation: 0.0 = hidden, 1.0 = fully visible.
    anim: Animated<f32>,
    no_anim: bool,
}

/// Create a new entrance animation for a dialog.
/// Store this in your state and pass it to the dialog builder via `.anim()`.
/// Call `.set(0.0)` to trigger the exit animation.
pub fn dialog_anim(animation_flag: Rc<Cell<bool>>) -> Animated<f32> {
    let transition = Transition::new(Duration::from_millis(DIALOG_ANIM_MS))
        .custom_bezier(0.4, 0.0, 0.2, 1.0);
    let anim = Animated::new(0.0, transition, animation_flag);
    anim.set(1.0); // start entrance animation
    anim
}

/// Create a modal dialog with optional animation support.
///
/// By default the dialog appears instantly. Call `.anim(handle)` to attach
/// a persisted animation handle from `dialog_anim()`.
pub fn dialog(theme: &Theme) -> Dialog {
    let dummy_flag = Rc::new(Cell::new(false));
    let anim = Animated::new(1.0, Transition::new(Duration::ZERO), dummy_flag);
    Dialog {
        children: Vec::new(),
        on_dismiss: None,
        dismiss_on_backdrop: true,
        dismiss_on_escape: true,
        backdrop_color: theme.overlay,
        bg: theme.popover,
        _fg: theme.popover_foreground,
        border_color: theme.border,
        shadow: theme.shadow_lg,
        corner_radius: theme.radius_lg,
        max_width: 480.0,
        anim,
        no_anim: true,
    }
}

impl Dialog {
    pub fn child(mut self, element: impl Element + 'static) -> Self {
        self.children.push(Box::new(element));
        self
    }

    pub fn children(mut self, elements: impl IntoIterator<Item = Box<dyn Element>>) -> Self {
        self.children.extend(elements);
        self
    }

    /// Handler called when the dialog should be dismissed (Escape or backdrop click).
    pub fn on_dismiss(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    pub fn dismiss_on_backdrop(mut self, v: bool) -> Self {
        self.dismiss_on_backdrop = v;
        self
    }

    pub fn dismiss_on_escape(mut self, v: bool) -> Self {
        self.dismiss_on_escape = v;
        self
    }

    pub fn max_width(mut self, v: f32) -> Self {
        self.max_width = v;
        self
    }

    pub fn backdrop_color(mut self, color: Color) -> Self {
        self.backdrop_color = color;
        self
    }

    /// Attach a persisted animation handle (from `dialog_anim()`).
    pub fn anim(mut self, anim: Animated<f32>) -> Self {
        self.anim = anim;
        self.no_anim = false;
        self
    }

    /// Disable entrance/exit animations. The dialog appears instantly.
    pub fn no_anim(mut self) -> Self {
        self.no_anim = true;
        self.anim.set_immediate(1.0);
        self
    }
}

impl Element for Dialog {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        // Build content children
        let child_nodes: Vec<taffy::NodeId> = self
            .children
            .iter()
            .map(|c| c.layout(engine, font_system))
            .collect();

        // Content panel
        let content = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                max_size: taffy::Size {
                    width: length(self.max_width),
                    height: auto(),
                },
                ..Default::default()
            },
            &child_nodes,
        );

        // Full-screen backdrop that centers the content
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
                justify_content: Some(JustifyContent::Center),
                align_items: Some(AlignItems::Center),
                ..Default::default()
            },
            &[content],
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
        let progress = if self.no_anim { 1.0 } else { self.anim.get() };
        let fade = |c: Color| -> Color { c.with_alpha(c.a * progress) };

        // Backdrop
        let backdrop_layout = layouts[*index];
        *index += 1;
        let backdrop_bounds = mozui_style::Rect::new(
            backdrop_layout.x,
            backdrop_layout.y,
            backdrop_layout.width,
            backdrop_layout.height,
        );

        // Draw backdrop with animated opacity
        draw_list.push(DrawCommand::Rect {
            bounds: backdrop_bounds,
            background: Fill::Solid(fade(self.backdrop_color)),
            corner_radii: Corners::ZERO,
            border: None,
            shadow: None,
        });

        // Content panel
        let content_layout = layouts[*index];
        *index += 1;

        // gpui-component: content scales from 0.95→1.0 during entrance
        let scale = 0.95 + 0.05 * progress;
        let cx = content_layout.x + content_layout.width / 2.0;
        let cy = content_layout.y + content_layout.height / 2.0;
        let scaled_w = content_layout.width * scale;
        let scaled_h = content_layout.height * scale;
        let content_bounds = mozui_style::Rect::new(
            cx - scaled_w / 2.0,
            cy - scaled_h / 2.0,
            scaled_w,
            scaled_h,
        );

        // Draw content background with shadow (shadow hidden during animation)
        let shadow = if progress < 0.85 {
            None
        } else {
            Some(self.shadow)
        };
        draw_list.push(DrawCommand::Rect {
            bounds: content_bounds,
            background: Fill::Solid(fade(self.bg)),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(mozui_renderer::Border {
                width: 1.0,
                color: fade(self.border_color),
            }),
            shadow,
        });

        // Push focus trap so Tab stays within dialog
        interactions.push_focus_trap();

        // Register escape key handler
        if self.dismiss_on_escape {
            if let Some(ref handler) = self.on_dismiss {
                let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                interactions.register_key_handler(Box::new(move |key, _mods, cx| {
                    if key == mozui_events::Key::Escape {
                        unsafe { (*handler_ptr)(cx) };
                    }
                }));
            }
        }

        // Register backdrop click handler (dismiss on clicking outside content)
        if self.dismiss_on_backdrop {
            if let Some(ref handler) = self.on_dismiss {
                let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                // Register click on backdrop area. We register the content area click first
                // as a no-op to prevent backdrop click from triggering when clicking content.
                interactions
                    .register_click(content_bounds, Box::new(move |_cx| { /* absorb click */ }));
                interactions.register_click(
                    backdrop_bounds,
                    Box::new(move |cx| unsafe { (*handler_ptr)(cx) }),
                );
            }
        }

        // Paint children
        for child in &self.children {
            child.paint(layouts, index, draw_list, interactions, font_system);
        }

        interactions.pop_focus_trap();
    }
}
