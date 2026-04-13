use std::rc::Rc;

use mozui::prelude::*;
use mozui::{
    AnyElement, App, Bounds, ClickEvent, ElementId, MouseButton, MouseDownEvent, Pixels, Point,
    SharedString, Window, canvas, div, point, px, size,
};

use crate::document::{ButtonVariant, ComponentDescriptor, DocumentTree, NodeId};

use super::canvas_state::CanvasState;
use super::drag::{DragPreview, NodeDragData, PaletteDragData};

type SelectHandler = Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>;
type PaletteDropHandler = Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&PaletteDragData, &mut Window, &mut App)>>;
type NodeDropHandler = Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&NodeDragData, &mut Window, &mut App)>>;
type ContextMenuHandler = Rc<dyn Fn(NodeId) -> Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>>;

const DOT_SPACING: f32 = 20.0;
const DOT_SIZE: f32 = 1.5;
const ZOOM_SENSITIVITY: f32 = 0.08;
const ARTBOARD_WIDTH: f32 = 500.0;

pub fn render_canvas(
    tree: &DocumentTree,
    selected: &[NodeId],
    canvas_state: &CanvasState,
    on_select: SelectHandler,
    on_palette_drop: PaletteDropHandler,
    on_node_drop: NodeDropHandler,
    on_context_menu: ContextMenuHandler,
    on_pan: Rc<dyn Fn(PanEvent, &mut Window, &mut App)>,
    on_zoom: Rc<dyn Fn(ZoomEvent, &mut Window, &mut App)>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let zoom = canvas_state.zoom;

    let rendered = render_canvas_node(
        tree,
        tree.root,
        selected,
        zoom,
        &on_select,
        &on_palette_drop,
        &on_node_drop,
        &on_context_menu,
        window,
        cx,
    );

    let offset = canvas_state.offset;
    let is_panning = canvas_state.is_panning();
    let space_held = canvas_state.space_held;

    // Dot grid background — painted via canvas() element
    let dot_grid = canvas(
        move |bounds, _window, _cx| bounds,
        move |bounds: Bounds<Pixels>, _prepaint: Bounds<Pixels>, window, _cx| {
            paint_dot_grid(bounds, offset, window);
        },
    )
    .size_full()
    .absolute()
    .left(px(0.0))
    .top(px(0.0));

    // Document content positioned with pan offset
    // Fixed artboard width scaled by zoom preserves layout ratios (no text reflow)
    let document_content = div()
        .absolute()
        .left(offset.x + px(24.0))
        .top(offset.y + px(24.0))
        .w(px(ARTBOARD_WIDTH * zoom))
        .text_size(px(14.0 * zoom))
        .child(
            div()
                .w_full()
                .border_1()
                .border_color(mozui::hsla(0.0, 0.0, 0.2, 1.0))
                .rounded(px(4.0 * zoom))
                .overflow_hidden()
                .child(rendered),
        );

    // Zoom indicator
    let zoom_pct = format!("{}%", (zoom * 100.0).round() as i32);
    let zoom_label = div()
        .absolute()
        .right(px(12.0))
        .bottom(px(12.0))
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(4.0))
        .bg(mozui::hsla(0.0, 0.0, 0.12, 0.8))
        .text_size(px(11.0))
        .text_color(mozui::hsla(0.0, 0.0, 0.5, 1.0))
        .child(SharedString::from(zoom_pct));

    // Pan/zoom event handlers
    let pan_scroll = on_pan.clone();
    let pan_middle_down = on_pan.clone();
    let pan_middle_move = on_pan.clone();
    let pan_middle_up = on_pan.clone();
    let pan_space_down = on_pan.clone();
    let pan_space_move = on_pan.clone();
    let pan_space_up = on_pan.clone();
    let zoom_scroll = on_zoom.clone();
    let zoom_pinch = on_zoom.clone();

    let mut viewport = div()
        .id("canvas-viewport")
        .relative()
        .w_full()
        .h_full()
        .overflow_hidden()
        .bg(mozui::hsla(0.0, 0.0, 0.06, 1.0))
        .child(dot_grid)
        .child(document_content)
        .child(zoom_label)
        // Scroll wheel: Cmd = zoom, otherwise = pan
        .on_scroll_wheel(move |event: &mozui::ScrollWheelEvent, window, cx| {
            if event.modifiers.platform {
                // Zoom
                let delta_y = match event.delta {
                    mozui::ScrollDelta::Pixels(p) => p.y.as_f32(),
                    mozui::ScrollDelta::Lines(l) => l.y * 20.0,
                };
                zoom_scroll(
                    ZoomEvent {
                        delta: -delta_y * ZOOM_SENSITIVITY / 20.0,
                        cursor: event.position,
                    },
                    window,
                    cx,
                );
                cx.stop_propagation();
            } else {
                // Pan
                let delta = match event.delta {
                    mozui::ScrollDelta::Pixels(p) => p,
                    mozui::ScrollDelta::Lines(l) => point(px(l.x * 20.0), px(l.y * 20.0)),
                };
                pan_scroll(PanEvent::Delta(delta), window, cx);
                cx.stop_propagation();
            }
        })
        // Pinch to zoom
        .on_pinch(move |event: &mozui::PinchEvent, window, cx| {
            zoom_pinch(
                ZoomEvent {
                    delta: event.delta,
                    cursor: event.position,
                },
                window,
                cx,
            );
            cx.stop_propagation();
        })
        // Middle-click pan
        .on_mouse_down(MouseButton::Middle, move |event, window, cx| {
            pan_middle_down(PanEvent::Start(event.position), window, cx);
            cx.stop_propagation();
        })
        .on_mouse_move(move |event: &mozui::MouseMoveEvent, window, cx| {
            if event.pressed_button == Some(MouseButton::Middle) {
                pan_middle_move(PanEvent::Move(event.position), window, cx);
            } else if space_held && event.pressed_button == Some(MouseButton::Left) {
                pan_space_move(PanEvent::Move(event.position), window, cx);
            }
        })
        .on_mouse_up(MouseButton::Middle, move |_event, window, cx| {
            pan_middle_up(PanEvent::End, window, cx);
        });

    // Space+left-click pan: intercept left mouse down when space is held
    if space_held {
        viewport = viewport
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                pan_space_down(PanEvent::Start(event.position), window, cx);
                cx.stop_propagation();
            })
            .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                pan_space_up(PanEvent::End, window, cx);
            });
    }

    if is_panning || space_held {
        viewport = viewport.cursor(mozui::CursorStyle::ClosedHand);
    }

    viewport.into_any_element()
}

