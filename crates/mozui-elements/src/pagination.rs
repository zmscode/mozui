use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct Pagination {
    current_page: usize,
    total_pages: usize,
    visible_pages: usize,
    compact: bool,
    disabled: bool,
    size: ComponentSize,
    fg: Color,
    active_bg: Color,
    active_fg: Color,
    hover_bg: Color,
    border_color: Color,
    on_click: Option<Box<dyn Fn(usize, &mut dyn std::any::Any)>>,
}

pub fn pagination(theme: &Theme) -> Pagination {
    Pagination {
        current_page: 1,
        total_pages: 1,
        visible_pages: 5,
        compact: false,
        disabled: false,
        size: ComponentSize::Medium,
        fg: theme.foreground,
        active_bg: theme.primary,
        active_fg: theme.primary_foreground,
        hover_bg: theme.secondary,
        border_color: theme.border,
        on_click: None,
    }
}

impl Pagination {
    pub fn current_page(mut self, page: usize) -> Self {
        self.current_page = page;
        self
    }

    pub fn total_pages(mut self, pages: usize) -> Self {
        self.total_pages = pages;
        self
    }

    pub fn visible_pages(mut self, count: usize) -> Self {
        self.visible_pages = count;
        self
    }

    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(usize, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn button_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 24.0,
            ComponentSize::Small => 28.0,
            ComponentSize::Medium => 32.0,
            ComponentSize::Large => 38.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }

    fn text_size(&self) -> f32 {
        self.size.input_text_size()
    }

    fn icon_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 12.0,
            ComponentSize::Small => 14.0,
            ComponentSize::Medium => 16.0,
            ComponentSize::Large => 18.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }

    /// Compute which page numbers to display, with ellipsis markers as `0`.
    fn page_items(&self) -> Vec<usize> {
        if self.compact {
            return vec![];
        }
        let total = self.total_pages;
        let vis = self.visible_pages;
        if total <= vis {
            return (1..=total).collect();
        }

        let half = vis / 2;
        let current = self.current_page.clamp(1, total);

        let mut items = Vec::new();
        items.push(1);

        let start = if current <= half + 1 {
            2
        } else if current >= total - half {
            total - vis + 2
        } else {
            current - half + 1
        };
        let end = (start + vis - 3).min(total - 1);

        if start > 2 {
            items.push(0); // ellipsis
        }
        for p in start..=end {
            items.push(p);
        }
        if end < total - 1 {
            items.push(0); // ellipsis
        }
        items.push(total);
        items
    }
}

impl Sizable for Pagination {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Pagination {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Element for Pagination {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let btn_sz = self.button_size();
        let mut children = Vec::new();

        // Prev button
        children.push(engine.new_leaf(Style {
            size: Size {
                width: length(btn_sz),
                height: length(btn_sz),
            },
            ..Default::default()
        }));

