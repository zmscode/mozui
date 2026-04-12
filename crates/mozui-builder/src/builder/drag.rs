use mozui::prelude::*;
use mozui::{SharedString, Window, div, px};

use crate::document::{ComponentDescriptor, NodeId, StyleDescriptor};

/// Data carried during a palette → canvas drag.
#[derive(Clone)]
pub struct PaletteDragData {
    pub component: ComponentDescriptor,
    pub style: StyleDescriptor,
}

/// Data carried when dragging an existing node to reorder/reparent.
#[derive(Clone, Copy)]
pub struct NodeDragData {
    pub node_id: NodeId,
}

/// Minimal view rendered under the cursor during any drag.
pub struct DragPreview {
    label: String,
}

impl DragPreview {
    pub fn new(label: String) -> Self {
        Self { label }
    }
}

impl Render for DragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(10.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(mozui::hsla(0.58, 0.6, 0.45, 0.9))
            .text_color(mozui::hsla(0.0, 0.0, 1.0, 1.0))
            .text_size(px(12.0))
            .child(SharedString::from(self.label.clone()))
    }
}
