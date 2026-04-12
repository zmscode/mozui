use crate::document::{DocumentTree, NodeId};

/// Snapshot-based undo/redo. Each entry stores the full tree state + selection.
pub struct UndoHistory {
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
}

#[derive(Clone)]
struct Snapshot {
    tree: DocumentTree,
    selected: Vec<NodeId>,
}

impl UndoHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Save current state before a mutation.
    pub fn save(&mut self, tree: &DocumentTree, selected: Vec<NodeId>) {
        self.undo_stack.push(Snapshot {
            tree: tree.clone(),
            selected,
        });
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Undo: restore the previous state. Returns (tree, selection) to apply.
    pub fn undo(
        &mut self,
        current_tree: &DocumentTree,
        current_selected: Vec<NodeId>,
    ) -> Option<(DocumentTree, Vec<NodeId>)> {
        let prev = self.undo_stack.pop()?;
        self.redo_stack.push(Snapshot {
            tree: current_tree.clone(),
            selected: current_selected,
        });
        Some((prev.tree, prev.selected))
    }

    /// Redo: restore the next state. Returns (tree, selection) to apply.
    pub fn redo(
        &mut self,
        current_tree: &DocumentTree,
        current_selected: Vec<NodeId>,
    ) -> Option<(DocumentTree, Vec<NodeId>)> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(Snapshot {
            tree: current_tree.clone(),
            selected: current_selected,
        });
        Some((next.tree, next.selected))
    }
}
