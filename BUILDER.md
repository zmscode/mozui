# mozui-builder — Implementation Plan

A visual drag-and-drop UI builder for mozui. The user drags components onto a canvas, edits their styles and properties, and the builder outputs a `.json` file that can be loaded to render the same UI.

No runtime functionality (event handlers, state, data binding) — that comes later. This phase is purely **layout + appearance → JSON**.

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────┐
│                    mozui-builder app                      │
│                                                          │
│  ┌─────────┐  ┌──────────────────────┐  ┌────────────┐  │
│  │Component│  │                      │  │ Property   │  │
│  │ Palette │  │   Canvas (renders    │  │ Panel      │  │
│  │         │  │   DocumentTree)      │  │            │  │
│  │ • Div   │  │                      │  │ Style      │  │
│  │ • Text  │  │   ┌──────────┐      │  │ Props      │  │
│  │ • Button│  │   │  Div     │      │  │ Layout     │  │
│  │ • Input │  │   │  ┌─────┐ │      │  │            │  │
│  │ • Image │  │   │  │ Btn │ │      │  │            │  │
│  │ • ...   │  │   │  └─────┘ │      │  │            │  │
│  │         │  │   └──────────┘      │  │            │  │
│  └─────────┘  └──────────────────────┘  └────────────┘  │
│                                                          │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ Tree View (outline of document hierarchy)           │ │
│  └─────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
         │
         │  Save / Load
         ▼
    layout.mozui.json
```

The builder is a standalone mozui application that uses mozui itself for its own UI. The central data structure is the `DocumentTree` — a persistent, serializable tree of nodes that describes a UI layout. Each frame, the canvas walks the `DocumentTree` and renders it into live mozui elements.

---

## Core Data Model

### `DocumentTree`

A persistent tree that is the single source of truth for the designed UI. Survives across frames (unlike mozui's transient element tree). Stored as a flat `SlotMap` for O(1) access and easy serialization.

```rust
use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct NodeId; }

pub struct DocumentTree {
    nodes: SlotMap<NodeId, DocumentNode>,
    root: NodeId,
}

