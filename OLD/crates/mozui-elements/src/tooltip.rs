use crate::{DeferredPosition, Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Placement, Rect, Shadow, Theme};
use taffy::prelude::*;

/// A tooltip that appears when hovering over the anchor element.
///
/// The tooltip renders the anchor (trigger) element normally. When the
/// mouse is over the anchor, a floating bubble is rendered nearby via
/// the deferred element system (painted on top of the main tree).
///
/// ```rust,ignore
/// tooltip(&theme, "Copy to clipboard")
///     .placement(Placement::Top)
///     .child(icon_button(IconName::Copy, &theme))
/// ```
pub struct Tooltip {
    text: String,
    shortcut: Option<String>,
    placement: Placement,
    visible: bool,
    bg: Color,
    fg: Color,
    shadow: Shadow,
    corner_radius: f32,
    font_size: f32,
    /// The trigger/anchor element
    anchor: Option<Box<dyn Element>>,
    // Layout IDs
    anchor_id: LayoutId,
}

/// Create a tooltip with text content, wrapping an anchor element.
pub fn tooltip(theme: &Theme, text: impl Into<String>) -> Tooltip {
    Tooltip {
        text: text.into(),
        shortcut: None,
        placement: Placement::Top,
        visible: false,
        bg: theme.foreground,
        fg: theme.background,
        shadow: theme.shadow_sm,
        corner_radius: theme.radius_sm,
        font_size: theme.font_size_xs,
        anchor: None,
        anchor_id: LayoutId::NONE,
    }
}

impl Tooltip {
    /// Set the anchor/trigger element that the tooltip appears over.
    pub fn child(mut self, element: impl Element + 'static) -> Self {
        self.anchor = Some(Box::new(element));
        self
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Control visibility directly (e.g. from a hover signal).
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

impl Element for Tooltip {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Tooltip",
            layout_id: LayoutId::NONE,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Layout anchor in the main tree
        if let Some(ref mut anchor) = self.anchor {
            self.anchor_id = anchor.layout(cx);
        }

        // Defer the tooltip bubble content for independent layout + paint-on-top
        cx.defer(
            Box::new(TooltipBubble {
                text: self.text.clone(),
                shortcut: self.shortcut.clone(),
                visible: self.visible,
                bg: self.bg,
                fg: self.fg,
                shadow: self.shadow,
                corner_radius: self.corner_radius,
                font_size: self.font_size,
                layout_id: LayoutId::NONE,
                text_id: LayoutId::NONE,
                shortcut_id: LayoutId::NONE,
            }),
            DeferredPosition::Anchored {
                anchor_id: self.anchor_id,
                placement: self.placement,
                gap: 6.0,
            },
        );

        // Tooltip is layout-transparent — returns only the anchor
        self.anchor_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Paint the anchor element
        if let Some(ref mut anchor) = self.anchor {
            anchor.paint(bounds, cx);
            // Register hover region so the app re-renders on hover
            cx.interactions.register_hover_region(bounds);
        }
    }
}

// ── Deferred tooltip bubble ───────────────────────────────────────

/// The floating tooltip bubble, laid out and painted by the deferred system.
struct TooltipBubble {
    text: String,
    shortcut: Option<String>,
    visible: bool,
    bg: Color,
    fg: Color,
    shadow: Shadow,
    corner_radius: f32,
    font_size: f32,
    // Layout IDs (in the sub-engine)
    layout_id: LayoutId,
    text_id: LayoutId,
    shortcut_id: LayoutId,
}

impl Element for TooltipBubble {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "TooltipBubble",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let mut tip_children = Vec::new();

        // Text leaf
        let style = mozui_text::TextStyle {
            font_size: self.font_size,
            color: self.fg,
            ..Default::default()
        };
        let m = mozui_text::measure_text(&self.text, &style, None, cx.font_system);
        self.text_id = cx.new_leaf(Style {
            size: taffy::Size {
                width: length(m.width),
                height: length(m.height),
            },
            ..Default::default()
        });
        tip_children.push(self.text_id);

        // Optional shortcut leaf
        if let Some(ref sc) = self.shortcut {
            let sc_style = mozui_text::TextStyle {
                font_size: self.font_size,
                color: self.fg.with_alpha(0.7),
                ..Default::default()
            };
            let sc_m = mozui_text::measure_text(sc, &sc_style, None, cx.font_system);
            self.shortcut_id = cx.new_leaf(Style {
                size: taffy::Size {
                    width: length(sc_m.width),
                    height: length(sc_m.height),
                },
                margin: taffy::Rect {
                    left: length(8.0),
                    right: zero(),
                    top: zero(),
                    bottom: zero(),
                },
                ..Default::default()
            });
            tip_children.push(self.shortcut_id);
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                padding: taffy::Rect {
                    left: length(8.0),
                    right: length(8.0),
                    top: length(4.0),
                    bottom: length(4.0),
                },
                gap: taffy::Size {
                    width: length(4.0),
                    height: zero(),
                },
                ..Default::default()
            },
            &tip_children,
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Check visibility: show if explicitly visible OR anchor is hovered
        let show = self.visible
            || cx
                .anchor_bounds
                .map_or(false, |ab| cx.interactions.is_hovered(ab));
        if !show {
            return;
        }

        // Draw tooltip background
        cx.draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: None,
            shadow: Some(self.shadow), shadows: vec![],
        });

        // Draw text
        let text_bounds = cx.bounds(self.text_id);
        cx.draw_list.push(DrawCommand::Text {
            text: self.text.clone(),
            bounds: text_bounds,
            font_size: self.font_size,
            color: self.fg,
            weight: 400,
            italic: false,
        });

        // Draw shortcut if present
        if let Some(ref sc) = self.shortcut {
            let sc_bounds = cx.bounds(self.shortcut_id);
            cx.draw_list.push(DrawCommand::Text {
                text: sc.clone(),
                bounds: sc_bounds,
                font_size: self.font_size,
                color: self.fg.with_alpha(0.7),
                weight: 400,
                italic: false,
            });
        }
    }
}
