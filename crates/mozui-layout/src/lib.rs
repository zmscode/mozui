#![forbid(unsafe_code)]

pub mod cache;

pub use taffy;
pub use taffy::prelude::*;

use mozui_text::FontSystem;

/// Computed layout result for an element.
#[derive(Debug, Clone, Copy, Default)]
pub struct ComputedLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Opaque handle to a layout node. Maps to an internal index in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutId(pub(crate) usize);

impl LayoutId {
    /// Sentinel value for "no layout".
    pub const NONE: Self = Self(usize::MAX);

    /// Create a LayoutId from a raw index (for debugging).
    pub fn from_raw(index: usize) -> Self {
        Self(index)
    }
}

impl Default for LayoutId {
    fn default() -> Self {
        Self::NONE
    }
}

/// Context attached to taffy leaf nodes that need constraint-aware measurement.
///
/// Instead of pre-computing a fixed size during layout, elements can register
/// a `MeasureContext` and let taffy call the measure function during solving
/// with the actual available space constraints.
pub enum MeasureContext {
    /// Text that needs shaping with a width constraint for wrapping.
    Text {
        text: String,
        style: mozui_text::TextStyle,
    },
    /// Custom measure function for user-defined leaf sizing.
    Custom(Box<dyn Fn(taffy::Size<Option<f32>>, taffy::Size<AvailableSpace>) -> taffy::Size<f32>>),
}

/// A tree-based layout engine backed by Taffy.
///
/// Uses `TaffyTree<MeasureContext>` to support lazy, constraint-aware leaf
/// measurement. After `compute_layout`, resolved absolute bounds are available
/// via `bounds(LayoutId)` — no more flat-array index tracking.
pub struct LayoutEngine {
    taffy: TaffyTree<MeasureContext>,
    /// LayoutId.0 → NodeId mapping.
    nodes: Vec<NodeId>,
    /// LayoutId.0 → resolved absolute bounds.
    resolved: Vec<ComputedLayout>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            nodes: Vec::new(),
            resolved: Vec::new(),
        }
    }

    /// Clear the entire tree for re-use.
    pub fn clear(&mut self) {
        self.taffy.clear();
        self.nodes.clear();
        self.resolved.clear();
    }

    /// Prepare for a new frame. Clears resolved bounds but preserves
    /// taffy nodes so cached LayoutIds remain valid across frames.
    pub fn begin_frame(&mut self) {
        self.resolved.clear();
    }

    /// Number of allocated layout nodes. Useful for monitoring growth
    /// and deciding when to trigger a full `clear()`.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Allocate a LayoutId for a taffy NodeId.
    fn alloc(&mut self, node: NodeId) -> LayoutId {
        let id = LayoutId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    /// Create a leaf node with fixed size (no measure function).
    pub fn new_leaf(&mut self, style: Style) -> LayoutId {
        let node = self.taffy.new_leaf(style).unwrap();
        self.alloc(node)
    }

    /// Create a leaf node with a measure context for lazy, constraint-aware sizing.
    pub fn new_measured_leaf(&mut self, style: Style, context: MeasureContext) -> LayoutId {
        let node = self.taffy.new_leaf_with_context(style, context).unwrap();
        self.alloc(node)
    }

    /// Create a node with children.
    pub fn new_with_children(&mut self, style: Style, children: &[LayoutId]) -> LayoutId {
        let child_nodes: Vec<NodeId> = children.iter().map(|id| self.nodes[id.0]).collect();
        let node = self.taffy.new_with_children(style, &child_nodes).unwrap();
        self.alloc(node)
    }

    /// Compute layout starting from root, then resolve all absolute bounds.
    pub fn compute_layout(
        &mut self,
        root: LayoutId,
        available: Size<AvailableSpace>,
        font_system: &FontSystem,
    ) {
        let root_node = self.nodes[root.0];
        self.taffy
            .compute_layout_with_measure(
                root_node,
                available,
                |known, space, _node_id, ctx, _style| {
                    let Some(ctx) = ctx else {
                        return taffy::Size::ZERO;
                    };
                    match ctx {
                        MeasureContext::Text { text, style } => {
                            measure_text_for_taffy(text, style, known, space, font_system)
                        }
                        MeasureContext::Custom(f) => f(known, space),
                    }
                },
            )
            .unwrap();

        // Resolve absolute positions for all registered nodes
        self.resolve_bounds(root_node, 0.0, 0.0);
    }

    /// Walk the taffy tree and fill `resolved` with absolute positions.
    fn resolve_bounds(&mut self, root: NodeId, offset_x: f32, offset_y: f32) {
        // Resize resolved to match nodes length
        self.resolved.resize(self.nodes.len(), ComputedLayout::default());

        self.resolve_recursive(root, offset_x, offset_y);
    }

    fn resolve_recursive(&mut self, node: NodeId, parent_x: f32, parent_y: f32) {
        let l = self.taffy.layout(node).unwrap();
        let x = parent_x + l.location.x;
        let y = parent_y + l.location.y;
        let layout = ComputedLayout {
            x,
            y,
            width: l.size.width,
            height: l.size.height,
        };

        // Find the LayoutId for this node and store resolved bounds
        if let Some(idx) = self.nodes.iter().position(|n| *n == node) {
            self.resolved[idx] = layout;
        }

        if let Ok(children) = self.taffy.children(node) {
            for &child_id in children.iter() {
                self.resolve_recursive(child_id, x, y);
            }
        }
    }

    /// Look up the resolved absolute bounds for a layout node.
    pub fn bounds(&self, id: LayoutId) -> ComputedLayout {
        if id == LayoutId::NONE {
            return ComputedLayout::default();
        }
        self.resolved[id.0]
    }

    /// Get the content size of a node (total size of children, may exceed the node's own size
    /// if overflow is set to scroll/hidden).
    pub fn content_size(&self, id: LayoutId) -> (f32, f32) {
        let node = self.nodes[id.0];
        let l = self.taffy.layout(node).unwrap();
        (l.content_size.width, l.content_size.height)
    }

    /// Re-resolve all absolute bounds with a new root offset.
    /// Used for deferred elements that need to be repositioned after their
    /// initial layout solve (e.g., tooltips placed relative to an anchor).
    pub fn resolve_with_offset(&mut self, root: LayoutId, offset_x: f32, offset_y: f32) {
        let root_node = self.nodes[root.0];
        self.resolve_bounds(root_node, offset_x, offset_y);
    }

    // ── Legacy API (kept during migration) ────────────────────────

    /// Get the computed layout for a node by raw NodeId.
    #[deprecated(note = "Use bounds(LayoutId) instead")]
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
    #[deprecated(note = "Use bounds(LayoutId) instead")]
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

/// Measure text for taffy's constraint-solving loop.
fn measure_text_for_taffy(
    text: &str,
    style: &mozui_text::TextStyle,
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<AvailableSpace>,
    font_system: &FontSystem,
) -> taffy::Size<f32> {
    if let (Some(w), Some(h)) = (known_dimensions.width, known_dimensions.height) {
        return taffy::Size {
            width: w,
            height: h,
        };
    }

    let max_width = known_dimensions
        .width
        .or_else(|| match available_space.width {
            AvailableSpace::Definite(w) => Some(w),
            AvailableSpace::MaxContent => None,
            AvailableSpace::MinContent => Some(0.0),
        });

    let measured = mozui_text::measure_text(text, style, max_width, font_system);

    taffy::Size {
        width: known_dimensions.width.unwrap_or(measured.width),
        height: known_dimensions.height.unwrap_or(measured.height),
    }
}
