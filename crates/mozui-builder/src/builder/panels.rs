use std::ops::DerefMut;
use std::rc::Rc;

use mozui::prelude::*;
use mozui::{
    App, ClickEvent, Context, EventEmitter, FocusHandle, Focusable, IntoElement, Render,
    SharedString, Subscription, WeakEntity, Window, div, px,
};
use mozui_components::dock::{Panel, PanelEvent};

use crate::document::NodeId;

use super::builder_view::BuilderView;
use super::canvas::{PanEvent, ZoomEvent, render_canvas};
use super::drag::{NodeDragData, PaletteDragData};
use super::palette::render_palette;
use super::property_panel::{PropertyPanelState, render_property_panel};
use super::tree_view::render_tree_view;

// ── Components (Palette) Panel ──────────────────────────────────────────────

pub struct ComponentsPanel {
    focus_handle: FocusHandle,
}

impl ComponentsPanel {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Panel for ComponentsPanel {
    fn panel_name(&self) -> &'static str {
        "ComponentsPanel"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Components"
    }

    fn closable(&self, _cx: &App) -> bool {
        false
    }

    fn zoomable(&self, _cx: &App) -> Option<mozui_components::dock::PanelControl> {
        None
    }
}

impl EventEmitter<PanelEvent> for ComponentsPanel {}

impl Focusable for ComponentsPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ComponentsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("components-panel")
            .size_full()
            .overflow_y_scroll()
            .child(render_palette(window, cx.deref_mut()))
    }
}

// ── Layers (Tree View) Panel ────────────────────────────────────────────────

pub struct LayersPanel {
    builder: WeakEntity<BuilderView>,
    focus_handle: FocusHandle,
    pub(crate) _subscription: Option<Subscription>,
}

impl LayersPanel {
    pub fn new(builder: WeakEntity<BuilderView>, _window: &mut Window, cx: &mut App) -> Self {
        Self {
            builder,
            focus_handle: cx.focus_handle(),
            _subscription: None,
        }
    }
}

impl Panel for LayersPanel {
    fn panel_name(&self) -> &'static str {
        "LayersPanel"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Layers"
    }

    fn closable(&self, _cx: &App) -> bool {
        false
    }

    fn zoomable(&self, _cx: &App) -> Option<mozui_components::dock::PanelControl> {
        None
    }
}

impl EventEmitter<PanelEvent> for LayersPanel {}

impl Focusable for LayersPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LayersPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(builder) = self.builder.upgrade() else {
            return div().into_any_element();
        };

        let on_select: Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>> = {
            let entity = self.builder.clone();
            Rc::new(move |node_id: NodeId| {
                let entity = entity.clone();
                Rc::new(
                    move |event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                        let shift = event.modifiers().shift;
                        entity
                            .update(cx, |view, cx| {
                                view.context_menu = None;
                                if shift {
                                    view.toggle_select(node_id);
                                } else {
                                    view.select(node_id);
                                }
                                cx.notify();
                            })
                            .ok();
                    },
                )
            })
        };

        let on_node_drop: Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&NodeDragData, &mut Window, &mut App)>> = {
            let entity = self.builder.clone();
            Rc::new(move |target_id: NodeId| {
                let entity = entity.clone();
                Rc::new(
                    move |data: &NodeDragData, _window: &mut Window, cx: &mut App| {
                        entity
                            .update(cx, |view, cx| {
                                view.save_undo();
                                let child_count = view.tree.node(target_id).children.len();
                                if view.tree.move_node(data.node_id, target_id, child_count) {
                                    view.selected = vec![data.node_id];
                                    cx.notify();
                                }
                            })
                            .ok();
                    },
                )
            })
        };

        let cx = cx.deref_mut();
        let tree_view = builder.update(cx, |view, cx| {
            render_tree_view(
                &view.tree,
                &view.selected,
                on_select,
                on_node_drop,
                window,
                cx,
            )
        });

        div()
            .id("layers-panel")
            .size_full()
            .overflow_y_scroll()
            .child(tree_view)
            .into_any_element()
    }
}

// ── Canvas Panel ────────────────────────────────────────────────────────────

pub struct CanvasPanel {
    builder: WeakEntity<BuilderView>,
    focus_handle: FocusHandle,
    pub(crate) _subscription: Option<Subscription>,
}

impl CanvasPanel {
    pub fn new(builder: WeakEntity<BuilderView>, _window: &mut Window, cx: &mut App) -> Self {
        Self {
            builder,
            focus_handle: cx.focus_handle(),
            _subscription: None,
        }
    }
}

impl Panel for CanvasPanel {
    fn panel_name(&self) -> &'static str {
        "CanvasPanel"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Canvas"
    }

    fn closable(&self, _cx: &App) -> bool {
        false
    }

    fn zoomable(&self, _cx: &App) -> Option<mozui_components::dock::PanelControl> {
        None
    }

    fn inner_padding(&self, _cx: &App) -> bool {
        false
    }
}

impl EventEmitter<PanelEvent> for CanvasPanel {}

