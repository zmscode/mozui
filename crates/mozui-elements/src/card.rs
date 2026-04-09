use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Shadow, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;
use taffy::Overflow;

const PAD: f32 = 20.0;
const HEADER_GAP: f32 = 4.0;
const SECTION_GAP: f32 = 16.0;
const TITLE_SIZE: f32 = 16.0;
const DESC_SIZE: f32 = 13.0;

/// A card container with optional header (title + description), body, and footer.
pub struct Card {
    title: Option<String>,
    description: Option<String>,
    body: Vec<Box<dyn Element>>,
    footer: Vec<Box<dyn Element>>,
    width: Option<f32>,
    // Theme
    bg: Color,
    fg: Color,
    muted_fg: Color,
    border_color: Color,
    corner_radius: f32,
    shadow: Option<Shadow>,
}

pub fn card(theme: &Theme) -> Card {
    Card {
        title: None,
        description: None,
        body: Vec::new(),
        footer: Vec::new(),
        width: None,
        bg: theme.surface,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        border_color: theme.border,
        corner_radius: theme.radius_lg,
        shadow: Some(theme.shadow_sm),
    }
}

impl Card {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn child(mut self, element: impl Element + 'static) -> Self {
        self.body.push(Box::new(element));
        self
    }

    pub fn footer(mut self, element: impl Element + 'static) -> Self {
        self.footer.push(Box::new(element));
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }

    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    fn has_header(&self) -> bool {
        self.title.is_some() || self.description.is_some()
    }

    fn has_footer(&self) -> bool {
        !self.footer.is_empty()
    }
}

impl Element for Card {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut sections = Vec::new();
        let max_text_width = self.width.map(|w| w - PAD * 2.0);

        // Header section (title + description)
        if self.has_header() {
            let mut header_children = Vec::new();

            if let Some(ref title) = self.title {
                let style = mozui_text::TextStyle {
                    font_size: TITLE_SIZE,
                    ..Default::default()
                };
                let m = mozui_text::measure_text(title, &style, max_text_width, font_system);
                header_children.push(engine.new_leaf(Style {
                    size: Size {
                        width: length(m.width),
                        height: length(m.height),
                    },
                    ..Default::default()
                }));
            }

            if let Some(ref desc) = self.description {
                let style = mozui_text::TextStyle {
                    font_size: DESC_SIZE,
                    ..Default::default()
                };
                let m = mozui_text::measure_text(desc, &style, max_text_width, font_system);
                header_children.push(engine.new_leaf(Style {
                    size: Size {
                        width: length(m.width),
                        height: length(m.height),
                    },
                    ..Default::default()
                }));
            }

            sections.push(engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    gap: Size {
                        width: zero(),
                        height: length(HEADER_GAP),
                    },
                    padding: taffy::Rect {
                        left: length(PAD),
                        right: length(PAD),
                        top: length(PAD),
                        bottom: zero(),
                    },
                    ..Default::default()
                },
                &header_children,
            ));
        }

        // Body section
        if !self.body.is_empty() {
            let body_nodes: Vec<_> = self
                .body
                .iter()
                .map(|child| child.layout(engine, font_system))
                .collect();
            sections.push(engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    gap: Size {
                        width: zero(),
                        height: length(8.0),
                    },
                    padding: taffy::Rect {
                        left: length(PAD),
                        right: length(PAD),
                        top: length(if self.has_header() { SECTION_GAP } else { PAD }),
                        bottom: length(if self.has_footer() { 0.0 } else { PAD }),
                    },
                    ..Default::default()
                },
                &body_nodes,
            ));
        }

        // Footer section
        if self.has_footer() {
            let footer_nodes: Vec<_> = self
                .footer
                .iter()
                .map(|child| child.layout(engine, font_system))
                .collect();
            sections.push(engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    gap: Size {
                        width: length(8.0),
                        height: zero(),
                    },
                    justify_content: Some(JustifyContent::FlexEnd),
                    padding: taffy::Rect {
                        left: length(PAD),
                        right: length(PAD),
                        top: length(SECTION_GAP),
                        bottom: length(PAD),
                    },
                    border: taffy::Rect {
                        left: zero(),
                        right: zero(),
                        top: length(1.0),
                        bottom: zero(),
                    },
                    ..Default::default()
                },
                &footer_nodes,
            ));
        }

        let width = self.width.map(length).unwrap_or(auto());
        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width,
                    height: auto(),
                },
                overflow: taffy::Point {
                    x: Overflow::Hidden,
                    y: Overflow::Visible,
                },
                ..Default::default()
            },
            &sections,
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &FontSystem,
    ) {
        // Card container
        let container = layouts[*index];
        *index += 1;
        let bounds =
            mozui_style::Rect::new(container.x, container.y, container.width, container.height);

        draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
            shadow: self.shadow,
        });

        // Clip content to card bounds
        draw_list.push_clip(bounds);

        // Header
        if self.has_header() {
            let _header = layouts[*index];
            *index += 1;

            if let Some(ref title) = self.title {
                let l = layouts[*index];
                *index += 1;
                draw_list.push(DrawCommand::Text {
                    text: title.clone(),
                    bounds: mozui_style::Rect::new(l.x, l.y, l.width, l.height),
                    font_size: TITLE_SIZE,
                    color: self.fg,
                    weight: 600,
                    italic: false,
                });
            }

            if let Some(ref desc) = self.description {
                let l = layouts[*index];
                *index += 1;
                draw_list.push(DrawCommand::Text {
                    text: desc.clone(),
                    bounds: mozui_style::Rect::new(l.x, l.y, l.width, l.height),
                    font_size: DESC_SIZE,
                    color: self.muted_fg,
                    weight: 400,
                    italic: false,
                });
            }
        }

        // Body
        if !self.body.is_empty() {
            let _body = layouts[*index];
            *index += 1;
            for child in &self.body {
                child.paint(layouts, index, draw_list, interactions, font_system);
            }
        }

        // Footer
        if self.has_footer() {
            let footer_layout = layouts[*index];
            *index += 1;

            // Footer top border
            draw_list.push(DrawCommand::Rect {
                bounds: mozui_style::Rect::new(
                    footer_layout.x,
                    footer_layout.y,
                    footer_layout.width,
                    1.0,
                ),
                background: Fill::Solid(self.border_color),
                corner_radii: Corners::uniform(0.0),
                border: None,
                shadow: None,
            });

            for child in &self.footer {
                child.paint(layouts, index, draw_list, interactions, font_system);
            }
        }

        draw_list.pop_clip();
    }
}