fn paint_dot_grid(bounds: Bounds<Pixels>, offset: Point<Pixels>, window: &mut Window) {
    // Keep dot spacing constant on screen (don't scale with zoom)
    let spacing = px(DOT_SPACING);
    let dot_radius = px(DOT_SIZE * 0.5);
    let dot_color = mozui::hsla(0.0, 0.0, 0.35, 0.5);

    let sp = spacing.as_f32();
    // Compute grid start aligned to spacing, offset by pan
    let offset_x = ((offset.x.as_f32() % sp) + sp) % sp;
    let offset_y = ((offset.y.as_f32() % sp) + sp) % sp;

    let mut x = bounds.origin.x + px(offset_x);
    while x < bounds.origin.x + bounds.size.width {
        let mut y = bounds.origin.y + px(offset_y);
        while y < bounds.origin.y + bounds.size.height {
            let dot_bounds = Bounds {
                origin: point(x - dot_radius, y - dot_radius),
                size: size(dot_radius * 2.0, dot_radius * 2.0),
            };
            window.paint_quad(mozui::quad(
                dot_bounds,
                dot_radius,
                dot_color,
                px(0.0),
                mozui::hsla(0.0, 0.0, 0.0, 0.0),
                mozui::BorderStyle::default(),
            ));
            y += spacing;
        }
        x += spacing;
    }
}