        if self.compact {
            // "Page X of Y" text
            let label = format!("Page {} of {}", self.current_page, self.total_pages);
            let text_style = mozui_text::TextStyle {
                font_size: self.text_size(),
                color: self.fg,
                ..Default::default()
            };
            let measured = mozui_text::measure_text(&label, &text_style, None, font_system);
            children.push(engine.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
                },
                ..Default::default()
            }));
        } else {
            // Page number buttons + ellipsis
            for item in self.page_items() {
                if item == 0 {
                    // Ellipsis
                    let text_style = mozui_text::TextStyle {
                        font_size: self.text_size(),
                        color: self.fg,
                        ..Default::default()
                    };
                    let measured = mozui_text::measure_text("…", &text_style, None, font_system);
                    children.push(engine.new_leaf(Style {
                        size: Size {
                            width: length(measured.width.max(btn_sz * 0.5)),
                            height: length(btn_sz),
                        },
                        ..Default::default()
                    }));
                } else {
                    children.push(engine.new_leaf(Style {
                        size: Size {
                            width: length(btn_sz),
                            height: length(btn_sz),
                        },
                        ..Default::default()
                    }));
                }
            }
        }

        // Next button
        children.push(engine.new_leaf(Style {
            size: Size {
                width: length(btn_sz),
                height: length(btn_sz),
            },
            ..Default::default()
        }));

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: Size {
                    width: length(4.0),
                    height: zero(),
                },
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
        let _outer = layouts[*index];
        *index += 1;

        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let _btn_sz = self.button_size();
        let radius = 6.0;
        let current = self.current_page;

        // Helper: paint a page button
        let paint_button = |index: &mut usize,
                            draw_list: &mut DrawList,
                            interactions: &mut InteractionMap,
                            page: Option<usize>,
                            icon: Option<IconName>,
                            label: Option<&str>,
                            is_active: bool,
                            enabled: bool| {
            let lay = layouts[*index];
            *index += 1;
            let bounds = mozui_style::Rect::new(lay.x, lay.y, lay.width, lay.height);

            let hovered = enabled && interactions.is_hovered(bounds);
            let bg = if is_active {
                self.active_bg.with_alpha(alpha)
            } else if hovered {
                self.hover_bg.with_alpha(alpha)
            } else {
                Color::TRANSPARENT
            };
            let fg = if is_active {
                self.active_fg.with_alpha(alpha)
            } else {
                self.fg.with_alpha(alpha)
            };

            // Border
            if bg.a > 0.0 {
                draw_list.push(DrawCommand::Rect {
                    bounds,
                    background: Fill::Solid(bg),
                    corner_radii: Corners::uniform(radius),
                    border: None,
                });
            } else {
                draw_list.push(DrawCommand::Rect {
                    bounds,
                    background: Fill::Solid(Color::TRANSPARENT),
                    corner_radii: Corners::uniform(radius),
                    border: Some(Border {
                        width: 1.0,
                        color: self.border_color.with_alpha(alpha * 0.5),
                    }),
                });
            }

            if let Some(icon_name) = icon {
                let icon_sz = self.icon_size();
                let ix = bounds.origin.x + (bounds.size.width - icon_sz) / 2.0;
                let iy = bounds.origin.y + (bounds.size.height - icon_sz) / 2.0;
                draw_list.push(DrawCommand::Icon {
                    name: icon_name,
                    weight: IconWeight::Regular,
                    bounds: mozui_style::Rect::new(ix, iy, icon_sz, icon_sz),
                    color: fg,
                    size_px: icon_sz,
                });
            }

            if let Some(text) = label {
                let font_size = self.text_size();
                let text_style = mozui_text::TextStyle {
                    font_size,
                    color: fg,
                    ..Default::default()
                };
                let measured = mozui_text::measure_text(text, &text_style, None, font_system);
                let text_x = bounds.origin.x + (bounds.size.width - measured.width) / 2.0;
                let text_y = bounds.origin.y + (bounds.size.height - measured.height) / 2.0;
                draw_list.push(DrawCommand::Text {
                    text: text.to_string(),
                    bounds: mozui_style::Rect::new(text_x, text_y, measured.width, measured.height),
                    font_size,
                    color: fg,
                    weight: if is_active { 600 } else { 400 },
                    italic: false,
                });
            }

            // Click handler
            if enabled {
                if let (Some(page), Some(handler)) = (page, &self.on_click) {
                    let handler_ptr =
                        handler.as_ref() as *const dyn Fn(usize, &mut dyn std::any::Any);
                    interactions.register_click(
                        bounds,
                        Box::new(move |cx| unsafe { (*handler_ptr)(page, cx) }),
                    );
                }
            }
        };

        let prev_enabled = !self.disabled && current > 1;
        paint_button(
            index,
            draw_list,
            interactions,
            Some(current.saturating_sub(1).max(1)),
            Some(IconName::CaretLeft),
            None,
            false,
            prev_enabled,
        );

        if self.compact {
            // Compact label
            let lay = layouts[*index];
            *index += 1;
            let bounds = mozui_style::Rect::new(lay.x, lay.y, lay.width, lay.height);
            let label = format!("Page {} of {}", current, self.total_pages);
            draw_list.push(DrawCommand::Text {
                text: label,
                bounds,
                font_size: self.text_size(),
                color: self.fg.with_alpha(alpha),
                weight: 500,
                italic: false,
            });
        } else {
            for item in self.page_items() {
                if item == 0 {
                    // Ellipsis
                    let lay = layouts[*index];
                    *index += 1;
                    let bounds = mozui_style::Rect::new(lay.x, lay.y, lay.width, lay.height);
                    let ellipsis_style = mozui_text::TextStyle {
                        font_size: self.text_size(),
                        color: self.fg.with_alpha(alpha * 0.6),
                        ..Default::default()
                    };
                    let measured = mozui_text::measure_text("…", &ellipsis_style, None, font_system);
                    let ex = bounds.origin.x + (bounds.size.width - measured.width) / 2.0;
                    let ey = bounds.origin.y + (bounds.size.height - measured.height) / 2.0;
                    draw_list.push(DrawCommand::Text {
                        text: "…".to_string(),
                        bounds: mozui_style::Rect::new(ex, ey, measured.width, measured.height),
                        font_size: self.text_size(),
                        color: self.fg.with_alpha(alpha * 0.6),
                        weight: 400,
                        italic: false,
                    });
                } else {
                    let is_current = item == current;
                    paint_button(
                        index,
                        draw_list,
                        interactions,
                        Some(item),
                        None,
                        Some(&item.to_string()),
                        is_current,
                        !self.disabled && !is_current,
                    );
                }
            }
        }

        let next_enabled = !self.disabled && current < self.total_pages;
        paint_button(
            index,
            draw_list,
            interactions,
            Some((current + 1).min(self.total_pages)),
            Some(IconName::CaretRight),
            None,
            false,
            next_enabled,
        );
    }
}
