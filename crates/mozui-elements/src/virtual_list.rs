use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::DrawList;
use mozui_style::{Corners, Fill, Rect, Size};
use mozui_text::FontSystem;
use taffy::NodeId;

/// Direction of a virtual list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualListDirection {
    Vertical,
    Horizontal,
}

/// A virtualized list that only renders visible items.
///
/// Instead of creating Element nodes for every item, it calculates which
/// items are visible based on the scroll offset and only renders those.
pub struct VirtualList {
    direction: VirtualListDirection,
    item_count: usize,
    item_height: f32, // uniform item height (for now)
    scroll_offset: f32,
    viewport_size: Size,
    render_item: Box<dyn Fn(usize) -> Box<dyn Element>>,
    background: Option<Fill>,
    corner_radii: Corners,
}

impl VirtualList {
    /// Create a vertical virtual list with uniform item height.
    pub fn vertical(
        item_count: usize,
        item_height: f32,
        viewport_size: Size,
        render_item: impl Fn(usize) -> Box<dyn Element> + 'static,
    ) -> Self {
        Self {
            direction: VirtualListDirection::Vertical,
            item_count,
            item_height,
            scroll_offset: 0.0,
            viewport_size,
            render_item: Box::new(render_item),
            background: None,
            corner_radii: Corners::ZERO,
        }
    }

    /// Create a horizontal virtual list with uniform item width.
    pub fn horizontal(
        item_count: usize,
        item_width: f32,
        viewport_size: Size,
        render_item: impl Fn(usize) -> Box<dyn Element> + 'static,
    ) -> Self {
        Self {
            direction: VirtualListDirection::Horizontal,
            item_count,
            item_height: item_width,
            scroll_offset: 0.0,
            viewport_size,
            render_item: Box::new(render_item),
            background: None,
            corner_radii: Corners::ZERO,
        }
    }

    pub fn scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn bg(mut self, fill: impl Into<Fill>) -> Self {
        self.background = Some(fill.into());
        self
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.corner_radii = Corners::uniform(radius);
        self
    }

    /// Total content size along the scroll axis.
    pub fn total_content_size(&self) -> f32 {
        self.item_count as f32 * self.item_height
    }

    /// Maximum scroll offset.
    pub fn max_scroll_offset(&self) -> f32 {
        let viewport_extent = match self.direction {
            VirtualListDirection::Vertical => self.viewport_size.height,
            VirtualListDirection::Horizontal => self.viewport_size.width,
        };
        (self.total_content_size() - viewport_extent).max(0.0)
    }

    /// Calculate the range of visible item indices.
    pub fn visible_range(&self) -> (usize, usize) {
        let viewport_extent = match self.direction {
            VirtualListDirection::Vertical => self.viewport_size.height,
            VirtualListDirection::Horizontal => self.viewport_size.width,
        };

        let offset = self.scroll_offset.clamp(0.0, self.max_scroll_offset());
        let first = (offset / self.item_height).floor() as usize;
        let visible_count = (viewport_extent / self.item_height).ceil() as usize + 1;
        let last = (first + visible_count).min(self.item_count);

        (first, last)
    }

    /// Scroll to make a specific item visible.
    pub fn scroll_to_item(&mut self, index: usize) {
        let viewport_extent = match self.direction {
            VirtualListDirection::Vertical => self.viewport_size.height,
            VirtualListDirection::Horizontal => self.viewport_size.width,
        };

        let item_start = index as f32 * self.item_height;
        let item_end = item_start + self.item_height;

        if item_start < self.scroll_offset {
            self.scroll_offset = item_start;
        } else if item_end > self.scroll_offset + viewport_extent {
            self.scroll_offset = item_end - viewport_extent;
        }
    }
}

impl Element for VirtualList {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> NodeId {
        // The virtual list itself occupies the viewport size
        let style = taffy::Style {
            size: taffy::Size {
                width: taffy::Dimension::Length(self.viewport_size.width),
                height: taffy::Dimension::Length(self.viewport_size.height),
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Hidden,
                y: taffy::Overflow::Hidden,
            },
            ..Default::default()
        };

        // Only create layout nodes for visible items
        let (first, last) = self.visible_range();
        let mut children = Vec::new();
        for idx in first..last {
            let item = (self.render_item)(idx);
            children.push(item.layout(engine, font_system));
        }

        engine.new_with_children(style, &children)
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &FontSystem,
    ) {
        if *index >= layouts.len() {
            return;
        }

        let layout = layouts[*index];
        *index += 1;

        // Paint background
        if let Some(ref bg) = self.background {
            draw_list.push(mozui_renderer::DrawCommand::Rect {
                bounds: Rect::new(layout.x, layout.y, layout.width, layout.height),
                background: bg.clone(),
                corner_radii: self.corner_radii,
                border: None,
                    shadow: None,
                });
        }

        // Paint visible items
        let (first, last) = self.visible_range();
        for idx in first..last {
            let item = (self.render_item)(idx);
            item.paint(layouts, index, draw_list, interactions, font_system);
        }
    }
}