/// Events sent from the canvas viewport to the builder view.
pub enum PanEvent {
    Start(Point<Pixels>),
    Move(Point<Pixels>),
    Delta(Point<Pixels>),
    End,
}

pub struct ZoomEvent {
    pub delta: f32,
    pub cursor: Point<Pixels>,
}

// ---- Document node rendering (unchanged) ----

fn node_element_id(node_id: NodeId) -> ElementId {
    ElementId::Name(format!("canvas-node-{node_id:?}").into())
}

fn render_canvas_node(
    tree: &DocumentTree,
    node_id: NodeId,
    selected: &[NodeId],
    zoom: f32,
    on_select: &SelectHandler,
    on_palette_drop: &PaletteDropHandler,
    on_node_drop: &NodeDropHandler,
    on_context_menu: &ContextMenuHandler,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let node = tree.node(node_id);

    if !node.visible {
        return div().into_any_element();
    }

    let is_selected = selected.contains(&node_id);
    let is_container = node.component.can_have_children();
    let is_root = node_id == tree.root;

    let content = render_component_content(
        tree,
        node_id,
        selected,
        zoom,
        on_select,
        on_palette_drop,
        on_node_drop,
        on_context_menu,
        window,
        cx,
    );

    let click_handler = on_select(node_id);
    let right_click_handler = on_context_menu(node_id);
    let mut wrapper = div()
        .id(node_element_id(node_id))
        .relative()
        .cursor_pointer()
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            click_handler(event, window, cx);
        })
        .on_mouse_down(MouseButton::Right, move |event, window, cx| {
            cx.stop_propagation();
            right_click_handler(event, window, cx);
        })
        .child(content);

    if !is_root {
        let display_name = node
            .name
            .clone()
            .unwrap_or_else(|| node.component.display_name().to_string());
        wrapper = wrapper.on_drag(
            NodeDragData { node_id },
            move |_data, _offset, _window, cx| cx.new(|_cx| DragPreview::new(display_name.clone())),
        );
    }

    if is_container {
        let palette_handler = on_palette_drop(node_id);
        let node_handler = on_node_drop(node_id);
        wrapper = wrapper
            .on_drop(move |data: &PaletteDragData, window, cx| {
                cx.stop_propagation();
                palette_handler(data, window, cx);
            })
            .on_drop(move |data: &NodeDragData, window, cx| {
                cx.stop_propagation();
                node_handler(data, window, cx);
            })
            .drag_over::<PaletteDragData>(|s, _data, _window, _cx| {
                s.bg(mozui::hsla(0.58, 0.6, 0.45, 0.1))
                    .border_2()
                    .border_color(mozui::hsla(0.58, 0.8, 0.55, 0.5))
            })
            .drag_over::<NodeDragData>(|s, _data, _window, _cx| {
                s.bg(mozui::hsla(0.42, 0.6, 0.45, 0.1))
                    .border_2()
                    .border_color(mozui::hsla(0.42, 0.8, 0.55, 0.5))
            });
    }

    if is_selected {
        wrapper = wrapper
            .border_2()
            .border_color(mozui::hsla(0.58, 0.8, 0.55, 0.8));
    } else {
        wrapper = wrapper.hover(|s| s.border_1().border_color(mozui::hsla(0.58, 0.6, 0.55, 0.3)));
    }

    wrapper.into_any_element()
}

