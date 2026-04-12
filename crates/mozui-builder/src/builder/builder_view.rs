use std::ops::DerefMut;
use std::path::PathBuf;
use std::rc::Rc;

use mozui::prelude::*;
use mozui::{AnyElement, App, ClickEvent, Context, Entity, SharedString, Window, div, point, px};
use mozui_components::dock::{DockArea, DockItem};

use crate::document::{ButtonVariant, ComponentDescriptor, DocumentTree, NodeId};

use super::canvas::{PanEvent, ZoomEvent};
use super::canvas_state::{CanvasState, InteractionMode};
use super::history::UndoHistory;
use super::panels::{CanvasPanel, ComponentsPanel, LayersPanel, PropertiesPanel};
use super::property_panel::{parse_edges_value, parse_length_value};
use super::toolbar::render_toolbar;

const LEFT_SIDEBAR_WIDTH: f32 = 220.0;
const RIGHT_SIDEBAR_WIDTH: f32 = 260.0;

pub struct BuilderView {
    pub tree: DocumentTree,
    pub selected: Vec<NodeId>,
    pub file_path: Option<PathBuf>,
    pub history: UndoHistory,
    pub context_menu: Option<ContextMenu>,
    pub canvas_state: CanvasState,
    dock_area: Entity<DockArea>,
}

pub struct ContextMenu {
    pub node_id: NodeId,
    pub position: mozui::Point<mozui::Pixels>,
}

impl BuilderView {
    pub fn new(
        tree: DocumentTree,
        file_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let root = tree.root;
        let weak_self = cx.entity().downgrade();

        let builder_entity = cx.entity().clone();

        // Create panel entities — each observes BuilderView for re-render
        let components = cx.new(|cx| ComponentsPanel::new(window, cx));
        let layers = cx.new(|cx| {
            let mut panel = LayersPanel::new(weak_self.clone(), window, cx);
            panel._subscription = Some(cx.observe(&builder_entity, |_, _, cx| cx.notify()));
            panel
        });
        let canvas = cx.new(|cx| {
            let mut panel = CanvasPanel::new(weak_self.clone(), window, cx);
            panel._subscription = Some(cx.observe(&builder_entity, |_, _, cx| cx.notify()));
            panel
        });
        let properties = cx.new(|cx| {
            let mut panel = PropertiesPanel::new(weak_self.clone(), window, cx);
            panel._subscription = Some(cx.observe(&builder_entity, |_, _, cx| cx.notify()));
            panel
        });

        // Create dock area
        let dock_area = cx.new(|cx| {
            let mut area = DockArea::new("builder-dock", None, window, cx);
            let weak_area = cx.entity().downgrade();

            // Left dock: Components + Layers in a vertical split
            let left_items = DockItem::v_split(
                vec![
                    DockItem::tab(components, &weak_area, window, cx),
                    DockItem::tab(layers, &weak_area, window, cx),
                ],
                &weak_area,
                window,
                cx,
            );
            area.set_left_dock(left_items, Some(px(LEFT_SIDEBAR_WIDTH)), true, window, cx);

            // Center: Canvas
            let center = DockItem::tab(canvas, &weak_area, window, cx);
            area.set_center(center, window, cx);

            // Right dock: Properties
            let right_items = DockItem::tab(properties, &weak_area, window, cx);
            area.set_right_dock(right_items, Some(px(RIGHT_SIDEBAR_WIDTH)), true, window, cx);

            area
        });

        Self {
            tree,
            selected: vec![root],
            file_path,
            history: UndoHistory::new(),
            context_menu: None,
            canvas_state: CanvasState::new(),
            dock_area,
        }
    }

    pub fn primary_selected(&self) -> Option<NodeId> {
        self.selected.last().copied()
    }

    pub fn is_selected(&self, node_id: NodeId) -> bool {
        self.selected.contains(&node_id)
    }

    pub(crate) fn select(&mut self, node_id: NodeId) {
        self.selected = vec![node_id];
    }

    pub(crate) fn toggle_select(&mut self, node_id: NodeId) {
        if let Some(pos) = self.selected.iter().position(|&id| id == node_id) {
            self.selected.remove(pos);
        } else {
            self.selected.push(node_id);
        }
    }

    fn save(&self) {
        if let Some(path) = &self.file_path {
            match self.tree.save_to_path(path) {
                Ok(()) => eprintln!("Saved to {}", path.display()),
                Err(e) => eprintln!("Failed to save: {e:#}"),
            }
        } else {
            eprintln!("No file path set — use 'edit <file>' to specify a path");
        }
    }

