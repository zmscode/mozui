use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::{Border, DrawCommand};
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use std::rc::Rc;
use taffy::prelude::*;

pub struct Pagination {
    layout_id: LayoutId,
    /// Layout IDs for: prev_btn, [page items...], next_btn
    button_ids: Vec<LayoutId>,

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
    on_click: Option<Rc<dyn Fn(usize, &mut dyn std::any::Any)>>,
}

pub fn pagination(theme: &Theme) -> Pagination {
    Pagination {
        layout_id: LayoutId::NONE,
        button_ids: Vec::new(),
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
        self.on_click = Some(Rc::new(handler));
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
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Pagination",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let btn_sz = self.button_size();
        self.button_ids.clear();
        let mut children = Vec::new();

        // Prev button
        let prev_id = cx.new_leaf(Style {
            size: Size {
                width: length(btn_sz),
                height: length(btn_sz),
            },
            ..Default::default()
        });
        self.button_ids.push(prev_id);
        children.push(prev_id);

        if self.compact {
            // "Page X of Y" text
            let label = format!("Page {} of {}", self.current_page, self.total_pages);
            let text_style = mozui_text::TextStyle {
                font_size: self.text_size(),
                color: self.fg,
                ..Default::default()
            };
            let measured = mozui_text::measure_text(&label, &text_style, None, cx.font_system);
            let label_id = cx.new_leaf(Style {
                size: Size {
                    width: length(measured.width),
                    height: length(measured.height),
                },
                ..Default::default()
            });
            self.button_ids.push(label_id);
            children.push(label_id);
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
                    let measured = mozui_text::measure_text("...", &text_style, None, cx.font_system);
                    let ell_id = cx.new_leaf(Style {
                        size: Size {
                            width: length(measured.width.max(btn_sz * 0.5)),
                            height: length(btn_sz),
                        },
                        ..Default::default()
                    });
                    self.button_ids.push(ell_id);
                    children.push(ell_id);
                } else {
                    let page_id = cx.new_leaf(Style {
                        size: Size {
                            width: length(btn_sz),
                            height: length(btn_sz),
                        },
                        ..Default::default()
                    });
                    self.button_ids.push(page_id);
                    children.push(page_id);
                }
            }
        }

        // Next button
        let next_id = cx.new_leaf(Style {
            size: Size {
                width: length(btn_sz),
                height: length(btn_sz),
            },
            ..Default::default()
        });
        self.button_ids.push(next_id);
        children.push(next_id);

        self.layout_id = cx.new_with_children(
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
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let radius = 6.0;
        let current = self.current_page;
        let mut id_idx = 0;

        // Helper closure-like approach: paint a page button
        let paint_button = |id_idx: &mut usize,
                            cx: &mut PaintContext,
                            page: Option<usize>,
                            icon: Option<IconName>,
                            label: Option<&str>,
                            is_active: bool,
                            enabled: bool,
                            button_ids: &[LayoutId],
                            on_click: &Option<Rc<dyn Fn(usize, &mut dyn std::any::Any)>>,
                            fg: Color,
                            active_bg: Color,
                            active_fg: Color,
                            hover_bg: Color,
                            border_color: Color,
                            icon_size: f32,
                            text_size: f32| {
            let bounds = cx.bounds(button_ids[*id_idx]);
            *id_idx += 1;

            let hovered = enabled && cx.interactions.is_hovered(bounds);
            let bg = if is_active {
                active_bg.with_alpha(alpha)
            } else if hovered {
                hover_bg.with_alpha(alpha)
            } else {
                Color::TRANSPARENT
            };
            let fg_color = if is_active {
                active_fg.with_alpha(alpha)
            } else {
                fg.with_alpha(alpha)
            };

            // Border
            if bg.a > 0.0 {
                cx.draw_list.push(DrawCommand::Rect {
                    bounds,
                    background: Fill::Solid(bg),
                    corner_radii: Corners::uniform(radius),
                    border: None,
                    shadow: None, shadows: vec![],
                });
            } else {
                cx.draw_list.push(DrawCommand::Rect {
                    bounds,
                    background: Fill::Solid(Color::TRANSPARENT),
                    corner_radii: Corners::uniform(radius),
                    border: Some(Border {
                        width: 1.0,
                        color: border_color.with_alpha(alpha * 0.5),
                    }),
                    shadow: None, shadows: vec![],
                });
            }

            if let Some(icon_name) = icon {
                let ix = bounds.origin.x + (bounds.size.width - icon_size) / 2.0;
                let iy = bounds.origin.y + (bounds.size.height - icon_size) / 2.0;
                cx.draw_list.push(DrawCommand::Icon {
                    name: icon_name,
                    weight: IconWeight::Regular,
                    bounds: Rect::new(ix, iy, icon_size, icon_size),
                    color: fg_color,
                    size_px: icon_size,
                });
            }

            if let Some(text) = label {
                let ts = mozui_text::TextStyle {
                    font_size: text_size,
                    color: fg_color,
                    ..Default::default()
                };
                let measured = mozui_text::measure_text(text, &ts, None, cx.font_system);
                let text_x = bounds.origin.x + (bounds.size.width - measured.width) / 2.0;
                let text_y = bounds.origin.y + (bounds.size.height - measured.height) / 2.0;
                cx.draw_list.push(DrawCommand::Text {
                    text: text.to_string(),
                    bounds: Rect::new(text_x, text_y, measured.width, measured.height),
                    font_size: text_size,
                    color: fg_color,
                    weight: if is_active { 600 } else { 400 },
                    italic: false,
                });
            }

            // Click handler
            if enabled {
                if let (Some(page), Some(handler)) = (page, on_click) {
                    let h = handler.clone();
                    cx.interactions.register_click(
                        bounds,
                        Rc::new(move |cx: &mut dyn std::any::Any| { h(page, cx) }),
                    );
                }
            }
        };

        let prev_enabled = !self.disabled && current > 1;
        let fg = self.fg;
        let active_bg = self.active_bg;
        let active_fg = self.active_fg;
        let hover_bg = self.hover_bg;
        let border_color = self.border_color;
        let icon_size = self.icon_size();
        let text_size = self.text_size();

        paint_button(
            &mut id_idx,
            cx,
            Some(current.saturating_sub(1).max(1)),
            Some(IconName::CaretLeft),
            None,
            false,
            prev_enabled,
            &self.button_ids,
            &self.on_click,
            fg, active_bg, active_fg, hover_bg, border_color, icon_size, text_size,
        );

        if self.compact {
            // Compact label
            let bounds = cx.bounds(self.button_ids[id_idx]);
            id_idx += 1;
            let label = format!("Page {} of {}", current, self.total_pages);
            cx.draw_list.push(DrawCommand::Text {
                text: label,
                bounds,
                font_size: text_size,
                color: fg.with_alpha(alpha),
                weight: 500,
                italic: false,
            });
        } else {
            for item in self.page_items() {
                if item == 0 {
                    // Ellipsis
                    let bounds = cx.bounds(self.button_ids[id_idx]);
                    id_idx += 1;
                    let ellipsis_style = mozui_text::TextStyle {
                        font_size: text_size,
                        color: fg.with_alpha(alpha * 0.6),
                        ..Default::default()
                    };
                    let measured =
                        mozui_text::measure_text("...", &ellipsis_style, None, cx.font_system);
                    let ex = bounds.origin.x + (bounds.size.width - measured.width) / 2.0;
                    let ey = bounds.origin.y + (bounds.size.height - measured.height) / 2.0;
                    cx.draw_list.push(DrawCommand::Text {
                        text: "...".to_string(),
                        bounds: Rect::new(ex, ey, measured.width, measured.height),
                        font_size: text_size,
                        color: fg.with_alpha(alpha * 0.6),
                        weight: 400,
                        italic: false,
                    });
                } else {
                    let is_current = item == current;
                    paint_button(
                        &mut id_idx,
                        cx,
                        Some(item),
                        None,
                        Some(&item.to_string()),
                        is_current,
                        !self.disabled && !is_current,
                        &self.button_ids,
                        &self.on_click,
                        fg, active_bg, active_fg, hover_bg, border_color, icon_size, text_size,
                    );
                }
            }
        }

        let next_enabled = !self.disabled && current < self.total_pages;
        paint_button(
            &mut id_idx,
            cx,
            Some((current + 1).min(self.total_pages)),
            Some(IconName::CaretRight),
            None,
            false,
            next_enabled,
            &self.button_ids,
            &self.on_click,
            fg, active_bg, active_fg, hover_bg, border_color, icon_size, text_size,
        );
    }
}
