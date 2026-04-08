use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill};
use mozui_text::FontSystem;
use taffy::prelude::*;
use taffy::{Overflow, Point as TaffyPoint};
use mozui_events;

pub struct Div {
    // Visual style
    background: Option<Fill>,
    corner_radii: Corners,
    border_width: f32,
    border_color: Color,
    opacity: f32,

    // Taffy layout style
    taffy_style: Style,

    // Children
    children: Vec<Box<dyn Element>>,

    // Event handlers
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    on_key_down: Option<Box<dyn Fn(mozui_events::Key, mozui_events::Modifiers, &mut dyn std::any::Any)>>,
}

pub fn div() -> Div {
    Div {
        background: None,
        corner_radii: Corners::ZERO,
        border_width: 0.0,
        border_color: Color::TRANSPARENT,
        opacity: 1.0,
        taffy_style: Style::default(),
        children: Vec::new(),
        on_click: None,
        on_key_down: None,
    }
}

impl Div {
    // --- Sizing ---
    pub fn w(mut self, width: f32) -> Self {
        self.taffy_style.size.width = length(width);
        self
    }

    pub fn h(mut self, height: f32) -> Self {
        self.taffy_style.size.height = length(height);
        self
    }

    pub fn size(mut self, s: f32) -> Self {
        self.taffy_style.size = Size {
            width: length(s),
            height: length(s),
        };
        self
    }

    pub fn w_full(mut self) -> Self {
        self.taffy_style.size.width = percent(1.0);
        self
    }

    pub fn h_full(mut self) -> Self {
        self.taffy_style.size.height = percent(1.0);
        self
    }

    pub fn min_w(mut self, v: f32) -> Self {
        self.taffy_style.min_size.width = length(v);
        self
    }

    pub fn min_h(mut self, v: f32) -> Self {
        self.taffy_style.min_size.height = length(v);
        self
    }

    pub fn max_w(mut self, v: f32) -> Self {
        self.taffy_style.max_size.width = length(v);
        self
    }

    pub fn max_h(mut self, v: f32) -> Self {
        self.taffy_style.max_size.height = length(v);
        self
    }

    // --- Flex direction ---
    pub fn flex_row(mut self) -> Self {
        self.taffy_style.display = Display::Flex;
        self.taffy_style.flex_direction = FlexDirection::Row;
        self
    }

    pub fn flex_col(mut self) -> Self {
        self.taffy_style.display = Display::Flex;
        self.taffy_style.flex_direction = FlexDirection::Column;
        self
    }

    // --- Flex properties ---
    pub fn flex_grow(mut self, v: f32) -> Self {
        self.taffy_style.flex_grow = v;
        self
    }

    pub fn flex_shrink(mut self, v: f32) -> Self {
        self.taffy_style.flex_shrink = v;
        self
    }

    pub fn flex_1(mut self) -> Self {
        self.taffy_style.flex_grow = 1.0;
        self.taffy_style.flex_shrink = 1.0;
        self.taffy_style.flex_basis = percent(0.0);
        self
    }

    pub fn flex_wrap(mut self) -> Self {
        self.taffy_style.flex_wrap = FlexWrap::Wrap;
        self
    }

    // --- Gap ---
    pub fn gap(mut self, v: f32) -> Self {
        self.taffy_style.gap = Size {
            width: length(v),
            height: length(v),
        };
        self
    }

    pub fn gap_x(mut self, v: f32) -> Self {
        self.taffy_style.gap.width = length(v);
        self
    }

    pub fn gap_y(mut self, v: f32) -> Self {
        self.taffy_style.gap.height = length(v);
        self
    }

    // --- Padding ---
    pub fn p(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.padding = Rect {
            left: l,
            right: l,
            top: l,
            bottom: l,
        };
        self
    }

    pub fn px(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.padding.left = l;
        self.taffy_style.padding.right = l;
        self
    }

    pub fn py(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.padding.top = l;
        self.taffy_style.padding.bottom = l;
        self
    }

    pub fn pt(mut self, v: f32) -> Self {
        self.taffy_style.padding.top = length(v);
        self
    }

    pub fn pb(mut self, v: f32) -> Self {
        self.taffy_style.padding.bottom = length(v);
        self
    }

    pub fn pl(mut self, v: f32) -> Self {
        self.taffy_style.padding.left = length(v);
        self
    }

