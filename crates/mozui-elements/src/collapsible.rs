use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::DrawList;
use mozui_text::FontSystem;
use std::cell::Cell;
use taffy::prelude::*;
use taffy::Overflow;

/// A container that shows or hides its content based on a height factor.
///
/// `height_factor` of 0.0 = fully collapsed, 1.0 = fully open.
/// Animate this with `cx.use_animated()` for smooth expand/collapse.
///
/// ```rust,ignore
/// let open_anim = cx.use_animated(0.0f32, Transition::new(theme.transition_normal));
/// if is_open { open_anim.set(1.0); } else { open_anim.set(0.0); }
///
/// collapsible(open_anim.get())
///     .child(label("Content that expands/collapses"))
/// ```
pub struct CollapsibleContainer {
    height_factor: f32,
    children: Vec<Box<dyn Element>>,
    /// Remembers the full content height for max_size calculation.
    content_height: Cell<f32>,
}

pub fn collapsible(height_factor: f32) -> CollapsibleContainer {
    CollapsibleContainer {
        height_factor: height_factor.clamp(0.0, 1.0),
        children: Vec::new(),
        content_height: Cell::new(0.0),
    }
}

impl CollapsibleContainer {
    pub fn child(mut self, element: impl Element + 'static) -> Self {
        self.children.push(Box::new(element));
        self
    }

    pub fn children(mut self, elements: impl IntoIterator<Item = Box<dyn Element>>) -> Self {
        self.children.extend(elements);
        self
    }
}

impl Element for CollapsibleContainer {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        // Always lay out children so we get a stable node count and
        // can measure content height even when collapsed.
        let child_nodes: Vec<taffy::NodeId> = self
            .children
            .iter()
            .map(|c| c.layout(engine, font_system))
            .collect();

        let inner = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_nodes,
        );

        let max_height = if self.height_factor >= 0.999 {
            auto()
        } else {
            let content_h = self.content_height.get();
            length(content_h * self.height_factor)
        };

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                overflow: taffy::Point {
                    x: Overflow::Hidden,
                    y: Overflow::Hidden,
                },
                max_size: Size {
                    width: auto(),
                    height: max_height,
                },
                ..Default::default()
            },
            &[inner],
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
        // Outer clipping container
        let outer = layouts[*index];
        *index += 1;

        // Inner container — always measure its height
        let inner = layouts[*index];
        *index += 1;
        self.content_height.set(inner.height);

        // Clip children to the outer container's visible bounds
        let clip_rect = mozui_style::Rect::new(outer.x, outer.y, outer.width, outer.height);
        draw_list.push_clip(clip_rect);

        for child in &self.children {
            child.paint(layouts, index, draw_list, interactions, font_system);
        }

        draw_list.pop_clip();
    }
}
