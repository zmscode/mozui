use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{ComponentDescriptor, DocumentNode, DocumentTree, NodeId, StyleDescriptor};

const FORMAT_VERSION: &str = "0.1.0";

#[derive(Serialize, Deserialize)]
pub struct DocumentFile {
    pub version: String,
    pub root: String,
    pub nodes: HashMap<String, SerializedNode>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedNode {
    pub component: ComponentDescriptor,
    #[serde(default)]
    pub style: StyleDescriptor,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default = "default_visible")]
    pub visible: bool,
}

fn default_visible() -> bool {
    true
}

impl DocumentTree {
    pub fn to_file(&self) -> DocumentFile {
        let mut nodes = HashMap::new();
        let mut id_to_key: HashMap<NodeId, String> = HashMap::new();

        // Assign string keys — use the node name if available, otherwise auto-generate
        let mut counter = 0u64;
        for (id, node) in &self.nodes {
            let key = if let Some(name) = &node.name {
                name.clone()
            } else {
                counter += 1;
                format!("node_{counter}")
            };
            id_to_key.insert(id, key);
        }

        for (id, node) in &self.nodes {
            let key = id_to_key[&id].clone();
            nodes.insert(
                key,
                SerializedNode {
                    component: node.component.clone(),
                    style: node.style.clone(),
                    children: node
                        .children
                        .iter()
                        .filter_map(|c| id_to_key.get(c).cloned())
                        .collect(),
                    name: node.name.clone(),
                    visible: node.visible,
                },
            );
        }

        DocumentFile {
            version: FORMAT_VERSION.to_string(),
            root: id_to_key[&self.root].clone(),
            nodes,
        }
    }

    pub fn from_file(file: &DocumentFile) -> Result<Self> {
        let mut nodes = slotmap::SlotMap::with_key();
        let mut key_map: HashMap<String, NodeId> = HashMap::new();

        // First pass: insert all nodes with placeholder children/parent
        for (key, snode) in &file.nodes {
            let id = nodes.insert(DocumentNode {
                parent: None,
                children: Vec::new(),
                component: snode.component.clone(),
                style: snode.style.clone(),
                name: snode.name.clone(),
                visible: snode.visible,
            });
            key_map.insert(key.clone(), id);
        }

        // Second pass: resolve children and parent references
        for (key, snode) in &file.nodes {
            let parent_id = key_map[key];
            let children: Vec<NodeId> = snode
                .children
                .iter()
                .filter_map(|child_key| key_map.get(child_key).copied())
                .collect();

            for &child_id in &children {
                nodes[child_id].parent = Some(parent_id);
            }
            nodes[parent_id].children = children;
        }

        let root = *key_map
            .get(&file.root)
            .context("root node not found in file")?;

        Ok(Self { nodes, root })
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        let file = self.to_file();
        let json = serde_json::to_string_pretty(&file)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path).context("failed to read file")?;
        let file: DocumentFile = serde_json::from_str(&json).context("failed to parse JSON")?;
        Self::from_file(&file)
    }
}
