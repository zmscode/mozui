use std::ops::DerefMut;

use mozui::prelude::*;
use mozui::{AnyElement, App, Context, SharedString, Window, div, px};

use crate::document::{ButtonVariant, ComponentDescriptor, DocumentTree, NodeId};

pub struct DocumentRenderer {
    pub tree: DocumentTree,
}

impl DocumentRenderer {
    pub fn new(tree: DocumentTree) -> Self {
        Self { tree }
    }

    pub fn render_tree(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        self.render_node(self.tree.root, window, cx)
    }

    fn render_node(&self, node_id: NodeId, _window: &mut Window, _cx: &mut App) -> AnyElement {
        let node = self.tree.node(node_id);

        if !node.visible {
            return div().into_any_element();
        }

        let style = node.style.to_style_refinement();

        match &node.component {
            ComponentDescriptor::Container => {
                let mut el = div();
                *el.style() = style;
                let children: Vec<NodeId> = node.children.clone();
                for child_id in children {
                    el = el.child(self.render_node(child_id, _window, _cx));
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

                // Render a simple styled button representation
                let mut btn = div()
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .cursor_pointer();

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
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(mozui::hsla(0.0, 0.0, 0.4, 1.0))
                    .bg(mozui::hsla(0.0, 0.0, 0.1, 1.0))
                    .text_color(mozui::hsla(0.0, 0.0, 0.5, 1.0))
                    .min_w(px(120.0))
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
                    .gap(px(6.0))
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
                    .px(px(8.0))
                    .py(px(2.0))
                    .rounded(px(10.0))
                    .bg(mozui::hsla(0.6, 0.6, 0.4, 1.0))
                    .text_color(mozui::hsla(0.0, 0.0, 1.0, 1.0))
                    .text_size(px(12.0))
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
                        .h(px(1.0))
                        .my(px(4.0))
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
                let thumb_offset = if *checked { px(14.0) } else { px(2.0) };

                let track = div()
                    .w(px(32.0))
                    .h(px(18.0))
                    .rounded(px(9.0))
                    .bg(track_color)
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top(px(2.0))
                            .left(thumb_offset)
                            .w(px(14.0))
                            .h(px(14.0))
                            .rounded(px(7.0))
                            .bg(mozui::hsla(0.0, 0.0, 1.0, 1.0)),
                    );

                let row = div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
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
                    .h(px(8.0))
                    .rounded(px(4.0))
                    .bg(mozui::hsla(0.0, 0.0, 0.2, 1.0))
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(mozui::relative(pct / 100.0))
                            .rounded(px(4.0))
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
                    .w(px(36.0))
                    .h(px(36.0))
                    .rounded(px(18.0))
                    .bg(mozui::hsla(0.6, 0.4, 0.4, 1.0))
                    .text_color(mozui::hsla(0.0, 0.0, 1.0, 1.0))
                    .text_size(px(14.0))
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

                let mut col = div().flex().flex_col().gap(px(2.0));
                col = col.child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(mozui::FontWeight::MEDIUM)
                        .child(SharedString::from(text.clone())),
                );
                if !description.is_empty() {
                    col = col.child(
                        div()
                            .text_size(px(12.0))
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
                    .w(px(16.0))
                    .h(px(16.0))
                    .rounded(px(8.0))
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
                                .w(px(8.0))
                                .h(px(8.0))
                                .rounded(px(4.0))
                                .bg(mozui::hsla(0.6, 0.7, 0.5, 1.0)),
                        )
                    });

                let row = div()
                    .flex()
                    .flex_row()
                    .gap(px(6.0))
                    .items_center()
                    .child(circle)
                    .child(SharedString::from(label.clone()));

                outer = outer.child(row);
                outer.into_any_element()
            }
        }
    }
}

impl Render for DocumentRenderer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root_id = self.tree.root;
        self.render_node(root_id, window, cx.deref_mut())
    }
}
