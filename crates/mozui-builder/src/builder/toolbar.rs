use std::rc::Rc;

use mozui::prelude::*;
use mozui::{AnyElement, App, ClickEvent, SharedString, Window, div, px};

pub fn render_toolbar(
    on_save: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
    on_undo: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
    on_redo: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
    on_delete: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
    can_undo: bool,
    can_redo: bool,
    can_delete: bool,
    _window: &mut Window,
    _cx: &mut App,
) -> AnyElement {
    let toolbar_button = |label: &str, enabled: bool| {
        let mut btn = div()
            .px(px(10.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .border_1()
            .text_size(px(12.0));

        if enabled {
            btn = btn
                .bg(mozui::hsla(0.0, 0.0, 0.15, 1.0))
                .border_color(mozui::hsla(0.0, 0.0, 0.22, 1.0))
                .text_color(mozui::hsla(0.0, 0.0, 0.75, 1.0))
                .cursor_pointer()
                .hover(|s| s.bg(mozui::hsla(0.0, 0.0, 0.2, 1.0)));
        } else {
            btn = btn
                .bg(mozui::hsla(0.0, 0.0, 0.11, 1.0))
                .border_color(mozui::hsla(0.0, 0.0, 0.16, 1.0))
                .text_color(mozui::hsla(0.0, 0.0, 0.35, 1.0));
        }

        btn.child(SharedString::from(label.to_string()))
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(12.0))
        .py(px(6.0))
        .bg(mozui::hsla(0.0, 0.0, 0.08, 1.0))
        .border_t_1()
        .border_color(mozui::hsla(0.0, 0.0, 0.18, 1.0))
        .child(
            div()
                .id("toolbar-save")
                .child(toolbar_button("Save", true))
                .on_click(move |event, window, cx| {
                    on_save(event, window, cx);
                }),
        )
        .child(div().flex_1())
        .child(
            div()
                .id("toolbar-undo")
                .child(toolbar_button("Undo", can_undo))
                .when(can_undo, |el| {
                    let handler = on_undo.clone();
                    el.on_click(move |event, window, cx| {
                        handler(event, window, cx);
                    })
                }),
        )
        .child(
            div()
                .id("toolbar-redo")
                .child(toolbar_button("Redo", can_redo))
                .when(can_redo, |el| {
                    let handler = on_redo.clone();
                    el.on_click(move |event, window, cx| {
                        handler(event, window, cx);
                    })
                }),
        )
        .child(
            div()
                .w(px(1.0))
                .h(px(16.0))
                .bg(mozui::hsla(0.0, 0.0, 0.22, 1.0)),
        )
        .child(
            div()
                .id("toolbar-delete")
                .child(toolbar_button("Delete", can_delete))
                .when(can_delete, |el| {
                    let handler = on_delete.clone();
                    el.on_click(move |event, window, cx| {
                        handler(event, window, cx);
                    })
                }),
        )
        .into_any_element()
}