impl Focusable for CanvasPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CanvasPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(builder) = self.builder.upgrade() else {
            return div().into_any_element();
        };

        let on_select: Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>> = {
            let entity = self.builder.clone();
            Rc::new(move |node_id: NodeId| {
                let entity = entity.clone();
                Rc::new(
                    move |event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                        let shift = event.modifiers().shift;
                        entity
                            .update(cx, |view, cx| {
                                view.context_menu = None;
                                if shift {
                                    view.toggle_select(node_id);
                                } else {
                                    view.select(node_id);
                                }
                                cx.notify();
                            })
                            .ok();
                    },
                )
            })
        };

        let on_palette_drop: Rc<
            dyn Fn(NodeId) -> Rc<dyn Fn(&PaletteDragData, &mut Window, &mut App)>,
        > = {
            let entity = self.builder.clone();
            Rc::new(move |target_id: NodeId| {
                let entity = entity.clone();
                Rc::new(
                    move |data: &PaletteDragData, _window: &mut Window, cx: &mut App| {
                        entity
                            .update(cx, |view, cx| {
                                view.save_undo();
                                let child_count = view.tree.node(target_id).children.len();
                                let new_id = view.tree.insert_child(
                                    target_id,
                                    child_count,
                                    data.component.clone(),
                                    data.style.clone(),
                                    None,
                                );
                                view.selected = vec![new_id];
                                cx.notify();
                            })
                            .ok();
                    },
                )
            })
        };

        let on_node_drop: Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&NodeDragData, &mut Window, &mut App)>> = {
            let entity = self.builder.clone();
            Rc::new(move |target_id: NodeId| {
                let entity = entity.clone();
                Rc::new(
                    move |data: &NodeDragData, _window: &mut Window, cx: &mut App| {
                        entity
                            .update(cx, |view, cx| {
                                view.save_undo();
                                let child_count = view.tree.node(target_id).children.len();
                                if view.tree.move_node(data.node_id, target_id, child_count) {
                                    view.selected = vec![data.node_id];
                                    cx.notify();
                                }
                            })
                            .ok();
                    },
                )
            })
        };

        let on_context_menu: Rc<
            dyn Fn(NodeId) -> Rc<dyn Fn(&mozui::MouseDownEvent, &mut Window, &mut App)>,
        > = {
            let entity = self.builder.clone();
            Rc::new(move |node_id: NodeId| {
                let entity = entity.clone();
                Rc::new(
                    move |event: &mozui::MouseDownEvent, _window: &mut Window, cx: &mut App| {
                        entity
                            .update(cx, |view, cx| {
                                view.selected = vec![node_id];
                                view.context_menu = Some(super::builder_view::ContextMenu {
                                    node_id,
                                    position: event.position,
                                });
                                cx.notify();
                            })
                            .ok();
                    },
                )
            })
        };

        let on_pan: Rc<dyn Fn(PanEvent, &mut Window, &mut App)> = {
            let entity = self.builder.clone();
            Rc::new(move |event: PanEvent, _window: &mut Window, cx: &mut App| {
                entity
                    .update(cx, |view, cx| {
                        view.handle_pan(event);
                        cx.notify();
                    })
                    .ok();
            })
        };

        let on_zoom: Rc<dyn Fn(ZoomEvent, &mut Window, &mut App)> = {
            let entity = self.builder.clone();
            Rc::new(
                move |event: ZoomEvent, _window: &mut Window, cx: &mut App| {
                    entity
                        .update(cx, |view, cx| {
                            view.handle_zoom(event);
                            cx.notify();
                        })
                        .ok();
                },
            )
        };

        // Read file name before entering builder.update
        let file_name = {
            let b = builder.read(cx.deref_mut());
            b.file_path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".into())
        };

        let cx = cx.deref_mut();
        let canvas_el = builder.update(cx, |view, cx| {
            render_canvas(
                &view.tree,
                &view.selected,
                &view.canvas_state,
                on_select,
                on_palette_drop,
                on_node_drop,
                on_context_menu,
                on_pan,
                on_zoom,
                window,
                cx,
            )
        });

        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(16.0))
                    .py(px(8.0))
                    .bg(mozui::hsla(0.0, 0.0, 0.08, 1.0))
                    .border_b_1()
                    .border_color(mozui::hsla(0.0, 0.0, 0.18, 1.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(mozui::hsla(0.0, 0.0, 0.5, 1.0))
                            .child(SharedString::from(file_name)),
                    ),
            )
            .child(canvas_el)
            .into_any_element()
    }
}

// ── Properties Panel ────────────────────────────────────────────────────────

pub struct PropertiesPanel {
    builder: WeakEntity<BuilderView>,
    pub state: PropertyPanelState,
    focus_handle: FocusHandle,
    pub(crate) _subscription: Option<Subscription>,
}

impl PropertiesPanel {
    pub fn new(builder: WeakEntity<BuilderView>, _window: &mut Window, cx: &mut App) -> Self {
        Self {
            builder,
            state: PropertyPanelState::new(),
            focus_handle: cx.focus_handle(),
            _subscription: None,
        }
    }
}

impl Panel for PropertiesPanel {
    fn panel_name(&self) -> &'static str {
        "PropertiesPanel"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Properties"
    }

    fn closable(&self, _cx: &App) -> bool {
        false
    }

    fn zoomable(&self, _cx: &App) -> Option<mozui_components::dock::PanelControl> {
        None
    }
}

impl EventEmitter<PanelEvent> for PropertiesPanel {}

impl Focusable for PropertiesPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PropertiesPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(builder) = self.builder.upgrade() else {
            return div().into_any_element();
        };

        // Clone data out of builder to avoid borrow conflicts
        let (tree, selected) = {
            let b = builder.read(cx.deref_mut());
            (b.tree.clone(), b.primary_selected())
        };

        div()
            .id("properties-panel")
            .size_full()
            .overflow_y_scroll()
            .child(render_property_panel(
                &tree,
                selected,
                &mut self.state,
                self.builder.clone(),
                window,
                cx.deref_mut(),
            ))
            .into_any_element()
    }
}
