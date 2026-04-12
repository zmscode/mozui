use std::collections::HashMap;

use mozui::prelude::*;
use mozui::{AnyElement, App, Entity, SharedString, Subscription, WeakEntity, Window, div, px};
use mozui_components::input::{Input, InputEvent, InputState};
use mozui_components::Sizable;

use crate::document::{
    ComponentDescriptor, DocumentTree, EdgesValue, LengthValue, NodeId,
};

use super::builder_view::BuilderView;

/// Persistent state for the property panel's input fields.
pub struct PropertyPanelState {
    pub inputs: HashMap<&'static str, Entity<InputState>>,
    subscriptions: Vec<Subscription>,
    synced_node: Option<NodeId>,
}

impl PropertyPanelState {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            subscriptions: Vec::new(),
            synced_node: None,
        }
    }

    /// Ensure an input entity exists for a given field key.
    fn get_or_create(
        &mut self,
        key: &'static str,
        placeholder: &str,
        builder: WeakEntity<BuilderView>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<InputState> {
        if let Some(entity) = self.inputs.get(key) {
            return entity.clone();
        }

        let placeholder = SharedString::from(placeholder.to_string());
        let entity = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));

        let field_key = key;
        let sub = cx.subscribe(&entity, {
            let builder = builder.clone();
            let input_entity = entity.clone();
            move |_entity: Entity<InputState>, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change | InputEvent::PressEnter { .. }) {
                    let value = input_entity.read(cx).value().to_string();
                    builder
                        .update(cx, |view, cx| {
                            view.apply_property_change(field_key, &value);
                            cx.notify();
                        })
                        .ok();
                }
            }
        });

        self.subscriptions.push(sub);
        self.inputs.insert(key, entity.clone());
        entity
    }

    /// Sync input values from the document tree for the selected node.
    pub fn sync_from_tree(
        &mut self,
        tree: &DocumentTree,
        selected: Option<NodeId>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.synced_node == selected {
            return;
        }
        self.synced_node = selected;

        let Some(node_id) = selected else { return };
        let node = tree.node(node_id);

        // Component fields
        match &node.component {
            ComponentDescriptor::Text { content } => {
                self.set_input_value("component.content", content, window, cx);
            }
            ComponentDescriptor::Button { label, variant } => {
                self.set_input_value("component.label", label, window, cx);
                self.set_input_value("component.variant", &format!("{variant:?}"), window, cx);
            }
            ComponentDescriptor::Input { placeholder } => {
                self.set_input_value("component.placeholder", placeholder, window, cx);
            }
            ComponentDescriptor::Checkbox { label, checked } => {
                self.set_input_value("component.label", label, window, cx);
                self.set_input_value(
                    "component.checked",
                    if *checked { "true" } else { "false" },
                    window,
                    cx,
                );
            }
            ComponentDescriptor::Badge { text } => {
                self.set_input_value("component.text", text, window, cx);
            }
            ComponentDescriptor::Switch { label, checked } => {
                self.set_input_value("component.label", label, window, cx);
                self.set_input_value(
                    "component.checked",
                    if *checked { "true" } else { "false" },
                    window,
                    cx,
                );
            }
            ComponentDescriptor::Progress { value } => {
                self.set_input_value("component.value", &format!("{value}"), window, cx);
            }
            ComponentDescriptor::Avatar { name } => {
                self.set_input_value("component.name", name, window, cx);
            }
            ComponentDescriptor::Label { text, description } => {
                self.set_input_value("component.text", text, window, cx);
                self.set_input_value("component.description", description, window, cx);
            }
            ComponentDescriptor::Radio { label, selected } => {
                self.set_input_value("component.label", label, window, cx);
                self.set_input_value(
                    "component.selected",
                    if *selected { "true" } else { "false" },
                    window,
                    cx,
                );
            }
            ComponentDescriptor::Container | ComponentDescriptor::Divider => {}
        }

        // Style fields
        let style = &node.style;
        self.set_input_value(
            "style.width",
            &style
                .width
                .as_ref()
                .map(format_length)
                .unwrap_or_else(|| "auto".into()),
            window,
            cx,
        );
        self.set_input_value(
            "style.height",
            &style
                .height
                .as_ref()
                .map(format_length)
                .unwrap_or_else(|| "auto".into()),
            window,
            cx,
        );
        self.set_input_value(
            "style.padding",
            &style
                .padding
                .as_ref()
                .map(edges_to_string)
                .unwrap_or_else(|| "0".into()),
            window,
            cx,
        );
        self.set_input_value(
            "style.gap",
            &style
                .gap
                .as_ref()
                .and_then(|g| g.width.as_ref().map(format_length))
                .unwrap_or_else(|| "0".into()),
            window,
            cx,
        );
    }

    fn set_input_value(
        &self,
        key: &str,
        value: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(entity) = self.inputs.get(key) {
            let value = SharedString::from(value.to_string());
            entity.update(cx, |state, cx| {
                state.set_value(value, window, cx);
            });
        }
    }

    /// Force re-sync on next render (e.g. after undo/redo).
    pub fn invalidate(&mut self) {
        self.synced_node = None;
    }
}

