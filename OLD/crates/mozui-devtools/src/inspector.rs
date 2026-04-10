use mozui_layout::LayoutId;

/// Debug metadata for an element, collected during layout.
pub struct ElementInfo {
    pub type_name: &'static str,
    pub layout_id: LayoutId,
    pub properties: Vec<(&'static str, String)>,
}

/// An entry in the element debug tree, with resolved bounds.
#[derive(Debug, Clone)]
pub struct ElementTreeEntry {
    pub type_name: &'static str,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub depth: u32,
    pub properties: Vec<(&'static str, String)>,
}

/// Runtime state for the element inspector.
pub struct InspectorState {
    /// Collected element tree from the last frame.
    pub tree: Vec<ElementTreeEntry>,
    /// Index of the hovered element in `tree` (if any).
    pub hovered: Option<usize>,
    /// Index of the selected element in `tree` (if any).
    pub selected: Option<usize>,
}