fn render_component_content(
    tree: &DocumentTree,
    node_id: NodeId,
    selected: &[NodeId],
    zoom: f32,
    on_select: &SelectHandler,
    on_palette_drop: &PaletteDropHandler,
    on_node_drop: &NodeDropHandler,
    on_context_menu: &ContextMenuHandler,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let node = tree.node(node_id);
    let style = node.style.to_style_refinement_zoomed(zoom);

    // Helper to scale px values by zoom
    let zp = |v: f32| px(v * zoom);

    match &node.component {
        ComponentDescriptor::Container => {
            let mut el = div();
            *el.style() = style;
            let children: Vec<NodeId> = node.children.clone();
            for child_id in children {
                el = el.child(render_canvas_node(
                    tree,
                    child_id,
                    selected,
                    zoom,
                    on_select,
                    on_palette_drop,
                    on_node_drop,
                    on_context_menu,
                    window,
                    cx,
                ));
            }
            el.into_any_element()
        }
        ComponentDescriptor::Text { content } => {
            let mut el = div();
            *el.style() = style;
            el = el.child(SharedString::from(content.clone()));
            el.into_any_element()
        }
        ComponentDescriptor::Button { label, variant } => {
            let mut outer = div();
            *outer.style() = style;

            let mut btn = div().px(zp(12.0)).py(zp(6.0)).rounded(zp(4.0));

            btn = match variant {
                ButtonVariant::Primary => btn
                    .bg(mozui::hsla(0.6, 0.7, 0.5, 1.0))
                    .text_color(mozui::hsla(0.0, 0.0, 1.0, 1.0)),
                ButtonVariant::Secondary => btn
                    .bg(mozui::hsla(0.0, 0.0, 0.3, 1.0))
                    .text_color(mozui::hsla(0.0, 0.0, 1.0, 1.0)),
                ButtonVariant::Ghost => btn
                    .bg(mozui::hsla(0.0, 0.0, 0.0, 0.0))
                    .text_color(mozui::hsla(0.0, 0.0, 0.9, 1.0)),
                ButtonVariant::Danger => btn
                    .bg(mozui::hsla(0.0, 0.7, 0.5, 1.0))
                    .text_color(mozui::hsla(0.0, 0.0, 1.0, 1.0)),
            };

            btn = btn.child(SharedString::from(label.clone()));
            outer = outer.child(btn);
            outer.into_any_element()
        }
        ComponentDescriptor::Input { placeholder } => {
            let mut outer = div();
            *outer.style() = style;

            let input_el = div()
                .px(zp(8.0))
                .py(zp(6.0))
                .rounded(zp(4.0))
                .border_1()
                .border_color(mozui::hsla(0.0, 0.0, 0.4, 1.0))
                .bg(mozui::hsla(0.0, 0.0, 0.1, 1.0))
                .text_color(mozui::hsla(0.0, 0.0, 0.5, 1.0))
                .min_w(zp(120.0))
                .child(SharedString::from(placeholder.clone()));

            outer = outer.child(input_el);
            outer.into_any_element()
        }
        ComponentDescriptor::Checkbox { label, checked } => {
            let mut outer = div();
            *outer.style() = style;

            let check_char = if *checked { "\u{2611}" } else { "\u{2610}" };
            let row = div()
                .flex()
                .flex_row()
                .gap(zp(6.0))
                .items_center()
                .child(SharedString::from(check_char.to_string()))
                .child(SharedString::from(label.clone()));

            outer = outer.child(row);
            outer.into_any_element()
        }
        ComponentDescriptor::Badge { text } => {
            let mut outer = div();
            *outer.style() = style;

            let badge = div()
                .px(zp(8.0))
                .py(zp(2.0))
                .rounded(zp(10.0))
                .bg(mozui::hsla(0.6, 0.6, 0.4, 1.0))
                .text_color(mozui::hsla(0.0, 0.0, 1.0, 1.0))
                .text_size(zp(12.0))
                .child(SharedString::from(text.clone()));

            outer = outer.child(badge);
            outer.into_any_element()
        }
        ComponentDescriptor::Divider => {
            let mut outer = div();
            *outer.style() = style;
            outer = outer.child(
                div()
                    .w_full()
                    .h(zp(1.0))
                    .my(zp(4.0))
                    .bg(mozui::hsla(0.0, 0.0, 0.3, 1.0)),
            );
            outer.into_any_element()
        }
        ComponentDescriptor::Switch { label, checked } => {
            let mut outer = div();
            *outer.style() = style;

            let track_color = if *checked {
                mozui::hsla(0.6, 0.7, 0.5, 1.0)
            } else {
                mozui::hsla(0.0, 0.0, 0.3, 1.0)
            };
            let thumb_offset = if *checked { zp(14.0) } else { zp(2.0) };

            let track = div()
                .w(zp(32.0))
                .h(zp(18.0))
                .rounded(zp(9.0))
                .bg(track_color)
                .relative()
                .child(
                    div()
                        .absolute()
                        .top(zp(2.0))
                        .left(thumb_offset)
                        .w(zp(14.0))
                        .h(zp(14.0))
                        .rounded(zp(7.0))
                        .bg(mozui::hsla(0.0, 0.0, 1.0, 1.0)),
                );

            let row = div()
                .flex()
                .flex_row()
                .gap(zp(8.0))
                .items_center()
                .child(track)
                .child(SharedString::from(label.clone()));

            outer = outer.child(row);
            outer.into_any_element()
        }
        ComponentDescriptor::Progress { value } => {
            let mut outer = div();
            *outer.style() = style;

            let pct = value.clamp(0.0, 100.0);
            let bar = div()
                .w_full()
                .h(zp(8.0))
                .rounded(zp(4.0))
                .bg(mozui::hsla(0.0, 0.0, 0.2, 1.0))
                .overflow_hidden()
                .child(
                    div()
                        .h_full()
                        .w(mozui::relative(pct / 100.0))
                        .rounded(zp(4.0))
                        .bg(mozui::hsla(0.6, 0.7, 0.5, 1.0)),
                );

            outer = outer.child(bar);
            outer.into_any_element()
        }
        ComponentDescriptor::Avatar { name } => {
            let mut outer = div();
            *outer.style() = style;

            let initials: String = name
                .split_whitespace()
                .filter_map(|w| w.chars().next())
                .take(2)
                .collect::<String>()
                .to_uppercase();

            let avatar = div()
                .w(zp(36.0))
                .h(zp(36.0))
                .rounded(zp(18.0))
                .bg(mozui::hsla(0.6, 0.4, 0.4, 1.0))
                .text_color(mozui::hsla(0.0, 0.0, 1.0, 1.0))
                .text_size(zp(14.0))
                .flex()
                .items_center()
                .justify_center()
                .child(SharedString::from(initials));

            outer = outer.child(avatar);
            outer.into_any_element()
        }
        ComponentDescriptor::Label { text, description } => {
            let mut outer = div();
            *outer.style() = style;

            let mut col = div().flex().flex_col().gap(zp(2.0));
            col = col.child(
                div()
                    .text_size(zp(14.0))
                    .font_weight(mozui::FontWeight::MEDIUM)
                    .child(SharedString::from(text.clone())),
            );
            if !description.is_empty() {
                col = col.child(
                    div()
                        .text_size(zp(12.0))
                        .text_color(mozui::hsla(0.0, 0.0, 0.55, 1.0))
                        .child(SharedString::from(description.clone())),
                );
            }

            outer = outer.child(col);
            outer.into_any_element()
        }
        ComponentDescriptor::Radio { label, selected } => {
            let mut outer = div();
            *outer.style() = style;

            let circle = div()
                .w(zp(16.0))
                .h(zp(16.0))
                .rounded(zp(8.0))
                .border_2()
                .border_color(if *selected {
                    mozui::hsla(0.6, 0.7, 0.5, 1.0)
                } else {
                    mozui::hsla(0.0, 0.0, 0.4, 1.0)
                })
                .flex()
                .items_center()
                .justify_center()
                .when(*selected, |el| {
                    el.child(
                        div()
                            .w(zp(8.0))
                            .h(zp(8.0))
                            .rounded(zp(4.0))
                            .bg(mozui::hsla(0.6, 0.7, 0.5, 1.0)),
                    )
                });

            let row = div()
                .flex()
                .flex_row()
                .gap(zp(6.0))
                .items_center()
                .child(circle)
                .child(SharedString::from(label.clone()));

            outer = outer.child(row);
            outer.into_any_element()
        }
    }
}
