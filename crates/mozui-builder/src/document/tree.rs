use slotmap::{SlotMap, new_key_type};

use super::{ComponentDescriptor, StyleDescriptor};

new_key_type! {
    pub struct NodeId;
}

#[derive(Clone, Debug)]
pub struct DocumentNode {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub component: ComponentDescriptor,
    pub style: StyleDescriptor,
    pub name: Option<String>,
    pub visible: bool,
}

#[derive(Clone, Debug)]
pub struct DocumentTree {
    pub nodes: SlotMap<NodeId, DocumentNode>,
    pub root: NodeId,
}

impl DocumentTree {
    pub fn new() -> Self {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(DocumentNode {
            parent: None,
            children: Vec::new(),
            component: ComponentDescriptor::Container,
            style: StyleDescriptor {
                display: Some(super::DisplayMode::Flex),
                flex_direction: Some(super::FlexDir::Column),
                width: Some(super::LengthValue::Percent(100.0)),
                height: Some(super::LengthValue::Percent(100.0)),
                ..Default::default()
            },
            name: Some("root".into()),
            visible: true,
        });
        Self { nodes, root }
    }

    pub fn insert_child(
        &mut self,
        parent: NodeId,
        index: usize,
        component: ComponentDescriptor,
        style: StyleDescriptor,
        name: Option<String>,
    ) -> NodeId {
        let id = self.nodes.insert(DocumentNode {
            parent: Some(parent),
            children: Vec::new(),
            component,
            style,
            name,
            visible: true,
        });
        let parent_node = &mut self.nodes[parent];
        let idx = index.min(parent_node.children.len());
        parent_node.children.insert(idx, id);
        id
    }

    pub fn remove_node(&mut self, id: NodeId) {
        let children: Vec<NodeId> = self.nodes[id].children.clone();
        for child in children {
            self.remove_node(child);
        }

        if let Some(parent_id) = self.nodes[id].parent {
            if let Some(parent) = self.nodes.get_mut(parent_id) {
                parent.children.retain(|c| *c != id);
            }
        }

        self.nodes.remove(id);
    }

    pub fn node(&self, id: NodeId) -> &DocumentNode {
        &self.nodes[id]
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut DocumentNode {
        &mut self.nodes[id]
    }

    /// Move `node_id` to become a child of `new_parent` at `index`.
    /// Returns false if the move would create a cycle (new_parent is a descendant of node_id)
    /// or if node_id is the root.
    pub fn move_node(&mut self, node_id: NodeId, new_parent: NodeId, index: usize) -> bool {
        if node_id == self.root {
            return false;
        }
        if self.is_descendant(new_parent, node_id) {
            return false;
        }

        // Detach from old parent
        if let Some(old_parent) = self.nodes[node_id].parent {
            self.nodes[old_parent].children.retain(|c| *c != node_id);
        }

        // Attach to new parent
        self.nodes[node_id].parent = Some(new_parent);
        let children = &mut self.nodes[new_parent].children;
        let idx = index.min(children.len());
        children.insert(idx, node_id);
        true
    }

    /// Deep-clone a subtree rooted at `source`, inserting the copy as a child of `parent` at `index`.
    /// Returns the new root NodeId of the cloned subtree.
    pub fn duplicate_subtree(&mut self, source: NodeId, parent: NodeId, index: usize) -> NodeId {
        let node = self.nodes[source].clone();
        let new_id = self.nodes.insert(DocumentNode {
            parent: Some(parent),
            children: Vec::new(),
            component: node.component,
            style: node.style,
            name: node.name,
            visible: node.visible,
        });

        // Recursively clone children
        let old_children = self.nodes[source].children.clone();
        for child_id in old_children {
            self.duplicate_subtree(child_id, new_id, self.nodes[new_id].children.len());
        }

        // Insert into parent
        let parent_children = &mut self.nodes[parent].children;
        let idx = index.min(parent_children.len());
        parent_children.insert(idx, new_id);
        new_id
    }

    /// Returns true if `candidate` is the same as or a descendant of `ancestor`.
    fn is_descendant(&self, candidate: NodeId, ancestor: NodeId) -> bool {
        if candidate == ancestor {
            return true;
        }
        for &child in &self.nodes[ancestor].children {
            if self.is_descendant(candidate, child) {
                return true;
            }
        }
        false
    }
}
