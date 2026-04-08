use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use std::cell::Cell;
use taffy::prelude::*;
use taffy::Overflow;

pub struct AccordionItem {
    title: String,
    icon: Option<IconName>,
    open: bool,
    /// Animated height factor (0.0 to 1.0). Pass from `cx.use_animated()`.
    height_factor: f32,
    disabled: bool,
    children: Vec<Box<dyn Element>>,
    on_toggle: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    /// Remembers content height for animation calculations.
    content_height: Cell<f32>,
}

pub fn accordion_item(title: impl Into<String>) -> AccordionItem {
    AccordionItem {
        title: title.into(),
        icon: None,
        open: false,
        height_factor: 0.0,
        disabled: false,
        children: Vec::new(),
        on_toggle: None,
        content_height: Cell::new(0.0),
    }
}

impl AccordionItem {
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The animated height factor (0.0 collapsed, 1.0 expanded).
    pub fn height_factor(mut self, factor: f32) -> Self {
        self.height_factor = factor;
        self
    }

    pub fn on_toggle(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }

    pub fn child(mut self, element: impl Element + 'static) -> Self {
        self.children.push(Box::new(element));
        self
    }
}

impl Disableable for AccordionItem {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct Accordion {
    items: Vec<AccordionItem>,
    bordered: bool,
    size: ComponentSize,
    fg: Color,
    muted_fg: Color,
    border_color: Color,
    hover_bg: Color,
}

pub fn accordion(theme: &Theme) -> Accordion {
    Accordion {
        items: Vec::new(),
        bordered: true,
        size: ComponentSize::Medium,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        border_color: theme.border,
        hover_bg: theme.secondary,
    }
}

impl Accordion {
    pub fn child(mut self, item: AccordionItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn children(mut self, items: impl IntoIterator<Item = AccordionItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    fn text_size(&self) -> f32 {
        self.size.input_text_size()
    }

    fn header_py(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 6.0,
            ComponentSize::Small => 8.0,
            ComponentSize::Medium => 12.0,
            ComponentSize::Large => 16.0,
            ComponentSize::Custom(_) => 12.0,
        }
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
}

impl Sizable for Accordion {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Element for Accordion {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let py = self.header_py();
        let icon_sz = self.icon_size();
        let text_sz = self.text_size();
        let mut section_nodes = Vec::new();

        for item in &self.items {
            let mut header_children = Vec::new();

            // Optional leading icon
            if item.icon.is_some() {
                header_children.push(engine.new_leaf(Style {
                    size: Size {
                        width: length(icon_sz),
                        height: length(icon_sz),
                    },
                    ..Default::default()
                }));
            }

            // Title
            let title_style = mozui_text::TextStyle {
                font_size: text_sz,
                color: self.fg,
                ..Default::default()
            };
            let title_m = mozui_text::measure_text(&item.title, &title_style, None, font_system);
            header_children.push(engine.new_leaf(Style {
                size: Size {
                    width: length(title_m.width),
                    height: length(title_m.height),
                },
                flex_grow: 1.0,
                ..Default::default()
            }));

            // Chevron
            header_children.push(engine.new_leaf(Style {
                size: Size {
                    width: length(icon_sz),
                    height: length(icon_sz),
                },
                ..Default::default()
            }));

            let header_node = engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    padding: taffy::Rect {
                        left: length(12.0),
                        right: length(12.0),
                        top: length(py),
                        bottom: length(py),
                    },
                    gap: Size {
                        width: length(8.0),
                        height: zero(),
                    },
                    ..Default::default()
                },
                &header_children,
            );

            // Content area (collapsible)
            // Always lay out content children for stable node count and
            // content height measurement, even when collapsed.
            let content_children: Vec<taffy::NodeId> = item
                .children
                .iter()
                .map(|c| c.layout(engine, font_system))
                .collect();
            let inner = engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    padding: taffy::Rect {
                        left: length(12.0),
                        right: length(12.0),
                        top: zero(),
                        bottom: length(12.0),
                    },
                    gap: Size {
                        width: zero(),
                        height: length(8.0),
                    },
                    ..Default::default()
                },
                &content_children,
            );
            let max_height = if item.height_factor >= 0.999 {
                auto()
            } else {
                let max_h = item.content_height.get() * item.height_factor;
                length(max_h.max(0.0))
            };
            let content_node = engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    overflow: taffy::Point {
                        x: Overflow::Hidden,
                        y: Overflow::Hidden,
                    },
                    max_size: Size {
                        width: auto(),
                        height: max_height,
                    },
                    ..Default::default()
                },
                &[inner],
            );

            // Section = header + content
            let section = engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                &[header_node, content_node],
            );
            section_nodes.push(section);

            // Border divider
            if self.bordered {
                section_nodes.push(engine.new_leaf(Style {
                    size: Size {
                        width: percent(1.0),
                        height: length(1.0),
                    },
                    ..Default::default()
                }));
            }
        }

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &section_nodes,
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

        let icon_sz = self.icon_size();

        for (i, item) in self.items.iter().enumerate() {
            // Section container
            let _section = layouts[*index];
            *index += 1;

            // Header
            let header_layout = layouts[*index];
            *index += 1;
            let header_bounds = mozui_style::Rect::new(
                header_layout.x,
                header_layout.y,
                header_layout.width,
                header_layout.height,
            );

            let alpha = if item.disabled { 0.5 } else { 1.0 };
            let hovered = !item.disabled && interactions.is_hovered(header_bounds);

            if hovered {
                draw_list.push(DrawCommand::Rect {
                    bounds: header_bounds,
                    background: Fill::Solid(self.hover_bg.with_alpha(alpha)),
                    corner_radii: Corners::uniform(4.0),
                    border: None,
                });
            }

            // Optional icon
            if let Some(icon_name) = item.icon {
                let icon_layout = layouts[*index];
                *index += 1;
                draw_list.push(DrawCommand::Icon {
                    name: icon_name,
                    weight: IconWeight::Regular,
                    bounds: mozui_style::Rect::new(
                        icon_layout.x,
                        icon_layout.y,
                        icon_layout.width,
                        icon_layout.height,
                    ),
                    color: self.fg.with_alpha(alpha),
                    size_px: icon_sz,
                });
            }

            // Title
            let title_layout = layouts[*index];
            *index += 1;
            draw_list.push(DrawCommand::Text {
                text: item.title.clone(),
                bounds: mozui_style::Rect::new(
                    title_layout.x,
                    title_layout.y,
                    title_layout.width,
                    title_layout.height,
                ),
                font_size: self.text_size(),
                color: self.fg.with_alpha(alpha),
                weight: 500,
                italic: false,
            });

            // Chevron
            let chevron_layout = layouts[*index];
            *index += 1;
            let chevron_icon = if item.open {
                IconName::CaretUp
            } else {
                IconName::CaretDown
            };
            draw_list.push(DrawCommand::Icon {
                name: chevron_icon,
                weight: IconWeight::Regular,
                bounds: mozui_style::Rect::new(
                    chevron_layout.x,
                    chevron_layout.y,
                    chevron_layout.width,
                    chevron_layout.height,
                ),
                color: self.muted_fg.with_alpha(alpha),
                size_px: icon_sz,
            });

            // Click handler on header
            if !item.disabled {
                if let Some(ref handler) = item.on_toggle {
                    let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                    interactions.register_click(
                        header_bounds,
                        Box::new(move |cx| unsafe { (*handler_ptr)(cx) }),
                    );
                }
            }

            // Content area — always traverse nodes (layout is always built)
            // Outer clipping container
            let clip_layout = layouts[*index];
            *index += 1;
            // Inner padded container — remember height for animation
            let inner = layouts[*index];
            *index += 1;
            item.content_height.set(inner.height);

            // Clip content to the outer container's visible bounds
            let clip_rect = mozui_style::Rect::new(
                clip_layout.x,
                clip_layout.y,
                clip_layout.width,
                clip_layout.height,
            );
            draw_list.push_clip(clip_rect);

            for child in &item.children {
                child.paint(layouts, index, draw_list, interactions, font_system);
            }

            draw_list.pop_clip();

            // Border divider
            if self.bordered {
                let div_layout = layouts[*index];
                *index += 1;
                if i < self.items.len() - 1 {
                    draw_list.push(DrawCommand::Rect {
                        bounds: mozui_style::Rect::new(
                            div_layout.x,
                            div_layout.y,
                            div_layout.width,
                            div_layout.height,
                        ),
                        background: Fill::Solid(self.border_color),
                        corner_radii: Corners::uniform(0.0),
                        border: None,
                    });
                }
            }
        }
    }
}
