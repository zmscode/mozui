use std::rc::Rc;

use mozui::prelude::*;
use mozui::{AnyElement, App, ClickEvent, ElementId, SharedString, Window, div, px};

use crate::document::{DocumentTree, NodeId};

use super::drag::{DragPreview, NodeDragData};

type SelectHandler = Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>;
type NodeDropHandler = Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&NodeDragData, &mut Window, &mut App)>>;

pub fn render_tree_view(
    tree: &DocumentTree,
    selected: &[NodeId],
    on_select: SelectHandler,
    on_node_drop: NodeDropHandler,
    _window: &mut Window,
    _cx: &mut App,
) -> AnyElement {
    let mut rows = div().flex().flex_col().w_full();

    rows = rows.child(render_tree_node(
        tree,
        tree.root,
        0,
        selected,
        &on_select,
        &on_node_drop,
    ));

    div()
        .flex()
        .flex_col()
        .w_full()
        .child(
            div()
                .px(px(12.0))
                .py(px(8.0))
                .text_size(px(11.0))
                .font_weight(mozui::FontWeight::BOLD)
                .text_color(mozui::hsla(0.0, 0.0, 0.45, 1.0))
                .child(SharedString::from("LAYERS")),
        )
        .child(rows)
        .into_any_element()
}

fn tree_element_id(node_id: NodeId) -> ElementId {
    ElementId::Name(format!("tree-node-{node_id:?}").into())
}

fn render_tree_node(
    tree: &DocumentTree,
    node_id: NodeId,
    depth: usize,
    selected: &[NodeId],
    on_select: &SelectHandler,
    on_node_drop: &NodeDropHandler,
) -> AnyElement {
    let node = tree.node(node_id);
    let is_selected = selected.contains(&node_id);
    let is_root = node_id == tree.root;
    let is_container = node.component.can_have_children();

    let label = node
        .name
        .clone()
        .unwrap_or_else(|| node.component.display_name().to_string());

    let indent = depth as f32 * 16.0;

    let click_handler = on_select(node_id);
    let mut row = div()
        .id(tree_element_id(node_id))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .pl(px(indent + 10.0))
        .pr(px(10.0))
        .py(px(3.0))
        .rounded(px(3.0))
        .cursor_pointer()
        .text_size(px(12.0))
        .on_click(move |event, window, cx| {
            click_handler(event, window, cx);
        });

    // Non-root nodes are draggable
    if !is_root {
        let display_name = label.clone();
        row = row.on_drag(
            NodeDragData { node_id },
            move |_data, _offset, _window, cx| cx.new(|_cx| DragPreview::new(display_name.clone())),
        );
    }

    // Containers accept node drops for reparenting
    if is_container {
        let drop_handler = on_node_drop(node_id);
        row = row
            .on_drop(move |data: &NodeDragData, window, cx| {
                cx.stop_propagation();
                drop_handler(data, window, cx);
            })
            .drag_over::<NodeDragData>(|s, _data, _window, _cx| {
                s.bg(mozui::hsla(0.42, 0.6, 0.45, 0.15))
            });
    }

    if is_selected {
        row = row
            .bg(mozui::hsla(0.58, 0.6, 0.45, 0.2))
            .text_color(mozui::hsla(0.58, 0.7, 0.7, 1.0));
    } else {
        row = row
            .text_color(mozui::hsla(0.0, 0.0, 0.7, 1.0))
            .hover(|s| s.bg(mozui::hsla(0.0, 0.0, 1.0, 0.04)));
    }

    let has_children = !node.children.is_empty();
    if has_children {
        row = row.child(
            div()
                .text_size(px(10.0))
                .text_color(mozui::hsla(0.0, 0.0, 0.45, 1.0))
                .child(SharedString::from("\u{25BE}")),
        );
    } else {
        row = row.child(div().w(px(10.0)));
    }

    let type_icon = match &node.component {
        crate::document::ComponentDescriptor::Container => "\u{25A1}",
        crate::document::ComponentDescriptor::Text { .. } => "T",
        crate::document::ComponentDescriptor::Button { .. } => "\u{25A3}",
        crate::document::ComponentDescriptor::Input { .. } => "\u{2336}",
        crate::document::ComponentDescriptor::Checkbox { .. } => "\u{2611}",
        crate::document::ComponentDescriptor::Badge { .. } => "\u{25CF}",
        crate::document::ComponentDescriptor::Divider => "\u{2500}",
        crate::document::ComponentDescriptor::Switch { .. } => "\u{25C9}",
        crate::document::ComponentDescriptor::Progress { .. } => "\u{25AE}",
        crate::document::ComponentDescriptor::Avatar { .. } => "\u{25D0}",
        crate::document::ComponentDescriptor::Label { .. } => "\u{1D4DB}",
        crate::document::ComponentDescriptor::Radio { .. } => "\u{25CE}",
    };

    row = row
        .child(
            div()
                .text_size(px(10.0))
                .text_color(mozui::hsla(0.0, 0.0, 0.4, 1.0))
                .child(SharedString::from(type_icon)),
        )
        .child(SharedString::from(label));

    let mut col = div().flex().flex_col().child(row);

    let children: Vec<NodeId> = node.children.clone();
    for child_id in children {
        col = col.child(render_tree_node(
            tree,
            child_id,
            depth + 1,
            selected,
            on_select,
            on_node_drop,
        ));
    }

    col.into_any_element()
}