/// Render the property panel for the selected node.
pub fn render_property_panel(
    tree: &DocumentTree,
    selected: Option<NodeId>,
    panel_state: &mut PropertyPanelState,
    builder: WeakEntity<BuilderView>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let Some(node_id) = selected else {
        return div()
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .text_size(px(12.0))
            .text_color(mozui::hsla(0.0, 0.0, 0.4, 1.0))
            .child(SharedString::from("No selection"))
            .into_any_element();
    };

    let node = tree.node(node_id);
    let mut panel = div()
        .flex()
        .flex_col()
        .w_full()
        .gap(px(12.0))
        .p(px(12.0))
        .overflow_y_hidden();

    // Component section
    panel = panel.child(render_section(
        "Component",
        render_component_fields(
            &node.component,
            panel_state,
            builder.clone(),
            window,
            cx,
        ),
    ));

    // Size section
    panel = panel.child(render_section(
        "Size",
        render_size_fields(panel_state, builder.clone(), window, cx),
    ));

    // Spacing section
    panel = panel.child(render_section(
        "Spacing",
        render_spacing_fields(panel_state, builder.clone(), window, cx),
    ));

    // Sync AFTER inputs are created so set_input_value finds them
    panel_state.sync_from_tree(tree, selected, window, cx);

    panel.into_any_element()
}

fn render_section(title: &str, content: AnyElement) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(mozui::FontWeight::BOLD)
                .text_color(mozui::hsla(0.0, 0.0, 0.45, 1.0))
                .child(SharedString::from(title.to_string())),
        )
        .child(content)
        .into_any_element()
}

fn render_input_row(
    label: &str,
    key: &'static str,
    placeholder: &str,
    panel_state: &mut PropertyPanelState,
    builder: WeakEntity<BuilderView>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let input_entity = panel_state.get_or_create(key, placeholder, builder, window, cx);

    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(mozui::hsla(0.0, 0.0, 0.55, 1.0))
                .min_w(px(60.0))
                .child(SharedString::from(label.to_string())),
        )
        .child(
            div().flex_1().child(
                Input::new(&input_entity)
                    .with_size(mozui_components::Size::Small),
            ),
        )
        .into_any_element()
}

fn render_static_row(label: &str, value: &str) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(mozui::hsla(0.0, 0.0, 0.55, 1.0))
                .min_w(px(60.0))
                .child(SharedString::from(label.to_string())),
        )
        .child(
            div()
                .flex_1()
                .px(px(6.0))
                .py(px(3.0))
                .rounded(px(3.0))
                .text_size(px(12.0))
                .text_color(mozui::hsla(0.0, 0.0, 0.8, 1.0))
                .child(SharedString::from(value.to_string())),
        )
        .into_any_element()
}

