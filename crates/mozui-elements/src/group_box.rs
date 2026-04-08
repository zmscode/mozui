use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupBoxVariant {
    Outline,
    Fill,
}

pub struct GroupBox {
    title: String,
    children: Vec<Box<dyn Element>>,
    variant: GroupBoxVariant,
    border_color: Color,
    fill_color: Color,
    title_color: Color,
    title_bg: Color,
}

pub fn group_box(title: impl Into<String>, theme: &Theme) -> GroupBox {
    GroupBox {
        title: title.into(),
        children: Vec::new(),
        variant: GroupBoxVariant::Outline,
        border_color: theme.border,
        fill_color: theme.surface,
        title_color: theme.foreground,
        title_bg: theme.background,
    }
}

impl GroupBox {
    pub fn child(mut self, element: impl Element + 'static) -> Self {
        self.children.push(Box::new(element));
        self
    }

    pub fn fill(mut self) -> Self {
        self.variant = GroupBoxVariant::Fill;
        self
    }

    pub fn outline(mut self) -> Self {
        self.variant = GroupBoxVariant::Outline;
        self
    }

    fn title_font_size(&self) -> f32 {
        13.0
    }
}

impl Element for GroupBox {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let title_style = mozui_text::TextStyle {
            font_size: self.title_font_size(),
            color: self.title_color,
            ..Default::default()
        };
        let title_measured = mozui_text::measure_text(&self.title, &title_style, None, font_system);

        // Title node (positioned absolutely on the top border)
        let title_node = engine.new_leaf(Style {
            size: Size {
                width: length(title_measured.width + 12.0), // padding
                height: length(title_measured.height),
            },
            position: Position::Absolute,
            inset: taffy::Rect {
                left: length(12.0),
                right: auto(),
                top: length(-(title_measured.height / 2.0)),
                bottom: auto(),
            },
            ..Default::default()
        });

        // Content children
        let child_nodes: Vec<taffy::NodeId> = self
            .children
            .iter()
            .map(|c| c.layout(engine, font_system))
            .collect();

        // Content container
        let content_node = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: taffy::Rect {
                    left: length(16.0),
                    right: length(16.0),
                    top: length(12.0),
                    bottom: length(16.0),
                },
                gap: Size {
                    width: zero(),
                    height: length(8.0),
                },
                ..Default::default()
            },
            &child_nodes,
        );

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                margin: taffy::Rect {
                    left: zero(),
                    right: zero(),
                    top: length(8.0), // space for title overflow
                    bottom: zero(),
                },
                ..Default::default()
            },
            &[title_node, content_node],
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
        let outer = layouts[*index];
        *index += 1;

        let outer_bounds = mozui_style::Rect::new(outer.x, outer.y, outer.width, outer.height);

        // Draw border/fill for the box
        let bg = match self.variant {
            GroupBoxVariant::Fill => self.fill_color,
            GroupBoxVariant::Outline => Color::TRANSPARENT,
        };
        draw_list.push(DrawCommand::Rect {
            bounds: outer_bounds,
            background: Fill::Solid(bg),
            corner_radii: Corners::uniform(8.0),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
        });

        // Title
        let title_layout = layouts[*index];
        *index += 1;
        let title_bounds = mozui_style::Rect::new(
            title_layout.x,
            title_layout.y,
            title_layout.width,
            title_layout.height,
        );

        // Background behind title text to "cut" the border
        draw_list.push(DrawCommand::Rect {
            bounds: title_bounds,
            background: Fill::Solid(self.title_bg),
            corner_radii: Corners::uniform(0.0),
            border: None,
        });

        draw_list.push(DrawCommand::Text {
            text: self.title.clone(),
            bounds: mozui_style::Rect::new(
                title_bounds.origin.x + 6.0,
                title_bounds.origin.y,
                title_bounds.size.width - 12.0,
                title_bounds.size.height,
            ),
            font_size: self.title_font_size(),
            color: self.title_color,
            weight: 600,
            italic: false,
        });

        // Content container
        let _content = layouts[*index];
        *index += 1;

        // Paint children
        for child in &self.children {
            child.paint(layouts, index, draw_list, interactions, font_system);
        }
    }
}
