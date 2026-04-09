use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Placement, Shadow, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

/// A tooltip that appears when hovering over the anchor element.
///
/// The tooltip renders the anchor (trigger) element normally, and when the
/// mouse is over the anchor, renders the tooltip content nearby.
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
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut children = Vec::new();

        // Anchor element
        if let Some(ref anchor) = self.anchor {
            children.push(anchor.layout(engine, font_system));
        }

        // Tooltip content (always laid out for stable node count)
        let mut tip_children = Vec::new();

        let style = mozui_text::TextStyle {
            font_size: self.font_size,
            color: self.fg,
            ..Default::default()
        };
        let m = mozui_text::measure_text(&self.text, &style, None, font_system);
        tip_children.push(engine.new_leaf(Style {
            size: taffy::Size {
                width: length(m.width),
                height: length(m.height),
            },
            ..Default::default()
        }));

        if let Some(ref sc) = self.shortcut {
            let sc_style = mozui_text::TextStyle {
                font_size: self.font_size,
                color: self.fg.with_alpha(0.7),
                ..Default::default()
            };
            let sc_m = mozui_text::measure_text(sc, &sc_style, None, font_system);
            tip_children.push(engine.new_leaf(Style {
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
            }));
        }

        let tip_node = engine.new_with_children(
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
        children.push(tip_node);

        // Wrapper
        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &children,
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
        // Wrapper
        let _wrapper = layouts[*index];
        *index += 1;

        // Paint anchor element
        let anchor_start_index = *index;
        if let Some(ref anchor) = self.anchor {
            anchor.paint(layouts, index, draw_list, interactions, font_system);
        }

        // Get anchor bounds for hover detection and tooltip positioning
        let anchor_layout = layouts[anchor_start_index];
        let anchor_bounds = mozui_style::Rect::new(
            anchor_layout.x,
            anchor_layout.y,
            anchor_layout.width,
            anchor_layout.height,
        );

        // Tooltip bubble
        let tip_layout = layouts[*index];
        *index += 1;

        // Tooltip text
        let text_layout = layouts[*index];
        *index += 1;

        // Shortcut (if present)
        let shortcut_layout = if self.shortcut.is_some() {
            let sl = layouts[*index];
            *index += 1;
            Some(sl)
        } else {
            None
        };

        // Register anchor as hover region so app loop re-renders on hover
        interactions.register_hover_region(anchor_bounds);

        // Only show tooltip on hover or when explicitly visible
        let show = self.visible || interactions.is_hovered(anchor_bounds);
        if !show {
            return;
        }

        // Calculate tooltip position relative to anchor
        let gap = 6.0;
        let tip_w = tip_layout.width;
        let tip_h = tip_layout.height;

        let (tip_x, tip_y) = match self.placement {
            Placement::Top => (
                anchor_bounds.origin.x + (anchor_bounds.size.width - tip_w) / 2.0,
                anchor_bounds.origin.y - tip_h - gap,
            ),
            Placement::Bottom => (
                anchor_bounds.origin.x + (anchor_bounds.size.width - tip_w) / 2.0,
                anchor_bounds.origin.y + anchor_bounds.size.height + gap,
            ),
            Placement::Left => (
                anchor_bounds.origin.x - tip_w - gap,
                anchor_bounds.origin.y + (anchor_bounds.size.height - tip_h) / 2.0,
            ),
            Placement::Right => (
                anchor_bounds.origin.x + anchor_bounds.size.width + gap,
                anchor_bounds.origin.y + (anchor_bounds.size.height - tip_h) / 2.0,
            ),
        };

        let tip_bounds = mozui_style::Rect::new(tip_x, tip_y, tip_w, tip_h);

        // Draw tooltip background
        draw_list.push(DrawCommand::Rect {
            bounds: tip_bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: None,
            shadow: Some(self.shadow),
        });

        // Draw tooltip text at computed position
        let text_offset_x = tip_x + (text_layout.x - tip_layout.x);
        let text_offset_y = tip_y + (text_layout.y - tip_layout.y);
        draw_list.push(DrawCommand::Text {
            text: self.text.clone(),
            bounds: mozui_style::Rect::new(
                text_offset_x,
                text_offset_y,
                text_layout.width,
                text_layout.height,
            ),
            font_size: self.font_size,
            color: self.fg,
            weight: 400,
            italic: false,
        });

        // Draw shortcut if present
        if let (Some(sc), Some(sc_l)) = (&self.shortcut, shortcut_layout) {
            let sc_offset_x = tip_x + (sc_l.x - tip_layout.x);
            let sc_offset_y = tip_y + (sc_l.y - tip_layout.y);
            draw_list.push(DrawCommand::Text {
                text: sc.clone(),
                bounds: mozui_style::Rect::new(
                    sc_offset_x,
                    sc_offset_y,
                    sc_l.width,
                    sc_l.height,
                ),
                font_size: self.font_size,
                color: self.fg.with_alpha(0.7),
                weight: 400,
                italic: false,
            });
        }
    }
}