fn render_component_fields(
    component: &ComponentDescriptor,
    panel_state: &mut PropertyPanelState,
    builder: WeakEntity<BuilderView>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let mut fields = div().flex().flex_col().gap(px(4.0));

    fields = fields.child(render_static_row("Type", component.display_name()));

    match component {
        ComponentDescriptor::Text { .. } => {
            fields = fields.child(render_input_row(
                "Content",
                "component.content",
                "Text content",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Button { .. } => {
            fields = fields.child(render_input_row(
                "Label",
                "component.label",
                "Button label",
                panel_state,
                builder.clone(),
                window,
                cx,
            ));
            fields = fields.child(render_input_row(
                "Variant",
                "component.variant",
                "Primary",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Input { .. } => {
            fields = fields.child(render_input_row(
                "Placeholder",
                "component.placeholder",
                "Placeholder text",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Checkbox { .. } => {
            fields = fields.child(render_input_row(
                "Label",
                "component.label",
                "Label",
                panel_state,
                builder.clone(),
                window,
                cx,
            ));
            fields = fields.child(render_input_row(
                "Checked",
                "component.checked",
                "true/false",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Badge { .. } => {
            fields = fields.child(render_input_row(
                "Text",
                "component.text",
                "Badge text",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Switch { .. } => {
            fields = fields.child(render_input_row(
                "Label",
                "component.label",
                "Label",
                panel_state,
                builder.clone(),
                window,
                cx,
            ));
            fields = fields.child(render_input_row(
                "Checked",
                "component.checked",
                "true/false",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Progress { .. } => {
            fields = fields.child(render_input_row(
                "Value",
                "component.value",
                "0-100",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Avatar { .. } => {
            fields = fields.child(render_input_row(
                "Name",
                "component.name",
                "Name",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Label { .. } => {
            fields = fields.child(render_input_row(
                "Text",
                "component.text",
                "Label text",
                panel_state,
                builder.clone(),
                window,
                cx,
            ));
            fields = fields.child(render_input_row(
                "Description",
                "component.description",
                "Description",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Radio { .. } => {
            fields = fields.child(render_input_row(
                "Label",
                "component.label",
                "Label",
                panel_state,
                builder.clone(),
                window,
                cx,
            ));
            fields = fields.child(render_input_row(
                "Selected",
                "component.selected",
                "true/false",
                panel_state,
                builder,
                window,
                cx,
            ));
        }
        ComponentDescriptor::Container | ComponentDescriptor::Divider => {}
    }

    fields.into_any_element()
}

fn render_size_fields(
    panel_state: &mut PropertyPanelState,
    builder: WeakEntity<BuilderView>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let mut fields = div().flex().flex_col().gap(px(4.0));
    fields = fields.child(render_input_row(
        "Width",
        "style.width",
        "auto",
        panel_state,
        builder.clone(),
        window,
        cx,
    ));
    fields = fields.child(render_input_row(
        "Height",
        "style.height",
        "auto",
        panel_state,
        builder,
        window,
        cx,
    ));
    fields.into_any_element()
}

fn render_spacing_fields(
    panel_state: &mut PropertyPanelState,
    builder: WeakEntity<BuilderView>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let mut fields = div().flex().flex_col().gap(px(4.0));
    fields = fields.child(render_input_row(
        "Padding",
        "style.padding",
        "0",
        panel_state,
        builder.clone(),
        window,
        cx,
    ));
    fields = fields.child(render_input_row(
        "Gap",
        "style.gap",
        "0",
        panel_state,
        builder,
        window,
        cx,
    ));
    fields.into_any_element()
}

fn format_length(v: &LengthValue) -> String {
    match v {
        LengthValue::Auto => "auto".into(),
        LengthValue::Px(n) => format!("{n}px"),
        LengthValue::Rem(n) => format!("{n}rem"),
        LengthValue::Percent(n) => format!("{n}%"),
    }
}

fn edges_to_string(edges: &EdgesValue) -> String {
    let t = edges
        .top
        .as_ref()
        .map(|v| format_length(v))
        .unwrap_or_else(|| "0".into());
    let r = edges
        .right
        .as_ref()
        .map(|v| format_length(v))
        .unwrap_or_else(|| "0".into());
    let b = edges
        .bottom
        .as_ref()
        .map(|v| format_length(v))
        .unwrap_or_else(|| "0".into());
    let l = edges
        .left
        .as_ref()
        .map(|v| format_length(v))
        .unwrap_or_else(|| "0".into());

    if t == r && r == b && b == l {
        t
    } else if t == b && l == r {
        format!("{t} {r}")
    } else {
        format!("{t} {r} {b} {l}")
    }
}

/// Parse a length value string like "100px", "auto", "50%", "2rem"
pub fn parse_length_value(s: &str) -> Option<LengthValue> {
    let s = s.trim();
    if s.is_empty() || s == "auto" {
        return Some(LengthValue::Auto);
    }
    if let Some(rest) = s.strip_suffix("px") {
        rest.trim().parse::<f32>().ok().map(LengthValue::Px)
    } else if let Some(rest) = s.strip_suffix("rem") {
        rest.trim().parse::<f32>().ok().map(LengthValue::Rem)
    } else if let Some(rest) = s.strip_suffix('%') {
        rest.trim().parse::<f32>().ok().map(LengthValue::Percent)
    } else {
        // Bare number → treat as px
        s.parse::<f32>().ok().map(LengthValue::Px)
    }
}

/// Parse a padding/margin shorthand like "10px" or "10px 20px" or "10 20 10 20"
pub fn parse_edges_value(s: &str) -> Option<EdgesValue> {
    let s = s.trim();
    if s == "0" || s.is_empty() {
        return Some(EdgesValue::default());
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    match parts.len() {
        1 => {
            let v = parse_length_value(parts[0])?;
            Some(EdgesValue {
                top: Some(v.clone()),
                right: Some(v.clone()),
                bottom: Some(v.clone()),
                left: Some(v),
            })
        }
        2 => {
            let tb = parse_length_value(parts[0])?;
            let lr = parse_length_value(parts[1])?;
            Some(EdgesValue {
                top: Some(tb.clone()),
                bottom: Some(tb),
                left: Some(lr.clone()),
                right: Some(lr),
            })
        }
        4 => {
            Some(EdgesValue {
                top: Some(parse_length_value(parts[0])?),
                right: Some(parse_length_value(parts[1])?),
                bottom: Some(parse_length_value(parts[2])?),
                left: Some(parse_length_value(parts[3])?),
            })
        }
        _ => None,
    }
}
