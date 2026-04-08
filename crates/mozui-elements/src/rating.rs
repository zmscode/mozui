use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

pub struct Rating {
    value: f32,
    max: u32,
    disabled: bool,
    size: ComponentSize,
    color: Color,
    empty_color: Color,
    on_change: Option<Box<dyn Fn(f32, &mut dyn std::any::Any)>>,
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
    fn layout(&self, engine: &mut LayoutEngine, _font_system: &FontSystem) -> taffy::NodeId {
        let star_sz = self.star_size();
        let mut children = Vec::new();

        for _ in 0..self.max {
            children.push(engine.new_leaf(Style {
                size: Size {
                    width: length(star_sz),
                    height: length(star_sz),
                },
                ..Default::default()
            }));
        }

        engine.new_with_children(
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
            &children,
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        let _outer = layouts[*index];
        *index += 1;

        let alpha = if self.disabled { 0.5 } else { 1.0 };
        let star_sz = self.star_size();

        for i in 0..self.max {
            let star_layout = layouts[*index];
            *index += 1;

            let star_bounds = mozui_style::Rect::new(
                star_layout.x,
                star_layout.y,
                star_layout.width,
                star_layout.height,
            );

            let star_index = i as f32;
            let hovered = !self.disabled && interactions.is_hovered(star_bounds);

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

            draw_list.push(DrawCommand::Icon {
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
                    interactions.register_click(
                        star_bounds,
                        Box::new(move |cx| unsafe { (*handler_ptr)(new_value, cx) }),
                    );
                }
            }
        }
    }
}