    pub(crate) fn save_undo(&mut self) {
        self.history.save(&self.tree, self.selected.clone());
    }

    fn undo(&mut self) {
        if let Some((tree, sel)) = self.history.undo(&self.tree, self.selected.clone()) {
            self.tree = tree;
            self.selected = sel;
            // Property panel will re-sync on next render
        }
    }

    fn redo(&mut self) {
        if let Some((tree, sel)) = self.history.redo(&self.tree, self.selected.clone()) {
            self.tree = tree;
            self.selected = sel;
            // Property panel will re-sync on next render
        }
    }

    fn delete_selected(&mut self) {
        let to_delete: Vec<NodeId> = self
            .selected
            .iter()
            .copied()
            .filter(|&id| id != self.tree.root)
            .collect();
        if to_delete.is_empty() {
            return;
        }
        self.save_undo();
        let mut last_parent = None;
        for id in &to_delete {
            last_parent = self.tree.node(*id).parent;
            self.tree.remove_node(*id);
        }
        self.selected = last_parent.into_iter().collect();
    }

    fn duplicate_selected(&mut self) {
        let Some(id) = self.primary_selected() else {
            return;
        };
        if id == self.tree.root {
            return;
        }
        let parent = match self.tree.node(id).parent {
            Some(p) => p,
            None => return,
        };
        let index = self
            .tree
            .node(parent)
            .children
            .iter()
            .position(|c| *c == id)
            .map(|i| i + 1)
            .unwrap_or(self.tree.node(parent).children.len());

        self.save_undo();
        let new_id = self.tree.duplicate_subtree(id, parent, index);
        self.selected = vec![new_id];
    }

    fn wrap_in_container(&mut self, node_id: NodeId) {
        if node_id == self.tree.root {
            return;
        }
        let parent = match self.tree.node(node_id).parent {
            Some(p) => p,
            None => return,
        };
        let index = self
            .tree
            .node(parent)
            .children
            .iter()
            .position(|c| *c == node_id)
            .unwrap_or(0);

        self.save_undo();

        let container_id = self.tree.insert_child(
            parent,
            index,
            crate::document::ComponentDescriptor::Container,
            crate::document::StyleDescriptor {
                display: Some(crate::document::DisplayMode::Flex),
                flex_direction: Some(crate::document::FlexDir::Column),
                padding: Some(crate::document::EdgesValue::all(
                    crate::document::LengthValue::Px(8.0),
                )),
                ..Default::default()
            },
            None,
        );

        self.tree.move_node(node_id, container_id, 0);
        self.selected = vec![container_id];
    }

    pub(crate) fn handle_pan(&mut self, event: PanEvent) {
        match event {
            PanEvent::Start(pos) => {
                self.canvas_state.interaction = InteractionMode::Panning {
                    start_offset: self.canvas_state.offset,
                    start_mouse: pos,
                };
            }
            PanEvent::Move(pos) => {
                if let InteractionMode::Panning {
                    start_offset,
                    start_mouse,
                } = self.canvas_state.interaction
                {
                    self.canvas_state.offset = point(
                        start_offset.x + (pos.x - start_mouse.x),
                        start_offset.y + (pos.y - start_mouse.y),
                    );
                }
            }
            PanEvent::Delta(delta) => {
                self.canvas_state.apply_pan_delta(delta);
            }
            PanEvent::End => {
                self.canvas_state.interaction = InteractionMode::None;
            }
        }
    }

    pub(crate) fn handle_zoom(&mut self, event: ZoomEvent) {
        self.canvas_state.apply_zoom(event.delta, event.cursor);
    }

