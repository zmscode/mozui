use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Shadow, Theme};
use taffy::Overflow;
use taffy::prelude::*;

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
    // Layout IDs
    layout_id: LayoutId,
    section_ids: Vec<LayoutId>,
    title_id: LayoutId,
    desc_id: LayoutId,
    body_child_ids: Vec<LayoutId>,
    footer_child_ids: Vec<LayoutId>,
    footer_section_id: LayoutId,
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
        layout_id: LayoutId::NONE,
        section_ids: Vec::new(),
        title_id: LayoutId::NONE,
        desc_id: LayoutId::NONE,
        body_child_ids: Vec::new(),
        footer_child_ids: Vec::new(),
        footer_section_id: LayoutId::NONE,
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
    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.section_ids.clear();
        self.body_child_ids.clear();
        self.footer_child_ids.clear();
        let max_text_width = self.width.map(|w| w - PAD * 2.0);

        // Header section (title + description)
        if self.has_header() {
            let mut header_children = Vec::new();

            if let Some(ref title) = self.title {
                let style = mozui_text::TextStyle {
                    font_size: TITLE_SIZE,
                    ..Default::default()
                };
                let m = mozui_text::measure_text(title, &style, max_text_width, cx.font_system);
                self.title_id = cx.new_leaf(Style {
                    size: Size {
                        width: length(m.width),
                        height: length(m.height),
                    },
                    ..Default::default()
                });
                header_children.push(self.title_id);
            }

            if let Some(ref desc) = self.description {
                let style = mozui_text::TextStyle {
                    font_size: DESC_SIZE,
                    ..Default::default()
                };
                let m = mozui_text::measure_text(desc, &style, max_text_width, cx.font_system);
                self.desc_id = cx.new_leaf(Style {
                    size: Size {
                        width: length(m.width),
                        height: length(m.height),
                    },
                    ..Default::default()
                });
                header_children.push(self.desc_id);
            }

            let header_id = cx.new_with_children(
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
            );
            self.section_ids.push(header_id);
        }

        // Body section
        if !self.body.is_empty() {
            for i in 0..self.body.len() {
                let child_id = self.body[i].layout(cx);
                self.body_child_ids.push(child_id);
            }
            let body_id = cx.new_with_children(
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
                &self.body_child_ids,
            );
            self.section_ids.push(body_id);
        }

        // Footer section
        if self.has_footer() {
            for i in 0..self.footer.len() {
                let child_id = self.footer[i].layout(cx);
                self.footer_child_ids.push(child_id);
            }
            self.footer_section_id = cx.new_with_children(
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
                &self.footer_child_ids,
            );
            self.section_ids.push(self.footer_section_id);
        }

        let width = self.width.map(length).unwrap_or(auto());
        self.layout_id = cx.new_with_children(
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
            &self.section_ids,
        );
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        cx.draw_list.push(DrawCommand::Rect {
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
        cx.draw_list.push_clip(bounds);

        // Header
        if self.has_header() {
            if let Some(ref title) = self.title {
                let title_bounds = cx.bounds(self.title_id);
                cx.draw_list.push(DrawCommand::Text {
                    text: title.clone(),
                    bounds: title_bounds,
                    font_size: TITLE_SIZE,
                    color: self.fg,
                    weight: 600,
                    italic: false,
                });
            }

            if let Some(ref desc) = self.description {
                let desc_bounds = cx.bounds(self.desc_id);
                cx.draw_list.push(DrawCommand::Text {
                    text: desc.clone(),
                    bounds: desc_bounds,
                    font_size: DESC_SIZE,
                    color: self.muted_fg,
                    weight: 400,
                    italic: false,
                });
            }
        }

        // Body
        if !self.body.is_empty() {
            for i in 0..self.body.len() {
                let child_bounds = cx.bounds(self.body_child_ids[i]);
                self.body[i].paint(child_bounds, cx);
            }
        }

        // Footer
        if self.has_footer() {
            let footer_bounds = cx.bounds(self.footer_section_id);

            // Footer top border
            cx.draw_list.push(DrawCommand::Rect {
                bounds: Rect::new(
                    footer_bounds.origin.x,
                    footer_bounds.origin.y,
                    footer_bounds.size.width,
                    1.0,
                ),
                background: Fill::Solid(self.border_color),
                corner_radii: Corners::uniform(0.0),
                border: None,
                shadow: None,
            });

            for i in 0..self.footer.len() {
                let child_bounds = cx.bounds(self.footer_child_ids[i]);
                self.footer[i].paint(child_bounds, cx);
            }
        }

        cx.draw_list.pop_clip();
    }
}
