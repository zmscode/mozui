use crate::{DeferredPosition, Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Placement, Rect, Shadow, Theme};
use taffy::prelude::*;

/// A rich content popover triggered by hovering over a trigger element.
///
/// Unlike Tooltip (which shows plain text), HoverCard renders arbitrary
/// child content in a card-styled popover. Uses the deferred element
/// system so the card paints on top of the main tree.
///
/// ```rust,ignore
/// hover_card(&theme)
///     .trigger(avatar(&theme).initials("JD"))
///     .content(
///         div().flex_col().gap(8.0)
///             .child(label("John Doe").bold())
///             .child(label("Software Engineer").font_size(12.0))
///     )
/// ```
pub struct HoverCard {
    trigger: Option<Box<dyn Element>>,
    content: Option<Box<dyn Element>>,
    placement: Placement,
    visible: bool,
    bg: Color,
    border_color: Color,
    shadow: Shadow,
    corner_radius: f32,
    padding: f32,
    // Layout IDs
    trigger_id: LayoutId,
}

pub fn hover_card(theme: &Theme) -> HoverCard {
    HoverCard {
        trigger: None,
        content: None,
        placement: Placement::Bottom,
        visible: false,
        bg: theme.surface,
        border_color: theme.border,
        shadow: theme.shadow_md,
        corner_radius: theme.radius_md,
        padding: 12.0,
        trigger_id: LayoutId::NONE,
    }
}

impl HoverCard {
    /// Set the trigger element that activates the card on hover.
    pub fn trigger(mut self, element: impl Element + 'static) -> Self {
        self.trigger = Some(Box::new(element));
        self
    }

    /// Set the content element displayed inside the card.
    pub fn content(mut self, element: impl Element + 'static) -> Self {
        self.content = Some(Box::new(element));
        self
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Directly control visibility (e.g. from a signal).
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

impl Element for HoverCard {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "HoverCard",
            layout_id: LayoutId::NONE,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Layout trigger in the main tree
        if let Some(ref mut trigger) = self.trigger {
            self.trigger_id = trigger.layout(cx);
        }

        // Defer the card content for independent layout + paint-on-top
        if let Some(content) = self.content.take() {
            cx.defer(
                Box::new(HoverCardPanel {
                    content,
                    visible: self.visible,
                    bg: self.bg,
                    border_color: self.border_color,
                    shadow: self.shadow,
                    corner_radius: self.corner_radius,
                    padding: self.padding,
                    wrapper_id: LayoutId::NONE,
                    content_inner_id: LayoutId::NONE,
                }),
                DeferredPosition::Anchored {
                    anchor_id: self.trigger_id,
                    placement: self.placement,
                    gap: 8.0,
                },
            );
        }

        // HoverCard is layout-transparent — returns only the trigger
        self.trigger_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Paint trigger
        if let Some(ref mut trigger) = self.trigger {
            trigger.paint(bounds, cx);
            // Register hover region for card trigger
            cx.interactions.register_hover_region(bounds);
        }
    }
}

// ── Deferred hover card panel ─────────────────────────────────────

/// The floating card panel, laid out and painted by the deferred system.
struct HoverCardPanel {
    content: Box<dyn Element>,
    visible: bool,
    bg: Color,
    border_color: Color,
    shadow: Shadow,
    corner_radius: f32,
    padding: f32,
    // Layout IDs (in the sub-engine)
    wrapper_id: LayoutId,
    content_inner_id: LayoutId,
}

impl Element for HoverCardPanel {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "HoverCardPanel",
            layout_id: self.wrapper_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.content_inner_id = self.content.layout(cx);
        let p = self.padding;
        self.wrapper_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: taffy::Rect {
                    left: length(p),
                    right: length(p),
                    top: length(p),
                    bottom: length(p),
                },
                ..Default::default()
            },
            &[self.content_inner_id],
        );
        self.wrapper_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Check visibility: show if explicitly visible OR anchor/card is hovered
        let anchor_hovered = cx
            .anchor_bounds
            .map_or(false, |ab| cx.interactions.is_hovered(ab));
        let card_hovered = cx.interactions.is_hovered(bounds);
        let show = self.visible || anchor_hovered || card_hovered;
        if !show {
            return;
        }

        // Register hover on the card so it stays open when cursor moves to it
        cx.interactions.register_hover_region(bounds);

        // Draw card background
        cx.draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
            shadow: Some(self.shadow),
        });

        // Paint content
        let content_bounds = cx.bounds(self.content_inner_id);
        self.content.paint(content_bounds, cx);
    }
}