    pub fn apply_property_change(&mut self, field_key: &str, value: &str) {
        let Some(node_id) = self.primary_selected() else {
            return;
        };

        self.save_undo();
        let value = value.to_string();

        let node = self.tree.node_mut(node_id);
        match field_key {
            "component.content" => {
                if let ComponentDescriptor::Text { content } = &mut node.component {
                    *content = value;
                }
            }
            "component.label" => match &mut node.component {
                ComponentDescriptor::Button { label, .. } => *label = value,
                ComponentDescriptor::Checkbox { label, .. } => *label = value,
                ComponentDescriptor::Switch { label, .. } => *label = value,
                ComponentDescriptor::Radio { label, .. } => *label = value,
                _ => {}
            },
            "component.variant" => {
                if let ComponentDescriptor::Button { variant, .. } = &mut node.component {
                    *variant = match value.to_lowercase().as_str() {
                        "secondary" => ButtonVariant::Secondary,
                        "ghost" => ButtonVariant::Ghost,
                        "danger" => ButtonVariant::Danger,
                        _ => ButtonVariant::Primary,
                    };
                }
            }
            "component.placeholder" => {
                if let ComponentDescriptor::Input { placeholder } = &mut node.component {
                    *placeholder = value;
                }
            }
            "component.checked" => match &mut node.component {
                ComponentDescriptor::Checkbox { checked, .. } => {
                    *checked = value.trim() == "true";
                }
                ComponentDescriptor::Switch { checked, .. } => {
                    *checked = value.trim() == "true";
                }
                _ => {}
            },
            "component.text" => match &mut node.component {
                ComponentDescriptor::Badge { text } => *text = value,
                ComponentDescriptor::Label { text, .. } => *text = value,
                _ => {}
            },
            "component.description" => {
                if let ComponentDescriptor::Label { description, .. } = &mut node.component {
                    *description = value;
                }
            }
            "component.value" => {
                if let ComponentDescriptor::Progress { value: v } = &mut node.component {
                    *v = value.trim().parse::<f32>().unwrap_or(0.0).clamp(0.0, 100.0);
                }
            }
            "component.name" => {
                if let ComponentDescriptor::Avatar { name } = &mut node.component {
                    *name = value;
                }
            }
            "component.selected" => {
                if let ComponentDescriptor::Radio { selected, .. } = &mut node.component {
                    *selected = value.trim() == "true";
                }
            }
            "style.width" => {
                node.style.width = parse_length_value(&value)
                    .filter(|v| !matches!(v, crate::document::LengthValue::Auto));
            }
            "style.height" => {
                node.style.height = parse_length_value(&value)
                    .filter(|v| !matches!(v, crate::document::LengthValue::Auto));
            }
            "style.padding" => {
                node.style.padding = parse_edges_value(&value);
            }
            "style.gap" => {
                if let Some(len) = parse_length_value(&value) {
                    node.style.gap = Some(crate::document::SizeValue {
                        width: Some(len.clone()),
                        height: Some(len),
                    });
                } else {
                    node.style.gap = None;
                }
            }
            _ => {}
        }
    }
}

impl Render for BuilderView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let save_handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)> = {
            let entity = cx.entity().downgrade();
            Rc::new(
                move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                    entity
                        .update(cx, |view, _cx| {
                            view.save();
                        })
                        .ok();
                },
            )
        };

        let undo_handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)> = {
            let entity = cx.entity().downgrade();
            Rc::new(
                move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                    entity
                        .update(cx, |view, cx| {
                            view.undo();
                            cx.notify();
                        })
                        .ok();
                },
            )
        };

        let redo_handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)> = {
            let entity = cx.entity().downgrade();
            Rc::new(
                move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                    entity
                        .update(cx, |view, cx| {
                            view.redo();
                            cx.notify();
                        })
                        .ok();
                },
            )
        };

        let delete_handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)> = {
            let entity = cx.entity().downgrade();
            Rc::new(
                move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                    entity
                        .update(cx, |view, cx| {
                            view.delete_selected();
                            cx.notify();
                        })
                        .ok();
                },
            )
        };

        let key_entity = cx.entity().downgrade();
        let key_entity_up = key_entity.clone();

        let context_menu_el = self.render_context_menu(cx);

        let can_undo = self.history.can_undo();
        let can_redo = self.history.can_redo();
        let can_delete = self.selected.iter().any(|&s| s != self.tree.root);

        let dock_area = self.dock_area.clone();
        let cx = cx.deref_mut();

        div()
            .id("builder-root")
            .flex()
            .flex_col()
            .size_full()
            .bg(mozui::hsla(0.0, 0.0, 0.09, 1.0))
            .text_color(mozui::hsla(0.0, 0.0, 0.85, 1.0))
            .on_key_down(move |event: &mozui::KeyDownEvent, _window, cx| {
                let key = event.keystroke.key.as_str();
                let cmd = event.keystroke.modifiers.platform;
                let shift = event.keystroke.modifiers.shift;

                key_entity
                    .update(cx, |view, cx| match (cmd, shift, key) {
                        (true, false, "z") => {
                            view.undo();
                            cx.notify();
                        }
                        (true, true, "z") => {
                            view.redo();
                            cx.notify();
                        }
                        (true, false, "s") => {
                            view.save();
                        }
                        (true, false, "d") => {
                            view.duplicate_selected();
                            cx.notify();
                        }
                        (true, false, "0") => {
                            view.canvas_state.zoom = 1.0;
                            view.canvas_state.offset = point(px(0.0), px(0.0));
                            cx.notify();
                        }
                        (true, false, "=") | (true, false, "+") => {
                            view.canvas_state.zoom = (view.canvas_state.zoom * 1.2).min(5.0);
                            cx.notify();
                        }
                        (true, false, "-") => {
                            view.canvas_state.zoom = (view.canvas_state.zoom / 1.2).max(0.1);
                            cx.notify();
                        }
                        (false, false, "backspace") | (false, false, "delete") => {
                            view.delete_selected();
                            cx.notify();
                        }
                        (false, false, " ") => {
                            if !view.canvas_state.space_held {
                                view.canvas_state.space_held = true;
                                cx.notify();
                            }
                        }
                        _ => {}
                    })
                    .ok();
            })
            .on_key_up({
                let key_entity = key_entity_up;
                move |event: &mozui::KeyUpEvent, _window, cx| {
                    if event.keystroke.key == " " {
                        key_entity
                            .update(cx, |view, cx| {
                                view.canvas_state.space_held = false;
                                view.canvas_state.interaction = InteractionMode::None;
                                cx.notify();
                            })
                            .ok();
                    }
                }
            })
            .child(dock_area)
            .child(render_toolbar(
                save_handler,
                undo_handler,
                redo_handler,
                delete_handler,
                can_undo,
                can_redo,
                can_delete,
                window,
                cx,
            ))
            .children(context_menu_el)
    }
}

