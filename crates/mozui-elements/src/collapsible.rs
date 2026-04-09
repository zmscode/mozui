use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_style::Rect;
use std::cell::Cell;
use taffy::Overflow;
use taffy::prelude::*;

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
    layout_id: LayoutId,
    inner_id: LayoutId,
    child_ids: Vec<LayoutId>,

    height_factor: f32,
    children: Vec<Box<dyn Element>>,
    /// Remembers the full content height for max_size calculation.
    content_height: Cell<f32>,
}

pub fn collapsible(height_factor: f32) -> CollapsibleContainer {
    CollapsibleContainer {
        layout_id: LayoutId::NONE,
        inner_id: LayoutId::NONE,
        child_ids: Vec::new(),

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
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "CollapsibleContainer",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Always lay out children so we get a stable node count and
        // can measure content height even when collapsed.
        self.child_ids = self
            .children
            .iter_mut()
            .map(|c| c.layout(cx))
            .collect();

        self.inner_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &self.child_ids,
        );

        let max_height = if self.height_factor >= 0.999 {
            auto()
        } else {
            let content_h = self.content_height.get();
            length(content_h * self.height_factor)
        };

        self.layout_id = cx.new_with_children(
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
            &[self.inner_id],
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        // Inner container — always measure its height
        let inner_bounds = cx.bounds(self.inner_id);
        self.content_height.set(inner_bounds.size.height);

        // Clip children to the outer container's visible bounds
        cx.draw_list.push_clip(bounds);

        for i in 0..self.children.len() {
            let child_bounds = cx.bounds(self.child_ids[i]);
            self.children[i].paint(child_bounds, cx);
        }

        cx.draw_list.pop_clip();
    }
}
