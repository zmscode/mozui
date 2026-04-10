use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use std::cell::Cell;
use std::rc::Rc;
use taffy::Overflow;
use taffy::prelude::*;

pub struct AccordionItem {
    title: String,
    icon: Option<IconName>,
    open: bool,
    /// Animated height factor (0.0 to 1.0). Pass from `cx.use_animated()`.
    height_factor: f32,
    disabled: bool,
    children: Vec<Box<dyn Element>>,
    on_toggle: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
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
        self.on_toggle = Some(Rc::new(handler));
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

/// Tracks layout IDs for a single accordion section.
struct AccordionSectionLayout {
    header_id: LayoutId,
    icon_id: Option<LayoutId>,
    title_id: LayoutId,
    chevron_id: LayoutId,
    clip_id: LayoutId,
    inner_id: LayoutId,
    child_ids: Vec<LayoutId>,
    divider_id: Option<LayoutId>,
}

pub struct Accordion {
    layout_id: LayoutId,
    section_layouts: Vec<AccordionSectionLayout>,

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
        layout_id: LayoutId::NONE,
        section_layouts: Vec::new(),

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
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Accordion",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let py = self.header_py();
        let icon_sz = self.icon_size();
        let text_sz = self.text_size();
        let mut section_nodes = Vec::new();
        self.section_layouts = Vec::new();

        for item in &mut self.items {
            let mut header_children = Vec::new();

            // Optional leading icon
            let icon_id = if item.icon.is_some() {
                let id = cx.new_leaf(Style {
                    size: Size {
                        width: length(icon_sz),
                        height: length(icon_sz),
                    },
                    ..Default::default()
                });
                header_children.push(id);
                Some(id)
            } else {
                None
            };

            // Title
            let title_style = mozui_text::TextStyle {
                font_size: text_sz,
                color: self.fg,
                ..Default::default()
            };
            let title_m = mozui_text::measure_text(&item.title, &title_style, None, cx.font_system);
            let title_id = cx.new_leaf(Style {
                size: Size {
                    width: length(title_m.width),
                    height: length(title_m.height),
                },
                flex_grow: 1.0,
                ..Default::default()
            });
            header_children.push(title_id);

            // Chevron
            let chevron_id = cx.new_leaf(Style {
                size: Size {
                    width: length(icon_sz),
                    height: length(icon_sz),
                },
                ..Default::default()
            });
            header_children.push(chevron_id);

            let header_id = cx.new_with_children(
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
            let child_ids: Vec<LayoutId> = item.children.iter_mut().map(|c| c.layout(cx)).collect();
            let inner_id = cx.new_with_children(
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
                &child_ids,
            );
            let max_height = if item.height_factor >= 0.999 {
                auto()
            } else {
                let max_h = item.content_height.get() * item.height_factor;
                length(max_h.max(0.0))
            };
            let clip_id = cx.new_with_children(
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
                &[inner_id],
            );

            // Section = header + content
            let section_id = cx.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                &[header_id, clip_id],
            );
            section_nodes.push(section_id);

            // Border divider
            let divider_id = if self.bordered {
                let id = cx.new_leaf(Style {
                    size: Size {
                        width: percent(1.0),
                        height: length(1.0),
                    },
                    ..Default::default()
                });
                section_nodes.push(id);
                Some(id)
            } else {
                None
            };

            self.section_layouts.push(AccordionSectionLayout {
                header_id,
                icon_id,
                title_id,
                chevron_id,
                clip_id,
                inner_id,
                child_ids,
                divider_id,
            });
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &section_nodes,
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        let icon_sz = self.icon_size();
        let item_count = self.items.len();

        for i in 0..item_count {
            let sl = &self.section_layouts[i];

            // Header
            let header_bounds = cx.bounds(sl.header_id);

            let alpha = if self.items[i].disabled { 0.5 } else { 1.0 };
            let hovered = !self.items[i].disabled && cx.interactions.is_hovered(header_bounds);

            if hovered {
                cx.draw_list.push(DrawCommand::Rect {
                    bounds: header_bounds,
                    background: Fill::Solid(self.hover_bg.with_alpha(alpha)),
                    corner_radii: Corners::uniform(4.0),
                    border: None,
                    shadow: None, shadows: vec![],
                });
            }

            // Optional icon
            if let Some(icon_name) = self.items[i].icon {
                let icon_bounds = cx.bounds(sl.icon_id.unwrap());
                cx.draw_list.push(DrawCommand::Icon {
                    name: icon_name,
                    weight: IconWeight::Regular,
                    bounds: icon_bounds,
                    color: self.fg.with_alpha(alpha),
                    size_px: icon_sz,
                });
            }

            // Title
            let title_bounds = cx.bounds(sl.title_id);
            cx.draw_list.push(DrawCommand::Text {
                text: self.items[i].title.clone(),
                bounds: title_bounds,
                font_size: self.text_size(),
                color: self.fg.with_alpha(alpha),
                weight: 500,
                italic: false,
            });

            // Chevron
            let chevron_bounds = cx.bounds(sl.chevron_id);
            let chevron_icon = if self.items[i].open {
                IconName::CaretUp
            } else {
                IconName::CaretDown
            };
            cx.draw_list.push(DrawCommand::Icon {
                name: chevron_icon,
                weight: IconWeight::Regular,
                bounds: chevron_bounds,
                color: self.muted_fg.with_alpha(alpha),
                size_px: icon_sz,
            });

            // Click handler on header
            if !self.items[i].disabled {
                if let Some(ref handler) = self.items[i].on_toggle {
                    cx.interactions.register_click(
                        header_bounds,
                        handler.clone(),
                    );
                }
            }

            // Content area — remember height for animation
            let inner_bounds = cx.bounds(sl.inner_id);
            self.items[i].content_height.set(inner_bounds.size.height);

            // Clip content to the outer container's visible bounds
            let clip_bounds = cx.bounds(sl.clip_id);
            cx.draw_list.push_clip(clip_bounds);

            let child_count = self.items[i].children.len();
            for ci in 0..child_count {
                let child_bounds = cx.bounds(sl.child_ids[ci]);
                self.items[i].children[ci].paint(child_bounds, cx);
            }

            cx.draw_list.pop_clip();

            // Border divider
            if let Some(div_id) = sl.divider_id {
                if i < item_count - 1 {
                    let div_bounds = cx.bounds(div_id);
                    cx.draw_list.push(DrawCommand::Rect {
                        bounds: div_bounds,
                        background: Fill::Solid(self.border_color),
                        corner_radii: Corners::uniform(0.0),
                        border: None,
                        shadow: None, shadows: vec![],
                    });
                }
            }
        }
    }
}
