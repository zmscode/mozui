use crate::div::ScrollOffset;
use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use std::rc::Rc;
use std::cell::Cell;
use taffy::Overflow;
use taffy::prelude::*;

/// Controls when the scrollbar is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarShow {
    /// Show the scrollbar only while actively scrolling (macOS-style).
    #[default]
    Scrolling,
    /// Show the scrollbar when hovering over the container.
    Hover,
    /// Always show the scrollbar.
    Always,
}

/// A scrollable container with a visible scrollbar overlay.
///
/// Wraps child content with vertical overflow scrolling and draws a
/// scrollbar thumb whose size is proportional to the viewport/content ratio.
///
/// ```rust,ignore
/// scroll_container(&theme)
///     .scroll(scroll)
///     .show(ScrollbarShow::Hover)
///     .child(long_content)
/// ```
pub struct ScrollContainer {
    children: Vec<Box<dyn Element>>,
    child_ids: Vec<LayoutId>,
    layout_id: LayoutId,
    scroll: Option<ScrollOffset>,
    show: ScrollbarShow,
    thumb_color: Color,
    track_color: Color,
    thumb_radius: f32,
    thumb_width: f32,
    thumb_margin: f32,
    // Taffy style for the outer container
    taffy_style: Style,
    /// Content height from last paint (for scrollbar calculations).
    content_height: Cell<f32>,
}

pub fn scroll_container(theme: &Theme) -> ScrollContainer {
    ScrollContainer {
        children: Vec::new(),
        child_ids: Vec::new(),
        layout_id: LayoutId::NONE,
        scroll: None,
        show: ScrollbarShow::default(),
        thumb_color: theme.muted_foreground.with_alpha(0.35),
        track_color: Color::TRANSPARENT,
        thumb_radius: 3.0,
        thumb_width: 6.0,
        thumb_margin: 2.0,
        taffy_style: Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            overflow: taffy::Point {
                x: Overflow::Hidden,
                y: Overflow::Scroll,
            },
            flex_grow: 1.0,
            size: taffy::Size {
                width: Dimension::Percent(1.0),
                height: Dimension::Auto,
            },
            ..Default::default()
        },
        content_height: Cell::new(0.0),
    }
}

impl ScrollContainer {
    /// Set the scroll state (create via `cx.use_scroll()`).
    pub fn scroll(mut self, scroll: ScrollOffset) -> Self {
        self.scroll = Some(scroll);
        self
    }

    /// Set when the scrollbar should be visible.
    pub fn show(mut self, show: ScrollbarShow) -> Self {
        self.show = show;
        self
    }

    pub fn thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }

    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    pub fn thumb_width(mut self, width: f32) -> Self {
        self.thumb_width = width;
        self
    }

    // Layout helpers

    pub fn w_full(mut self) -> Self {
        self.taffy_style.size.width = Dimension::Percent(1.0);
        self
    }

    pub fn h_full(mut self) -> Self {
        self.taffy_style.size.height = Dimension::Percent(1.0);
        self
    }

    pub fn w(mut self, w: f32) -> Self {
        self.taffy_style.size.width = length(w);
        self
    }

    pub fn h(mut self, h: f32) -> Self {
        self.taffy_style.size.height = length(h);
        self
    }

    pub fn flex_1(mut self) -> Self {
        self.taffy_style.flex_grow = 1.0;
        self.taffy_style.flex_shrink = 1.0;
        self.taffy_style.flex_basis = Dimension::Percent(0.0);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.taffy_style.gap = taffy::Size {
            width: zero(),
            height: length(gap),
        };
        self
    }

    pub fn p(mut self, p: f32) -> Self {
        self.taffy_style.padding = taffy::Rect {
            left: length(p),
            right: length(p),
            top: length(p),
            bottom: length(p),
        };
        self
    }

    pub fn child(mut self, child: impl Element + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    pub fn children<I, E>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Element + 'static,
    {
        for child in children {
            self.children.push(Box::new(child));
        }
        self
    }
}

