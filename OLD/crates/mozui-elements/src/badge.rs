use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use taffy::prelude::*;

#[derive(Debug, Clone)]
enum BadgeContent {
    Dot,
    Count(u32),
}

pub struct Badge {
    layout_id: LayoutId,
    child_id: LayoutId,
    indicator_id: LayoutId,
    content: BadgeContent,
    max: u32,
    color: Color,
    text_color: Color,
    child: Option<Box<dyn Element>>,
}

pub fn badge(theme: &Theme) -> Badge {
    Badge {
        layout_id: LayoutId::NONE,
        child_id: LayoutId::NONE,
        indicator_id: LayoutId::NONE,
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
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Badge",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let mut children = Vec::new();

        // Child element
        if let Some(ref mut child) = self.child {
            self.child_id = child.layout(cx);
            children.push(self.child_id);
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
                    let measured =
                        mozui_text::measure_text(&text, &text_style, None, cx.font_system);
                    let w = (measured.width + 8.0).max(16.0);
                    (w, 16.0)
                }
            }
        };

        self.indicator_id = cx.new_leaf(Style {
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
        children.push(self.indicator_id);

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                position: Position::Relative,
                ..Default::default()
            },
            &children,
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        // Paint child
        if let Some(ref mut child) = self.child {
            let child_bounds = cx.bounds(self.child_id);
            child.paint(child_bounds, cx);
        }

        // Badge indicator
        let indicator_bounds = cx.bounds(self.indicator_id);

        match &self.content {
            BadgeContent::Dot => {
                let dot_size = 8.0;
                let dot_bounds = Rect::new(
                    indicator_bounds.origin.x,
                    indicator_bounds.origin.y,
                    dot_size,
                    dot_size,
                );
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: dot_bounds,
                    background: Fill::Solid(self.color),
                    corner_radii: Corners::uniform(dot_size / 2.0),
                    border: None,
                    shadow: None, shadows: vec![],
                });
            }
            BadgeContent::Count(n) => {
                if *n > 0 {
                    let text = self.display_text().unwrap_or_default();

                    // Pill background
                    cx.draw_list.push(DrawCommand::Rect {
                        bounds: indicator_bounds,
                        background: Fill::Solid(self.color),
                        corner_radii: Corners::uniform(indicator_bounds.size.height / 2.0),
                        border: None,
                        shadow: None, shadows: vec![],
                    });

                    // Centered text
                    let text_style = mozui_text::TextStyle {
                        font_size: 10.0,
                        color: self.text_color,
                        ..Default::default()
                    };
                    let measured =
                        mozui_text::measure_text(&text, &text_style, None, cx.font_system);
                    let text_x = indicator_bounds.origin.x
                        + (indicator_bounds.size.width - measured.width) / 2.0;
                    let text_y = indicator_bounds.origin.y
                        + (indicator_bounds.size.height - measured.height) / 2.0;
                    cx.draw_list.push(DrawCommand::Text {
                        text,
                        bounds: Rect::new(text_x, text_y, measured.width, measured.height),
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
