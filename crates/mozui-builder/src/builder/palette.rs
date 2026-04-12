use mozui::prelude::*;
use mozui::{AnyElement, App, ElementId, SharedString, Window, div, px};

use crate::document::{
    ButtonVariant, ComponentDescriptor, DisplayMode, EdgesValue, FlexDir, LengthValue,
    StyleDescriptor,
};

use super::drag::{DragPreview, PaletteDragData};

struct PaletteItem {
    label: &'static str,
    icon_char: &'static str,
    component: ComponentDescriptor,
    style: StyleDescriptor,
}

struct PaletteSection {
    title: &'static str,
    items: Vec<PaletteItem>,
}

fn palette_sections() -> Vec<PaletteSection> {
    vec![
        PaletteSection {
            title: "Layout",
            items: vec![PaletteItem {
                label: "Container",
                icon_char: "\u{25A1}",
                component: ComponentDescriptor::Container,
                style: StyleDescriptor {
                    display: Some(DisplayMode::Flex),
                    flex_direction: Some(FlexDir::Column),
                    padding: Some(EdgesValue::all(LengthValue::Px(8.0))),
                    min_width: Some(LengthValue::Px(48.0)),
                    min_height: Some(LengthValue::Px(48.0)),
                    ..Default::default()
                },
            }],
        },
        PaletteSection {
            title: "Content",
            items: vec![
                PaletteItem {
                    label: "Text",
                    icon_char: "T",
                    component: ComponentDescriptor::Text {
                        content: "Text".into(),
                    },
                    style: StyleDescriptor::default(),
                },
                PaletteItem {
                    label: "Label",
                    icon_char: "\u{1D4DB}",
                    component: ComponentDescriptor::Label {
                        text: "Label".into(),
                        description: String::new(),
                    },
                    style: StyleDescriptor::default(),
                },
                PaletteItem {
                    label: "Badge",
                    icon_char: "\u{25CF}",
                    component: ComponentDescriptor::Badge {
                        text: "Badge".into(),
                    },
                    style: StyleDescriptor::default(),
                },
                PaletteItem {
                    label: "Avatar",
                    icon_char: "\u{25D0}",
                    component: ComponentDescriptor::Avatar {
                        name: "User".into(),
                    },
                    style: StyleDescriptor::default(),
                },
                PaletteItem {
                    label: "Divider",
                    icon_char: "\u{2500}",
                    component: ComponentDescriptor::Divider,
                    style: StyleDescriptor::default(),
                },
            ],
        },
        PaletteSection {
            title: "Form",
            items: vec![
                PaletteItem {
                    label: "Button",
                    icon_char: "\u{25A3}",
                    component: ComponentDescriptor::Button {
                        label: "Button".into(),
                        variant: ButtonVariant::Primary,
                    },
                    style: StyleDescriptor::default(),
                },
                PaletteItem {
                    label: "Input",
                    icon_char: "\u{2336}",
                    component: ComponentDescriptor::Input {
                        placeholder: "Placeholder...".into(),
                    },
                    style: StyleDescriptor::default(),
                },
                PaletteItem {
                    label: "Checkbox",
                    icon_char: "\u{2611}",
                    component: ComponentDescriptor::Checkbox {
                        label: "Checkbox".into(),
                        checked: false,
                    },
                    style: StyleDescriptor::default(),
                },
                PaletteItem {
                    label: "Switch",
                    icon_char: "\u{25C9}",
                    component: ComponentDescriptor::Switch {
                        label: "Toggle".into(),
                        checked: false,
                    },
                    style: StyleDescriptor::default(),
                },
                PaletteItem {
                    label: "Radio",
                    icon_char: "\u{25CE}",
                    component: ComponentDescriptor::Radio {
                        label: "Option".into(),
                        selected: false,
                    },
                    style: StyleDescriptor::default(),
                },
            ],
        },
        PaletteSection {
            title: "Feedback",
            items: vec![PaletteItem {
                label: "Progress",
                icon_char: "\u{25AE}",
                component: ComponentDescriptor::Progress { value: 50.0 },
                style: StyleDescriptor::default(),
            }],
        },
    ]
}

pub fn render_palette(_window: &mut Window, _cx: &mut App) -> AnyElement {
    let sections = palette_sections();

    let mut list = div().flex().flex_col().w_full();

    list = list.child(
        div()
            .px(px(12.0))
            .py(px(8.0))
            .text_size(px(11.0))
            .font_weight(mozui::FontWeight::BOLD)
            .text_color(mozui::hsla(0.0, 0.0, 0.45, 1.0))
            .child(SharedString::from("COMPONENTS")),
    );

    let mut idx = 0;
    for section in sections {
        // Section header
        list = list.child(
            div()
                .px(px(12.0))
                .pt(px(8.0))
                .pb(px(4.0))
                .text_size(px(10.0))
                .font_weight(mozui::FontWeight::SEMIBOLD)
                .text_color(mozui::hsla(0.0, 0.0, 0.35, 1.0))
                .child(SharedString::from(section.title)),
        );

        for item in section.items {
            let drag_data = PaletteDragData {
                component: item.component,
                style: item.style,
            };
            let label_for_preview = item.label.to_string();

            let row = div()
                .id(ElementId::Name(format!("palette-{idx}").into()))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(10.0))
                .py(px(5.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .hover(|s| s.bg(mozui::hsla(0.0, 0.0, 1.0, 0.06)))
                .on_drag(drag_data, move |_data, _offset, _window, cx| {
                    cx.new(|_cx| DragPreview::new(label_for_preview.clone()))
                })
                .child(
                    div()
                        .w(px(20.0))
                        .flex()
                        .justify_center()
                        .text_size(px(12.0))
                        .text_color(mozui::hsla(0.0, 0.0, 0.5, 1.0))
                        .child(SharedString::from(item.icon_char)),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(mozui::hsla(0.0, 0.0, 0.8, 1.0))
                        .child(SharedString::from(item.label)),
                );

            list = list.child(row);
            idx += 1;
        }
    }

    list.into_any_element()
}