impl Element for ScrollContainer {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "ScrollContainer",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.child_ids = self
            .children
            .iter_mut()
            .map(|c| c.layout(cx))
            .collect();

        self.layout_id = cx.new_with_children(self.taffy_style.clone(), &self.child_ids);
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let viewport_height = bounds.size.height;
        let scroll_offset_y = self.scroll.as_ref().map(|s| s.get()).unwrap_or(0.0);
        let is_scrollable = self.scroll.is_some();

        // Apply scroll offset for children
        if is_scrollable {
            cx.draw_list.push_scroll_offset(-scroll_offset_y);
            cx.interactions.push_scroll_offset(-scroll_offset_y);
        }

        // Paint children
        for i in 0..self.children.len() {
            let child_bounds = cx.bounds(self.child_ids[i]);
            self.children[i].paint(child_bounds, cx);
        }

        if is_scrollable {
            cx.draw_list.pop_scroll_offset();
            cx.interactions.pop_scroll_offset();
        }

        // Calculate content height and register scroll region
        if let Some(ref scroll) = self.scroll {
            let mut content_bottom = 0.0_f32;
            for &child_id in &self.child_ids {
                let cl = cx.engine.bounds(child_id);
                let bot = cl.y + cl.height - bounds.origin.y;
                content_bottom = content_bottom.max(bot);
            }

            let max_scroll = (content_bottom - viewport_height).max(0.0);
            self.content_height.set(content_bottom);

            // Clamp
            let clamped = scroll_offset_y.clamp(0.0, max_scroll);
            if clamped != scroll_offset_y {
                scroll.set(clamped);
            }

            scroll.tick_momentum(max_scroll);

            let scroll_clone = scroll.clone();
            cx.interactions.register_scroll_region(
                bounds,
                Rc::new(move |_dx, dy, _cx: &mut dyn std::any::Any| {
                    scroll_clone.scroll_by(dy, max_scroll);
                }),
            );

            // Draw scrollbar
            if max_scroll > 0.0 {
                let show_scrollbar = match self.show {
                    ScrollbarShow::Always => true,
                    ScrollbarShow::Hover => cx.interactions.is_hovered(bounds),
                    ScrollbarShow::Scrolling => scroll.has_momentum() || scroll_offset_y != clamped,
                };

                if show_scrollbar {
                    self.paint_scrollbar(
                        cx.draw_list,
                        bounds,
                        viewport_height,
                        content_bottom,
                        scroll_offset_y,
                    );
                }

                // Register hover region for hover-mode scrollbar
                if self.show == ScrollbarShow::Hover {
                    cx.interactions.register_hover_region(bounds);
                }
            }
        }
    }
}

impl ScrollContainer {
    fn paint_scrollbar(
        &self,
        draw_list: &mut mozui_renderer::DrawList,
        bounds: Rect,
        viewport_height: f32,
        content_height: f32,
        scroll_offset: f32,
    ) {
        let margin = self.thumb_margin;
        let track_x = bounds.origin.x + bounds.size.width - self.thumb_width - margin;
        let track_y = bounds.origin.y + margin;
        let track_h = viewport_height - margin * 2.0;

        // Draw track background if not transparent
        if self.track_color.a > 0.0 {
            draw_list.push(DrawCommand::Rect {
                bounds: Rect::new(track_x, track_y, self.thumb_width, track_h),
                background: Fill::Solid(self.track_color),
                corner_radii: Corners::uniform(self.thumb_radius),
                border: None,
                shadow: None,
            });
        }

        // Thumb
        let ratio = viewport_height / content_height;
        let thumb_h = (track_h * ratio).max(20.0); // minimum thumb size
        let max_scroll = content_height - viewport_height;
        let scroll_ratio = if max_scroll > 0.0 {
            scroll_offset / max_scroll
        } else {
            0.0
        };
        let thumb_y = track_y + scroll_ratio * (track_h - thumb_h);

        draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(track_x, thumb_y, self.thumb_width, thumb_h),
            background: Fill::Solid(self.thumb_color),
            corner_radii: Corners::uniform(self.thumb_radius),
            border: None,
            shadow: None,
        });
    }
}
