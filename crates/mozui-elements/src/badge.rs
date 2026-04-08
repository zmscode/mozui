use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

#[derive(Debug, Clone)]
enum BadgeContent {
    Dot,
    Count(u32),
}

pub struct Badge {
    content: BadgeContent,
    max: u32,
    color: Color,
    text_color: Color,
    child: Option<Box<dyn Element>>,
}

pub fn badge(theme: &Theme) -> Badge {
    Badge {
        content: BadgeContent::Dot,
        max: 99,
        color: theme.danger,
        text_color: theme.danger_foreground,
        child: None,
    }
}

impl Badge {
    pub fn dot(mut self) -> Self {
        self.content = BadgeContent::Dot;
        self
    }

    pub fn count(mut self, n: u32) -> Self {
        self.content = BadgeContent::Count(n);
        self
    }

    pub fn max(mut self, max: u32) -> Self {
        self.max = max;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn child(mut self, child: impl Element + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }

    fn display_text(&self) -> Option<String> {
        match self.content {
            BadgeContent::Dot => None,
            BadgeContent::Count(n) => {
                if n == 0 {
                    None
                } else if n > self.max {
                    Some(format!("{}+", self.max))
                } else {
                    Some(format!("{}", n))
                }
            }
        }
    }
}

impl Element for Badge {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let mut children = Vec::new();

        // Child element
        if let Some(ref child) = self.child {
            children.push(child.layout(engine, font_system));
        }

        // Badge indicator (placeholder leaf for size tracking — we draw it manually)
        let (badge_w, badge_h) = match &self.content {
            BadgeContent::Dot => (8.0, 8.0),
            BadgeContent::Count(n) => {
                if *n == 0 {
                    (0.0, 0.0)
                } else {
                    let text = self.display_text().unwrap_or_default();
                    let text_style = mozui_text::TextStyle {
                        font_size: 10.0,
                        color: self.text_color,
                        ..Default::default()
                    };
                    let measured = mozui_text::measure_text(&text, &text_style, None, font_system);
                    let w = (measured.width + 8.0).max(16.0);
                    (w, 16.0)
                }
            }
        };

        let indicator = engine.new_leaf(Style {
            size: Size {
                width: length(badge_w),
                height: length(badge_h),
            },
            position: Position::Absolute,
            inset: taffy::Rect {
                top: length(-(badge_h / 2.0)),
                right: length(-(badge_w / 2.0)),
                left: auto(),
                bottom: auto(),
            },
            ..Default::default()
        });
        children.push(indicator);

        engine.new_with_children(
            Style {
                display: Display::Flex,
                position: Position::Relative,
                ..Default::default()
            },
            &children,
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
        // Outer container
        let _layout = layouts[*index];
        *index += 1;

        // Paint child
        if let Some(ref child) = self.child {
            child.paint(layouts, index, draw_list, interactions, font_system);
        }

        // Badge indicator
        let indicator_layout = layouts[*index];
        *index += 1;

        match &self.content {
            BadgeContent::Dot => {
                let dot_size = 8.0;
                let dot_bounds = mozui_style::Rect::new(
                    indicator_layout.x,
                    indicator_layout.y,
                    dot_size,
                    dot_size,
                );
                draw_list.push(DrawCommand::Rect {
                    bounds: dot_bounds,
                    background: Fill::Solid(self.color),
                    corner_radii: Corners::uniform(dot_size / 2.0),
                    border: None,
                });
            }
            BadgeContent::Count(n) => {
                if *n > 0 {
                    let text = self.display_text().unwrap_or_default();
                    let bounds = mozui_style::Rect::new(
                        indicator_layout.x,
                        indicator_layout.y,
                        indicator_layout.width,
                        indicator_layout.height,
                    );

                    // Pill background
                    draw_list.push(DrawCommand::Rect {
                        bounds,
                        background: Fill::Solid(self.color),
                        corner_radii: Corners::uniform(bounds.size.height / 2.0),
                        border: None,
                    });

                    // Centered text
                    let text_style = mozui_text::TextStyle {
                        font_size: 10.0,
                        color: self.text_color,
                        ..Default::default()
                    };
                    let measured = mozui_text::measure_text(&text, &text_style, None, font_system);
                    let text_x = bounds.origin.x + (bounds.size.width - measured.width) / 2.0;
                    let text_y = bounds.origin.y + (bounds.size.height - measured.height) / 2.0;
                    draw_list.push(DrawCommand::Text {
                        text,
                        bounds: mozui_style::Rect::new(
                            text_x,
                            text_y,
                            measured.width,
                            measured.height,
                        ),
                        font_size: 10.0,
                        color: self.text_color,
                        weight: 600,
                        italic: false,
                    });
                }
            }
        }
    }
}