pub struct DocumentNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub component: ComponentDescriptor,
    pub style: StyleDescriptor,
    pub name: Option<String>,       // user-assigned label, e.g. "header-row"
    pub locked: bool,               // prevent accidental edits
    pub visible: bool,              // toggle visibility in canvas
}
```

### `ComponentDescriptor`

Describes *what* this node renders, without any runtime behavior.

```rust
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ComponentDescriptor {
    /// A plain container (div). The most common node type.
    Container,

    /// Static text content.
    Text {
        content: String,
    },

    /// A button (visual only — no on_click).
    Button {
        label: String,
        variant: ButtonVariant,
    },

    /// A text input (visual only — no state).
    Input {
        placeholder: String,
    },

    /// An image element.
    Image {
        source: ImageSource,
        object_fit: ObjectFit,
    },

    /// An icon from the icon set.
    Icon {
        name: String,
        size: Option<f32>,
    },

    /// A checkbox (visual only).
    Checkbox {
        label: String,
        checked: bool,
    },

    /// A badge/label.
    Badge {
        text: String,
    },

    /// Placeholder for future component types.
    Custom {
        type_name: String,
        props: serde_json::Value,
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ImageSource {
    Path(String),
    Url(String),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ObjectFit {
    Fill,
    Contain,
    Cover,
    ScaleDown,
    None,
}
```

### `StyleDescriptor`

A serializable subset of mozui's `StyleRefinement` — only the properties that make sense for a visual builder. This maps 1:1 to mozui's style system but uses simple JSON-friendly types.

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct StyleDescriptor {
    // Display & layout
    pub display: Option<DisplayMode>,       // flex | grid | block
    pub flex_direction: Option<FlexDir>,     // row | column
    pub flex_wrap: Option<WrapMode>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<LengthValue>,
    pub align_items: Option<Align>,
    pub justify_content: Option<Justify>,
    pub align_self: Option<Align>,
    pub gap: Option<SizeValue>,

    // Sizing
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub min_width: Option<LengthValue>,
    pub min_height: Option<LengthValue>,
    pub max_width: Option<LengthValue>,
    pub max_height: Option<LengthValue>,

    // Spacing
    pub margin: Option<EdgesValue>,
    pub padding: Option<EdgesValue>,

    // Position
    pub position: Option<PositionMode>,     // relative | absolute
    pub inset: Option<EdgesValue>,

    // Borders
    pub border_widths: Option<EdgesValue>,
    pub border_color: Option<ColorValue>,
    pub border_style: Option<BorderStyleValue>,
    pub corner_radii: Option<CornersValue>,

    // Visual
    pub background: Option<ColorValue>,
    pub opacity: Option<f32>,
    pub box_shadow: Option<Vec<BoxShadowValue>>,
    pub overflow_x: Option<OverflowMode>,
    pub overflow_y: Option<OverflowMode>,

    // Text (inherited by text children)
    pub font_size: Option<f32>,
    pub font_weight: Option<f32>,
    pub font_style: Option<FontStyleValue>,
    pub text_color: Option<ColorValue>,
    pub text_align: Option<TextAlignValue>,
    pub line_height: Option<f32>,
    pub white_space: Option<WhiteSpaceValue>,

    // Cursor
    pub cursor: Option<CursorStyleValue>,
}

// Supporting types — all Serialize + Deserialize

#[derive(Serialize, Deserialize, Clone)]
pub enum LengthValue {
    Auto,
    Px(f32),
    Rem(f32),
    Percent(f32),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EdgesValue {
    pub top: Option<LengthValue>,
    pub right: Option<LengthValue>,
    pub bottom: Option<LengthValue>,
    pub left: Option<LengthValue>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CornersValue {
    pub top_left: Option<f32>,
    pub top_right: Option<f32>,
    pub bottom_left: Option<f32>,
    pub bottom_right: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ColorValue {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SizeValue {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BoxShadowValue {
    pub color: ColorValue,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
}
```

### Conversion: `StyleDescriptor → StyleRefinement`

A `From<&StyleDescriptor> for StyleRefinement` impl that maps the builder's simple types to mozui's actual style types. This is the bridge between the JSON world and the rendering world.

```rust
impl StyleDescriptor {
    pub fn to_style_refinement(&self) -> StyleRefinement {
        let mut style = StyleRefinement::default();

        if let Some(display) = &self.display {
            style.display = Some(match display {
                DisplayMode::Flex => Display::Flex,
                DisplayMode::Grid => Display::Grid,
                DisplayMode::Block => Display::Block,
            });
        }

        if let Some(w) = &self.width {
            style.size.width = Some(to_length(w));
        }

        if let Some(bg) = &self.background {
            style.background = Some(Fill::Color(to_hsla(bg)));
        }

        // ... map all fields
        style
    }
}

fn to_length(v: &LengthValue) -> Length {
    match v {
        LengthValue::Auto => Length::Auto,
        LengthValue::Px(n) => Length::Absolute(AbsoluteLength::Pixels(px(*n))),
        LengthValue::Rem(n) => Length::Absolute(AbsoluteLength::Rems(rems(*n))),
        LengthValue::Percent(n) => Length::Relative(*n / 100.0),
    }
}
```

---

## JSON File Format

The output `.mozui.json` file is a direct serialization of the `DocumentTree`. It should be human-readable and hand-editable.

### Example output

```json
{
  "version": "0.1.0",
  "root": "node_0",
  "nodes": {
    "node_0": {
      "component": { "type": "Container" },
      "style": {
        "display": "flex",
        "flex_direction": "column",
        "width": { "Px": 800.0 },
        "height": { "Px": 600.0 },
        "padding": { "top": { "Px": 16.0 }, "right": { "Px": 16.0 }, "bottom": { "Px": 16.0 }, "left": { "Px": 16.0 } },
        "background": { "h": 220.0, "s": 0.15, "l": 0.12, "a": 1.0 },
        "gap": { "width": { "Px": 12.0 }, "height": { "Px": 12.0 } }
      },
      "children": ["node_1", "node_2"],
      "name": "root"
    },
    "node_1": {
      "component": {
        "type": "Text",
        "content": "Welcome to my app"
      },
      "style": {
        "font_size": 24.0,
        "font_weight": 700.0,
        "text_color": { "h": 0.0, "s": 0.0, "l": 1.0, "a": 1.0 }
      },
      "children": [],
      "name": "title"
    },
    "node_2": {
      "component": {
        "type": "Button",
        "label": "Get Started",
        "variant": "Primary"
      },
      "style": {
        "padding": { "top": { "Px": 8.0 }, "right": { "Px": 16.0 }, "bottom": { "Px": 8.0 }, "left": { "Px": 16.0 } },
        "corner_radii": { "top_left": 6.0, "top_right": 6.0, "bottom_left": 6.0, "bottom_right": 6.0 },
        "background": { "h": 220.0, "s": 0.8, "l": 0.5, "a": 1.0 }
      },
      "children": [],
      "name": "cta-button"
    }
  }
}
```

### File format rules

- **Version field** — semver for schema migrations as the format evolves.
- **Flat node map** — nodes are a flat `HashMap<String, SerializedNode>` keyed by string IDs (converted from `SlotMap` keys on export). This avoids deeply nested JSON and makes diffs clean.
- **`root` pointer** — identifies the root node.
- **Optional fields** — all style fields are `Option`. Omitted = inherit/default.
- **`children` ordering** — the `children` array defines render order (first = top/left).
- **`name` field** — optional human label, not required for rendering.

---

## JSON Loader (Renderer)

A standalone module that takes a `.mozui.json` file and renders it as a live mozui window. This is the proof that the format works — load → render → same UI.

### `DocumentRenderer`

```rust
pub struct DocumentRenderer {
    tree: DocumentTree,
}

impl DocumentRenderer {
    pub fn from_file(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let tree: DocumentTree = serde_json::from_str(&json)?;
        Ok(Self { tree })
    }

    /// Recursively render the document tree into mozui elements.
    fn render_node(&self, node_id: NodeId, window: &mut Window, cx: &mut App) -> AnyElement {
        let node = &self.tree.nodes[node_id];
        let style = node.style.to_style_refinement();

        match &node.component {
            ComponentDescriptor::Container => {
                let mut el = div();
                el = apply_style(el, &style);
                for &child_id in &node.children {
                    el = el.child(self.render_node(child_id, window, cx));
                }
                el.into_any_element()
            }
            ComponentDescriptor::Text { content } => {
                let mut el = div();
                el = apply_style(el, &style);
                el = el.child(content.clone());
                el.into_any_element()
            }
            ComponentDescriptor::Button { label, variant } => {
                // Use mozui-components Button, apply variant + style overrides
                let mut el = div();
                el = apply_style(el, &style);
                el = el.child(label.clone()); // simplified — real impl uses Button component
                el.into_any_element()
            }
            // ... each ComponentDescriptor variant
        }
    }
}

impl Render for DocumentRenderer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_node(self.tree.root, window, cx)
    }
}

fn apply_style(el: Div, style: &StyleRefinement) -> Div {
    // Apply the StyleRefinement to the div.
    // StyleRefinement can be applied via the Styled trait methods,
    // or by directly setting el.interactivity().base_style.
    el
}
```

### Loading from CLI

```
$ mozui-builder open layout.mozui.json
```

Opens a window and renders the JSON layout. This is the minimum viable deliverable — round-trip proof.

---

## Builder Application

The builder itself is a mozui app with these panels:

### Panel Layout

```
┌──────────┬────────────────────────────────┬──────────────┐
│          │                                │              │
│ Component│         Canvas                 │  Property    │
│ Palette  │                                │  Panel       │
│          │  (live render of               │              │
│ ──────── │   DocumentTree with            │  [Component] │
│          │   selection overlays)          │  [Style]     │
│ Tree     │                                │  [Layout]    │
│ View     │                                │              │
│          │                                │              │
└──────────┴────────────────────────────────┴──────────────┘
│                    Toolbar                                │
│  [Save] [Load] [Undo] [Redo] [Delete]                    │
└───────────────────────────────────────────────────────────┘
```

### Component Palette

A sidebar listing available components. Each item is draggable onto the canvas.

```rust
pub struct PaletteItem {
    pub component_type: ComponentType,
    pub label: &'static str,
    pub icon: Icon,
    pub default_descriptor: ComponentDescriptor,
    pub default_style: StyleDescriptor,
}

pub enum ComponentType {
    Container,
    Text,
    Button,
    Input,
    Image,
    Icon,
    Checkbox,
    Badge,
}
```

Components in the palette are registered via a `ComponentRegistry`:

```rust
pub struct ComponentRegistry {
    items: Vec<PaletteItem>,
}

impl ComponentRegistry {
    pub fn default_registry() -> Self {
        Self {
            items: vec![
                PaletteItem {
                    component_type: ComponentType::Container,
                    label: "Container",
                    default_descriptor: ComponentDescriptor::Container,
                    default_style: StyleDescriptor {
                        display: Some(DisplayMode::Flex),
                        flex_direction: Some(FlexDir::Column),
                        padding: Some(EdgesValue::all(LengthValue::Px(8.0))),
                        min_width: Some(LengthValue::Px(48.0)),
                        min_height: Some(LengthValue::Px(48.0)),
                        ..Default::default()
                    },
                },
                PaletteItem {
                    component_type: ComponentType::Text,
                    label: "Text",
                    default_descriptor: ComponentDescriptor::Text {
                        content: "Text".into(),
                    },
                    default_style: StyleDescriptor::default(),
                },
                PaletteItem {
                    component_type: ComponentType::Button,
                    label: "Button",
                    default_descriptor: ComponentDescriptor::Button {
                        label: "Button".into(),
                        variant: ButtonVariant::Primary,
                    },
                    default_style: StyleDescriptor {
                        padding: Some(EdgesValue::symmetric(
                            LengthValue::Px(8.0),
                            LengthValue::Px(16.0),
                        )),
                        corner_radii: Some(CornersValue::all(6.0)),
                        ..Default::default()
                    },
                },
                // ... more items
            ],
        }
    }
}
```

### Canvas

The canvas renders the `DocumentTree` live, with overlays for:

- **Selection highlight** — blue border around the selected node.
- **Hover highlight** — lighter border on mouse hover.
- **Drop indicators** — blue lines showing where a dragged component will be inserted.
- **Resize handles** — corner/edge handles on the selected node (optional, stretch goal).

The canvas wraps each rendered node in an invisible interaction layer that:
1. Tags each element with its `NodeId` (via `ElementId`).
2. Listens for `on_click` to select the node.
3. Listens for `on_drag` to start moving the node.
4. Listens for `on_drag_move` to show drop indicators.

```rust
fn render_canvas_node(&self, node_id: NodeId, window: &mut Window, cx: &mut App) -> AnyElement {
    let node = &self.tree.nodes[node_id];
    let is_selected = self.selection == Some(node_id);
    let is_hovered = self.hover == Some(node_id);

    let content = self.render_component(node, window, cx);

    // Wrap in interaction layer
    div()
        .id(ElementId::from(node_id_to_string(node_id)))
        .relative()
        .child(content)
        // Selection/hover overlays
        .when(is_selected, |el| {
            el.border_2().border_color(blue_500())
        })
        .when(is_hovered && !is_selected, |el| {
            el.border_1().border_color(blue_300().opacity(0.5))
        })
        .on_click(cx.listener(move |this, _event, _window, _cx| {
            this.selection = Some(node_id);
        }))
        .on_drag(node_id, |node_id, _window, _cx| {
            // Return a drag preview element
            div().w(px(100.0)).h(px(30.0)).bg(blue_100()).into_any_element()
        })
        .into_any_element()
}
```

### Drop Target Resolution

When a drag hovers over the canvas, we need to determine the insertion point. The algorithm:

1. Find which rendered node the mouse is over (hit test via `ElementId` → `NodeId`).
2. Determine position within that node's bounds:
   - **Top 25%** → insert before this node (sibling, above).
   - **Bottom 25%** → insert after this node (sibling, below).
   - **Center 50%** → insert as last child of this node (nest inside).
3. For horizontal flex containers, use left/right instead of top/bottom.
4. Show a blue indicator line at the resolved insertion point.

```rust
pub enum DropPosition {
    Before(NodeId),     // insert as sibling before this node
    After(NodeId),      // insert as sibling after this node
    Inside(NodeId),     // insert as last child of this node
}

pub struct DropTarget {
    pub position: DropPosition,
    pub indicator_bounds: Bounds<Pixels>,  // where to draw the blue line
}
```

### Property Panel

A right sidebar that shows editable properties for the selected node. Divided into sections:

**Component section** — fields specific to the component type:
- Text: content text area
- Button: label text field, variant dropdown
- Input: placeholder text field
- Image: source path field, object fit dropdown

**Layout section** — flex/grid properties:
- Display mode (flex/grid/block)
- Flex direction (row/column)
- Align items / justify content dropdowns
- Gap input
- Flex grow / shrink / basis

**Size section:**
- Width / height (with unit selector: px/rem/%/auto)
- Min/max width/height

**Spacing section:**
- Margin (4 inputs: top/right/bottom/left, with link-all toggle)
- Padding (same)

**Border section:**
- Border width (4 edges)
- Border color (color picker)
- Border radius (4 corners)

**Background section:**
- Background color (color picker)
- Opacity slider

**Text section** (shown when component has text):
- Font size
- Font weight
- Color
- Alignment

Each property change immediately mutates the `DocumentTree` and the canvas re-renders next frame.

### Tree View

A collapsible tree outline in the left sidebar (below the palette) showing the document hierarchy. Each row shows:
- Component type icon
- Node name (or auto-generated label like "Container #3")
- Visibility toggle
- Lock toggle

Tree view supports:
- Click to select
- Drag to reorder/reparent
- Right-click context menu (duplicate, delete, wrap in container)

---

## Selection & Interaction State

```rust
pub struct BuilderState {
    pub tree: DocumentTree,
    pub selection: Option<NodeId>,
    pub hover: Option<NodeId>,
    pub drag_state: Option<DragState>,
    pub drop_target: Option<DropTarget>,
    pub clipboard: Option<ClipboardEntry>,
    pub history: UndoHistory,
}

pub enum DragState {
    /// Dragging a new component from the palette
    FromPalette {
        item: PaletteItem,
    },
    /// Dragging an existing node to reorder/reparent
    Reorder {
        node_id: NodeId,
    },
}

pub struct ClipboardEntry {
    pub subtree: Vec<DocumentNode>,  // serialized subtree for copy/paste
}
```

---

## Undo / Redo

Every mutation to the `DocumentTree` goes through a command that can be undone.

```rust
pub enum Command {
    InsertNode { parent: NodeId, index: usize, subtree: Vec<DocumentNode> },
    RemoveNode { node_id: NodeId, parent: NodeId, index: usize, subtree: Vec<DocumentNode> },
    MoveNode { node_id: NodeId, old_parent: NodeId, old_index: usize, new_parent: NodeId, new_index: usize },
    UpdateStyle { node_id: NodeId, old_style: StyleDescriptor, new_style: StyleDescriptor },
    UpdateComponent { node_id: NodeId, old: ComponentDescriptor, new: ComponentDescriptor },
    Rename { node_id: NodeId, old_name: Option<String>, new_name: Option<String> },
}

pub struct UndoHistory {
    undo_stack: Vec<Command>,
    redo_stack: Vec<Command>,
}

impl UndoHistory {
    pub fn execute(&mut self, tree: &mut DocumentTree, cmd: Command) {
        cmd.apply(tree);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, tree: &mut DocumentTree) {
        if let Some(cmd) = self.undo_stack.pop() {
            cmd.reverse(tree);
            self.redo_stack.push(cmd);
        }
    }

    pub fn redo(&mut self, tree: &mut DocumentTree) {
        if let Some(cmd) = self.redo_stack.pop() {
            cmd.apply(tree);
            self.undo_stack.push(cmd);
        }
    }
}
```

---

## Crate Structure

```
crates/mozui-builder/
├── Cargo.toml
├── src/
│   ├── main.rs                 # App entry point, window setup
│   ├── lib.rs                  # Re-exports
│   │
│   ├── document/
│   │   ├── mod.rs
│   │   ├── tree.rs             # DocumentTree, DocumentNode
│   │   ├── component.rs        # ComponentDescriptor, ComponentType
│   │   ├── style.rs            # StyleDescriptor + conversion to StyleRefinement
│   │   └── serialization.rs    # JSON save/load, file format versioning
│   │
│   ├── registry/
│   │   ├── mod.rs
│   │   └── components.rs       # ComponentRegistry, PaletteItem, defaults
│   │
│   ├── renderer/
│   │   ├── mod.rs
│   │   └── document_renderer.rs # DocumentTree → mozui elements
│   │
│   ├── builder/
│   │   ├── mod.rs
│   │   ├── state.rs            # BuilderState, selection, drag state
│   │   ├── canvas.rs           # Canvas panel (renders tree + overlays)
│   │   ├── palette.rs          # Component palette sidebar
│   │   ├── property_panel.rs   # Property editor sidebar
│   │   ├── tree_view.rs        # Document tree outline
│   │   ├── toolbar.rs          # Top toolbar (save/load/undo/redo)
│   │   └── drop_target.rs      # Drop position calculation
│   │
│   └── history/
│       ├── mod.rs
│       └── commands.rs         # Command enum, UndoHistory
```

### Cargo.toml

```toml
[package]
name = "mozui-builder"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
mozui.workspace = true
mozui-components.workspace = true
serde.workspace = true
serde_json.workspace = true
slotmap.workspace = true
anyhow.workspace = true
uuid.workspace = true
```

---

## Implementation Phases

### Phase 1 — Document model + JSON round-trip

**Goal:** Define the data model, serialize to JSON, deserialize back, render in a window. Prove the format works.

1. Create `crates/mozui-builder/` with Cargo.toml.
2. Implement `DocumentTree`, `DocumentNode`, `ComponentDescriptor`, `StyleDescriptor`.
3. Add serde derives, write JSON serialization/deserialization.
4. Implement `StyleDescriptor::to_style_refinement()` conversion.
5. Implement `DocumentRenderer` — walks the tree and produces mozui elements.
6. Write `main.rs` that:
   - Accepts a `.mozui.json` file path as CLI arg.
   - Parses it into a `DocumentTree`.
   - Opens a mozui window rendering the tree.
7. Hand-write a sample `.json` file and verify it renders correctly.

**Deliverable:** `cargo run -p mozui-builder -- open sample.mozui.json` opens a window showing the designed UI.

### Phase 2 — Builder chrome (empty shell)

**Goal:** The builder app window with panel layout but no interactivity.

1. Implement the three-panel layout (palette | canvas | property panel).
2. Component palette renders the list of available components from the registry.
3. Canvas renders a hardcoded `DocumentTree`.
4. Property panel shows fields for a hardcoded selected node.
5. Tree view shows the document hierarchy.

**Deliverable:** Builder app opens with all panels visible, rendering a sample document.

### Phase 3 — Selection + property editing

**Goal:** Click nodes to select them, edit their properties live.

1. Wrap each canvas node in an interaction layer with `on_click` for selection.
2. Wire selection state to the property panel (show selected node's properties).
3. Implement property inputs: text fields, dropdowns, number inputs, color picker.
4. Property changes mutate the `DocumentTree` → canvas re-renders.
5. Implement the toolbar save button (serialize tree → JSON file).

**Deliverable:** Can select nodes, change their styles/props, save to JSON, reopen.

### Phase 4 — Drag and drop from palette

**Goal:** Drag components from the palette onto the canvas to add them.

1. Make palette items draggable (`on_drag`).
2. Implement drop target resolution (top/bottom/center zones).
3. Render drop indicator lines on the canvas.
4. On drop: insert new `DocumentNode` into the tree at the resolved position.
5. Auto-select newly inserted nodes.

**Deliverable:** Can build a layout by dragging components onto the canvas.

### Phase 5 — Reorder + reparent

**Goal:** Drag existing nodes to move them within the tree.

1. Make canvas nodes draggable (`on_drag` with `NodeId` payload).
2. Reuse drop target resolution from Phase 4.
3. On drop: move the node in the `DocumentTree` (remove from old parent, insert at new position).
4. Prevent dropping a node inside itself (cycle detection).
5. Tree view drag-and-drop for reordering.

**Deliverable:** Full drag-and-drop reordering and reparenting.

### Phase 6 — Undo/redo + polish

**Goal:** Undo/redo, keyboard shortcuts, quality of life.

1. Implement the `Command` enum and `UndoHistory`.
2. Route all tree mutations through commands.
3. Keyboard shortcuts: Cmd+Z (undo), Cmd+Shift+Z (redo), Delete/Backspace (delete node), Cmd+D (duplicate), Cmd+S (save).
4. Right-click context menu on canvas nodes (delete, duplicate, wrap in container).
5. Copy/paste (Cmd+C / Cmd+V) — serialize subtree to clipboard.
6. Load button in toolbar (file picker → open `.mozui.json`).

**Deliverable:** Full builder workflow with undo/redo and keyboard shortcuts.

---

## Key Technical Decisions

### Why a flat SlotMap instead of a recursive tree?

- O(1) node lookup by ID (needed for selection, property editing, drop targets).
- Easy serialization — no recursive structures to deal with.
- Parent/child relationships via IDs, so moving nodes is just updating IDs.
- SlotMap keys are stable across insertions/removals (unlike Vec indices).

### Why a custom `StyleDescriptor` instead of using `StyleRefinement` directly?

- `StyleRefinement` is tightly coupled to mozui internals (uses `Pixels`, `Rems`, etc. which have non-trivial serde).
- A custom descriptor gives us control over the JSON schema — we want clean, hand-editable JSON.
- The conversion layer (`to_style_refinement`) is straightforward and lets us add builder-specific defaults.
- If mozui's style types change, only the conversion layer needs updating.

### Why not generate Rust code instead of JSON?

- JSON is editable, diffable, and language-agnostic.
- Code generation can be a later feature that reads the same JSON.
- JSON files can be loaded at runtime without recompilation.
- The builder can hot-reload JSON changes during development.

### Why `#[serde(tag = "type")]` on ComponentDescriptor?

- Produces `{ "type": "Button", "label": "Click me" }` instead of `{ "Button": { "label": "Click me" } }`.
- More readable, easier to hand-edit, matches how component palettes work conceptually.

---

## Open Questions

1. **Theme integration** — should the builder include a theme context so components render with the correct colors? Probably yes — load the active `mozui-components` theme.
2. **Viewport scaling** — should the canvas support zoom in/out? Useful but not required for v1.
3. **Multi-select** — select multiple nodes for bulk style changes? Defer to post-v1.
4. **Grid layout editing** — grid is more complex than flex. Support flex first, grid later.
5. **Code generation** — generating Rust source from the JSON is a natural next step but out of scope for this plan.
6. **Hot reload** — watch the JSON file and re-render on change? Nice for development, easy to add.