impl BuilderView {
    fn render_context_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.context_menu.as_ref()?;
        let node_id = menu.node_id;
        let is_root = node_id == self.tree.root;
        let pos = menu.position;

        let delete_entity = cx.entity().downgrade();
        let duplicate_entity = cx.entity().downgrade();
        let wrap_entity = cx.entity().downgrade();

        let menu_item = |label: &str, id: &str, enabled: bool| {
            let mut item = div()
                .id(mozui::ElementId::Name(id.to_string().into()))
                .px(px(12.0))
                .py(px(5.0))
                .text_size(px(12.0))
                .rounded(px(3.0));

            if enabled {
                item = item
                    .cursor_pointer()
                    .text_color(mozui::hsla(0.0, 0.0, 0.85, 1.0))
                    .hover(|s| s.bg(mozui::hsla(0.58, 0.6, 0.45, 0.3)));
            } else {
                item = item.text_color(mozui::hsla(0.0, 0.0, 0.35, 1.0));
            }

            item.child(SharedString::from(label.to_string()))
        };

        let menu_el = div()
            .absolute()
            .left(pos.x)
            .top(pos.y)
            .w(px(160.0))
            .bg(mozui::hsla(0.0, 0.0, 0.14, 1.0))
            .border_1()
            .border_color(mozui::hsla(0.0, 0.0, 0.25, 1.0))
            .rounded(px(6.0))
            .py(px(4.0))
            .shadow_lg()
            .child(menu_item("Delete", "ctx-delete", !is_root).on_click(
                move |_event: &ClickEvent, _window, cx| {
                    delete_entity
                        .update(cx, |view, cx| {
                            view.context_menu = None;
                            view.delete_selected();
                            cx.notify();
                        })
                        .ok();
                },
            ))
            .child(menu_item("Duplicate", "ctx-duplicate", !is_root).on_click(
                move |_event: &ClickEvent, _window, cx| {
                    duplicate_entity
                        .update(cx, |view, cx| {
                            view.context_menu = None;
                            view.duplicate_selected();
                            cx.notify();
                        })
                        .ok();
                },
            ))
            .child(
                div()
                    .w_full()
                    .h(px(1.0))
                    .bg(mozui::hsla(0.0, 0.0, 0.25, 1.0))
                    .my(px(4.0)),
            )
            .child(
                menu_item("Wrap in Container", "ctx-wrap", !is_root).on_click(
                    move |_event: &ClickEvent, _window, cx| {
                        wrap_entity
                            .update(cx, |view, cx| {
                                view.context_menu = None;
                                view.wrap_in_container(node_id);
                                cx.notify();
                            })
                            .ok();
                    },
                ),
            )
            .into_any_element();

        Some(menu_el)
    }
}
