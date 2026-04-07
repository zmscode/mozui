pub use taffy;
pub use taffy::prelude::*;

/// Computed layout result for an element.
#[derive(Debug, Clone, Copy, Default)]
pub struct ComputedLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A tree-based layout engine backed by Taffy.
pub struct LayoutEngine {
    taffy: TaffyTree,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
        }
    }

    /// Clear the entire tree.
    pub fn clear(&mut self) {
        self.taffy.clear();
    }

    /// Create a leaf node.
    pub fn new_leaf(&mut self, style: Style) -> NodeId {
        self.taffy.new_leaf(style).unwrap()
    }

    /// Create a node with children.
    pub fn new_with_children(&mut self, style: Style, children: &[NodeId]) -> NodeId {
        self.taffy.new_with_children(style, children).unwrap()
    }

    /// Compute layout starting from root.
    pub fn compute_layout(&mut self, root: NodeId, available: Size<AvailableSpace>) {
        self.taffy.compute_layout(root, available).unwrap();
    }

    /// Get the computed layout for a node.
    pub fn layout(&self, node: NodeId) -> ComputedLayout {
        let l = self.taffy.layout(node).unwrap();
        ComputedLayout {
            x: l.location.x,
            y: l.location.y,
            width: l.size.width,
            height: l.size.height,
        }
    }

    /// Recursively collect layouts in pre-order, resolving absolute positions.
    pub fn collect_layouts(&self, root: NodeId) -> Vec<ComputedLayout> {
        let mut results = Vec::new();
        self.collect_recursive(root, 0.0, 0.0, &mut results);
        results
    }

    fn collect_recursive(
        &self,
        node: NodeId,
        parent_x: f32,
        parent_y: f32,
        out: &mut Vec<ComputedLayout>,
    ) {
        let l = self.taffy.layout(node).unwrap();
        let x = parent_x + l.location.x;
        let y = parent_y + l.location.y;

        out.push(ComputedLayout {
            x,
            y,
            width: l.size.width,
            height: l.size.height,
        });

        if let Ok(children) = self.taffy.children(node) {
            for &child_id in children.iter() {
                self.collect_recursive(child_id, x, y, out);
            }
        }
    }
}