    pub fn pr(mut self, v: f32) -> Self {
        self.taffy_style.padding.right = length(v);
        self
    }

    // --- Margin ---
    pub fn m(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.margin = Rect {
            left: l,
            right: l,
            top: l,
            bottom: l,
        };
        self
    }

    pub fn mx(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.margin.left = l;
        self.taffy_style.margin.right = l;
        self
    }

    pub fn my(mut self, v: f32) -> Self {
        let l = length(v);
        self.taffy_style.margin.top = l;
        self.taffy_style.margin.bottom = l;
        self
    }

    pub fn mx_auto(mut self) -> Self {
        self.taffy_style.margin.left = LengthPercentageAuto::Auto;
        self.taffy_style.margin.right = LengthPercentageAuto::Auto;
        self
    }

    // --- Alignment ---
    pub fn items_start(mut self) -> Self {
        self.taffy_style.align_items = Some(AlignItems::FlexStart);
        self
    }

    pub fn items_center(mut self) -> Self {
        self.taffy_style.align_items = Some(AlignItems::Center);
        self
    }

    pub fn items_end(mut self) -> Self {
        self.taffy_style.align_items = Some(AlignItems::FlexEnd);
        self
    }

    pub fn items_stretch(mut self) -> Self {
        self.taffy_style.align_items = Some(AlignItems::Stretch);
        self
    }

    pub fn justify_start(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::FlexStart);
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::Center);
        self
    }

    pub fn justify_end(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::FlexEnd);
        self
    }

    pub fn justify_between(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::SpaceBetween);
        self
    }

    pub fn justify_around(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::SpaceAround);
        self
    }

    pub fn justify_evenly(mut self) -> Self {
        self.taffy_style.justify_content = Some(JustifyContent::SpaceEvenly);
        self
    }

    // --- Overflow ---
    pub fn overflow_hidden(mut self) -> Self {
        self.taffy_style.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Hidden,
        };
        self
    }

    // --- Visual ---
    pub fn bg(mut self, fill: impl Into<Fill>) -> Self {
        self.background = Some(fill.into());
        self
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.corner_radii = Corners::uniform(radius);
        self
    }

    pub fn rounded_t(mut self, radius: f32) -> Self {
        self.corner_radii.top_left = radius;
        self.corner_radii.top_right = radius;
        self
    }

    pub fn rounded_b(mut self, radius: f32) -> Self {
        self.corner_radii.bottom_left = radius;
        self.corner_radii.bottom_right = radius;
        self
    }

    pub fn rounded_full(mut self) -> Self {
        self.corner_radii = Corners::uniform(9999.0);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    // --- Events ---
    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn on_key_down(
        mut self,
        handler: impl Fn(mozui_events::Key, mozui_events::Modifiers, &mut dyn std::any::Any) + 'static,
    ) -> Self {
        self.on_key_down = Some(Box::new(handler));
        self
    }

    // --- Children ---
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

impl Element for Div {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let child_nodes: Vec<taffy::NodeId> = self
            .children
            .iter()
            .map(|c| c.layout(engine, font_system))
            .collect();

        engine.new_with_children(self.taffy_style.clone(), &child_nodes)
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &mozui_text::FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);

        // Paint self
        if let Some(ref bg) = self.background {
            let border = if self.border_width > 0.0 {
                Some(Border {
                    width: self.border_width,
                    color: self.border_color,
                })
            } else {
                None
            };

            draw_list.push(DrawCommand::Rect {
                bounds,
                background: bg.clone(),
                corner_radii: self.corner_radii,
                border,
            });
        }

        // Register click handler if present
        if let Some(ref handler) = self.on_click {
            let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            interactions.register_click(bounds, Box::new(move |cx| unsafe { (*handler_ptr)(cx) }));
        }

        // Register key handler if present
        if let Some(ref handler) = self.on_key_down {
            let handler_ptr = handler.as_ref()
                as *const dyn Fn(mozui_events::Key, mozui_events::Modifiers, &mut dyn std::any::Any);
            interactions.register_key_handler(Box::new(move |key, mods, cx| unsafe {
                (*handler_ptr)(key, mods, cx)
            }));
        }

        // Paint children
        for child in &self.children {
            child.paint(layouts, index, draw_list, interactions, font_system);
        }
    }
}
