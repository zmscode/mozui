use crate::styled::{ComponentSize, Disableable, Sizable};
use crate::{Element, LayoutContext, PaintContext};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Point, Rect, Shadow, Theme};
use std::rc::Rc;
use taffy::prelude::*;

pub struct Slider {
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    disabled: bool,
    size: ComponentSize,
    track_color: Color,
    fill_color: Color,
    thumb_color: Color,
    shadow_sm: Shadow,
    on_change: Option<Rc<dyn Fn(f32, &mut dyn std::any::Any)>>,
    layout_id: LayoutId,
}

pub fn slider(theme: &Theme) -> Slider {
    Slider {
        min: 0.0,
        max: 100.0,
        step: 1.0,
        value: 0.0,
        disabled: false,
        size: ComponentSize::Medium,
        track_color: theme.slider_bar,
        fill_color: theme.primary,
        thumb_color: theme.slider_thumb,
        shadow_sm: theme.shadow_sm,
        on_change: None,
        layout_id: LayoutId::NONE,
    }
}

impl Slider {
    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f32, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    fn track_height(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 2.0,
            ComponentSize::Small => 3.0,
            ComponentSize::Medium => 4.0,
            ComponentSize::Large => 6.0,
            ComponentSize::Custom(_) => 4.0,
        }
    }

    fn thumb_size(&self) -> f32 {
        match self.size {
            ComponentSize::XSmall => 12.0,
            ComponentSize::Small => 14.0,
            ComponentSize::Medium => 16.0,
            ComponentSize::Large => 20.0,
            ComponentSize::Custom(px) => px as f32,
        }
    }

    fn total_height(&self) -> f32 {
        self.thumb_size()
    }

    /// Compute value from an x position within the track bounds.
    fn value_from_x(x: f32, track_x: f32, track_w: f32, min: f32, max: f32, step: f32) -> f32 {
        let ratio = ((x - track_x) / track_w).clamp(0.0, 1.0);
        let raw = min + ratio * (max - min);
        // Snap to step
        if step > 0.0 {
            (((raw - min) / step).round() * step + min).clamp(min, max)
        } else {
            raw.clamp(min, max)
        }
    }
}

impl Sizable for Slider {
    fn with_size(mut self, size: impl Into<ComponentSize>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Slider {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Element for Slider {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "Slider",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        // Single node — we handle all drawing ourselves
        self.layout_id = cx.new_leaf(Style {
            size: Size {
                width: Dimension::Auto,
                height: length(self.total_height()),
            },
            min_size: Size {
                width: length(60.0),
                height: length(self.total_height()),
            },
            flex_grow: 1.0,
            ..Default::default()
        });
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let alpha = if self.disabled { 0.5 } else { 1.0 };

        let track_h = self.track_height();
        let thumb_sz = self.thumb_size();
        let half_thumb = thumb_sz / 2.0;

        // Track is horizontally inset by half-thumb so thumb doesn't clip
        let track_x = bounds.origin.x + half_thumb;
        let track_w = bounds.size.width - thumb_sz;
        let track_y = bounds.origin.y + (bounds.size.height - track_h) / 2.0;
        let track_radius = track_h / 2.0;

        // Normalized position
        let range = self.max - self.min;
        let ratio = if range > 0.0 {
            ((self.value - self.min) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let fill_w = track_w * ratio;

        // Background track
        cx.draw_list.push(DrawCommand::Rect {
            bounds: Rect::new(track_x, track_y, track_w, track_h),
            background: Fill::Solid(self.track_color.with_alpha(alpha)),
            corner_radii: Corners::uniform(track_radius),
            border: None,
            shadow: None, shadows: vec![],
        });

        // Filled portion
        if fill_w > 0.5 {
            cx.draw_list.push(DrawCommand::Rect {
                bounds: Rect::new(track_x, track_y, fill_w, track_h),
                background: Fill::Solid(self.fill_color.with_alpha(alpha)),
                corner_radii: Corners::uniform(track_radius),
                border: None,
                shadow: None, shadows: vec![],
            });
        }

        // Thumb
        let thumb_x = track_x + fill_w - half_thumb;
        let thumb_y = bounds.origin.y + (bounds.size.height - thumb_sz) / 2.0;
        let thumb_bounds = Rect::new(thumb_x, thumb_y, thumb_sz, thumb_sz);

        let hovered = !self.disabled && cx.interactions.is_hovered(bounds);
        let active = !self.disabled && cx.interactions.is_active(bounds);

        let thumb_color = if active {
            self.thumb_color.with_alpha(alpha * 0.8)
        } else if hovered {
            self.thumb_color.with_alpha(alpha * 0.9)
        } else {
            self.thumb_color.with_alpha(alpha)
        };

        cx.draw_list.push(DrawCommand::Rect {
            bounds: thumb_bounds,
            background: Fill::Solid(thumb_color),
            corner_radii: Corners::uniform(thumb_sz / 2.0),
            border: None,
            shadow: Some(self.shadow_sm), shadows: vec![],
        });

        // Register drag handler for value changes
        if !self.disabled {
            if let Some(ref handler) = self.on_change {
                let h = handler.clone();
                let min = self.min;
                let max = self.max;
                let step = self.step;
                cx.interactions.register_drag_handler(
                    bounds,
                    Rc::new(move |pos: Point, cx: &mut dyn std::any::Any| {
                        let val = Slider::value_from_x(pos.x, track_x, track_w, min, max, step);
                        h(val, cx);
                    }),
                );
            }
        }
    }
}
