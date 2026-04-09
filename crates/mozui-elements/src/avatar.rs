use crate::styled::{ComponentSize, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::IconName;
use mozui_layout::LayoutId;
use mozui_renderer::{DrawCommand, ImageData};
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use std::sync::Arc;
use taffy::prelude::*;

/// Status indicator for an avatar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarStatus {
    Online,
    Offline,
    Away,
    Busy,
}

impl AvatarStatus {
    fn color(&self, theme: &Theme) -> Color {
        match self {
            Self::Online => theme.success,
            Self::Offline => theme.muted_foreground,
            Self::Away => theme.warning,
            Self::Busy => theme.danger,
        }
    }
}

/// Content to display in the avatar.
enum AvatarContent {
    /// Show initials (1-2 characters).
    Initials(String),
    /// Show an image.
    Image(Arc<ImageData>),
    /// Show an icon fallback.
    Icon,
}

pub struct Avatar {
    content: AvatarContent,
    size: f32,
    bg: Color,
    fg: Color,
    border_color: Color,
    status: Option<AvatarStatus>,
    status_color: Option<Color>,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    layout_id: LayoutId,
    circle_id: LayoutId,
    dot_id: LayoutId,
}

pub fn avatar(theme: &Theme) -> Avatar {
    Avatar {
        content: AvatarContent::Icon,
        size: 40.0,
        bg: theme.muted,
        fg: theme.muted_foreground,
        border_color: theme.border,
        status: None,
        status_color: None,
        on_click: None,
        layout_id: LayoutId::NONE,
        circle_id: LayoutId::NONE,
        dot_id: LayoutId::NONE,
    }
}

impl Avatar {
    /// Set initials to display (e.g. "JD" for "John Doe").
    pub fn initials(mut self, initials: impl Into<String>) -> Self {
        let s: String = initials.into();
        // Take at most 2 characters
        let truncated: String = s.chars().take(2).collect();
        self.content = AvatarContent::Initials(truncated.to_uppercase());
        self
    }

    /// Set an image to display.
    pub fn image(mut self, data: Arc<ImageData>) -> Self {
        self.content = AvatarContent::Image(data);
        self
    }

    /// Set the avatar size in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set the background color.
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }

    /// Set the foreground (text/icon) color.
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Add a status indicator dot.
    pub fn status(mut self, status: AvatarStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Override the status indicator color.
    pub fn status_color(mut self, color: Color) -> Self {
        self.status_color = Some(color);
        self
    }

    /// Set a click handler.
    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn font_size(&self) -> f32 {
        self.size * 0.4
    }

    fn icon_size(&self) -> f32 {
        self.size * 0.5
    }

    fn status_dot_size(&self) -> f32 {
        (self.size * 0.25).max(8.0)
    }
}

impl Sizable for Avatar {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        let cs: ComponentSize = size.into();
        self.size = cs.input_height();
        self
    }
}

impl Element for Avatar {
    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let mut children = Vec::new();

        // Main circle
        self.circle_id = cx.new_leaf(Style {
            size: Size {
                width: length(self.size),
                height: length(self.size),
            },
            ..Default::default()
        });
        children.push(self.circle_id);

        // Status dot (absolute positioned)
        if self.status.is_some() {
            let dot = self.status_dot_size();
            self.dot_id = cx.new_leaf(Style {
                size: Size {
                    width: length(dot),
                    height: length(dot),
                },
                position: Position::Absolute,
                inset: taffy::Rect {
                    top: auto(),
                    right: length(0.0),
                    left: auto(),
                    bottom: length(0.0),
                },
                ..Default::default()
            });
            children.push(self.dot_id);
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                position: Position::Relative,
                size: Size {
                    width: length(self.size),
                    height: length(self.size),
                },
                ..Default::default()
            },
            &children,
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        let bounds = cx.bounds(self.circle_id);
        let radius = self.size / 2.0;

        match &self.content {
            AvatarContent::Image(data) => {
                // Draw image in circle
                cx.draw_list.push(DrawCommand::Image {
                    bounds,
                    data: data.clone(),
                    corner_radii: Corners::uniform(radius),
                    opacity: 1.0,
                    object_fit: mozui_renderer::ObjectFit::Cover,
                });
            }
            AvatarContent::Initials(text) => {
                // Background circle
                cx.draw_list.push(DrawCommand::Rect {
                    bounds,
                    background: Fill::Solid(self.bg),
                    corner_radii: Corners::uniform(radius),
                    border: None,
                    shadow: None,
                });

                // Centered initials text
                let font_sz = self.font_size();
                let text_style = mozui_text::TextStyle {
                    font_size: font_sz,
                    color: self.fg,
                    weight: mozui_text::FontWeight::Bold,
                    ..Default::default()
                };
                let measured = mozui_text::measure_text(text, &text_style, None, cx.font_system);
                let tx = bounds.origin.x + (bounds.size.width - measured.width) / 2.0;
                let ty = bounds.origin.y + (bounds.size.height - measured.height) / 2.0;
                cx.draw_list.push(DrawCommand::Text {
                    text: text.clone(),
                    bounds: Rect::new(tx, ty, measured.width, measured.height),
                    font_size: font_sz,
                    color: self.fg,
                    weight: 700,
                    italic: false,
                });
            }
            AvatarContent::Icon => {
                // Background circle
                cx.draw_list.push(DrawCommand::Rect {
                    bounds,
                    background: Fill::Solid(self.bg),
                    corner_radii: Corners::uniform(radius),
                    border: None,
                    shadow: None,
                });

                // Centered icon
                let icon_sz = self.icon_size();
                let icon_x = bounds.origin.x + (bounds.size.width - icon_sz) / 2.0;
                let icon_y = bounds.origin.y + (bounds.size.height - icon_sz) / 2.0;
                cx.draw_list.push(DrawCommand::Icon {
                    name: IconName::User,
                    weight: mozui_icons::IconWeight::Regular,
                    bounds: Rect::new(icon_x, icon_y, icon_sz, icon_sz),
                    color: self.fg,
                    size_px: icon_sz,
                });
            }
        }

        // Click handler
        if let Some(ref handler) = self.on_click {
            let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            cx.interactions
                .register_click(bounds, Box::new(move |cx| unsafe { (*handler_ptr)(cx) }));
            cx.interactions.register_hover_region(bounds);
        }

        // Status dot
        if let Some(status) = self.status {
            let dot_bounds = cx.bounds(self.dot_id);
            let dot_size = self.status_dot_size();

            // White ring behind the dot
            let ring_size = dot_size + 2.0;
            let ring_bounds = Rect::new(
                dot_bounds.origin.x - 1.0,
                dot_bounds.origin.y - 1.0,
                ring_size,
                ring_size,
            );
            cx.draw_list.push(DrawCommand::Rect {
                bounds: ring_bounds,
                background: Fill::Solid(self.border_color),
                corner_radii: Corners::uniform(ring_size / 2.0),
                border: None,
                shadow: None,
            });

            let color = self
                .status_color
                .unwrap_or_else(|| status.color(&mozui_style::Theme::dark()));
            cx.draw_list.push(DrawCommand::Rect {
                bounds: dot_bounds,
                background: Fill::Solid(color),
                corner_radii: Corners::uniform(dot_size / 2.0),
                border: None,
                shadow: None,
            });
        }
    }
}
