use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Rect, Theme};
use taffy::prelude::*;

pub struct Rating {
    value: f32,
    max: u32,
    disabled: bool,
    size: ComponentSize,
    color: Color,
    empty_color: Color,
    on_change: Option<Box<dyn Fn(f32, &mut dyn std::any::Any)>>,
    layout_id: LayoutId,
    star_ids: Vec<LayoutId>,
}

pub fn rating(theme: &Theme) -> Rating {
    Rating {
        value: 0.0,
        max: 5,
        disabled: false,
        size: ComponentSize::Medium,
        color: Color::hex("#f9e2af"),
        empty_color: theme.muted,
        on_change: None,
        layout_id: LayoutId::NONE,
        star_ids: Vec::new(),
    }
}

impl Rating {
    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
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

    pub fn on_change(mut self, handler: impl Fn(f32, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    fn star_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 14.0,
            ComponentSize::Small => 18.0,
            ComponentSize::Medium => 22.0,
            ComponentSize::Large => 28.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }
}

impl Sizable for Rating {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Rating {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Element for Rating {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Rating",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let star_sz = self.star_size();
        self.star_ids.clear();

        for _ in 0..self.max {
            self.star_ids.push(cx.new_leaf(Style {
                size: Size {
                    width: length(star_sz),
                    height: length(star_sz),
                },
                ..Default::default()
            }));
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: Size {
                    width: length(2.0),
                    height: zero(),
                },
                ..Default::default()
            },
            &self.star_ids,
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let star_sz = self.star_size();

        for i in 0..self.max {
            let star_bounds = cx.bounds(self.star_ids[i as usize]);

            let star_index = i as f32;
            let hovered = !self.disabled && cx.interactions.is_hovered(star_bounds);

            // Determine fill: full, half, or empty
            let (icon_weight, icon_color) = if self.value >= star_index + 1.0 {
                // Full star
                (IconWeight::Fill, self.color)
            } else if self.value > star_index && self.value < star_index + 1.0 {
                // Half star — use StarHalf icon
                (IconWeight::Fill, self.color)
            } else {
                // Empty star
                (IconWeight::Regular, self.empty_color)
            };

            let color = if hovered {
                self.color.with_alpha(alpha * 0.8)
            } else {
                icon_color.with_alpha(alpha)
            };

            // Use StarHalf for half-filled stars
            let icon_name = if self.value > star_index
                && self.value < star_index + 1.0
                && (self.value - star_index) < 0.75
            {
                IconName::StarHalf
            } else {
                IconName::Star
            };

            cx.draw_list.push(DrawCommand::Icon {
                name: icon_name,
                weight: icon_weight,
                bounds: star_bounds,
                color,
                size_px: star_sz,
            });

            // Click handler — set rating to this star
            if !self.disabled {
                if let Some(ref handler) = self.on_change {
                    let handler_ptr =
                        handler.as_ref() as *const dyn Fn(f32, &mut dyn std::any::Any);
                    let new_value = (i + 1) as f32;
                    cx.interactions.register_click(
                        star_bounds,
                        Box::new(move |cx| unsafe { (*handler_ptr)(new_value, cx) }),
                    );
                }
            }
        }
    }
}
