use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use std::rc::Rc;
use taffy::prelude::*;

/// A button that copies text to the clipboard on click.
///
/// Shows a copy icon by default, and switches to a check icon for
/// a configurable duration after copying to provide visual feedback.
///
/// ```rust,ignore
/// clipboard_button("some text to copy", &theme)
///     .copied(is_copied)
///     .on_click(move |cx| {
///         let cx = cx.downcast_mut::<Context>().unwrap();
///         cx.clipboard_write("some text to copy");
///         cx.set(set_copied, true);
///         cx.set_timeout(Duration::from_secs(2), move |cx| {
///             let cx = cx.downcast_mut::<Context>().unwrap();
///             cx.set(set_copied, false);
///         });
///     })
/// ```
pub struct ClipboardButton {
    /// Whether the "copied" state is active (shows check icon).
    copied: bool,
    icon_size: f32,
    color: Color,
    copied_color: Color,
    hover_bg: Color,
    corner_radius: f32,
    padding: f32,
    on_click: Option<Rc<dyn Fn(&mut dyn std::any::Any)>>,
    layout_id: LayoutId,
}

pub fn clipboard_button(theme: &Theme) -> ClipboardButton {
    ClipboardButton {
        copied: false,
        icon_size: 16.0,
        color: theme.muted_foreground,
        copied_color: theme.success,
        hover_bg: theme.secondary,
        corner_radius: theme.radius_sm,
        padding: 6.0,
        on_click: None,
        layout_id: LayoutId::NONE,
    }
}

impl ClipboardButton {
    /// Set whether the button is in the "copied" state (shows check icon).
    pub fn copied(mut self, copied: bool) -> Self {
        self.copied = copied;
        self
    }

    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn copied_color(mut self, color: Color) -> Self {
        self.copied_color = color;
        self
    }

    /// Set the click handler. The handler should write to the clipboard
    /// and toggle the `copied` state with a timeout to reset it.
    pub fn on_click(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Element for ClipboardButton {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "ClipboardButton",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let total = self.icon_size + self.padding * 2.0;
        self.layout_id = cx.new_leaf(Style {
            size: Size {
                width: length(total),
                height: length(total),
            },
            ..Default::default()
        });
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let hovered = cx.interactions.is_hovered(bounds);
        let active = cx.interactions.is_active(bounds);

        // Hover background
        if hovered || active {
            let bg = if active {
                self.hover_bg.with_alpha(self.hover_bg.a * 1.3)
            } else {
                self.hover_bg
            };
            cx.draw_list.push(DrawCommand::Rect {
                bounds,
                background: Fill::Solid(bg),
                corner_radii: Corners::uniform(self.corner_radius),
                border: None,
                shadow: None,
            });
        }

        // Icon
        let icon_name = if self.copied {
            IconName::Check
        } else {
            IconName::Copy
        };
        let icon_color = if self.copied {
            self.copied_color
        } else {
            self.color
        };
        let icon_bounds = Rect::new(
            bounds.origin.x + self.padding,
            bounds.origin.y + self.padding,
            self.icon_size,
            self.icon_size,
        );
        cx.draw_list.push(DrawCommand::Icon {
            name: icon_name,
            weight: IconWeight::Regular,
            bounds: icon_bounds,
            color: icon_color,
            size_px: self.icon_size,
        });

        // Register click
        if let Some(ref handler) = self.on_click {
            cx.interactions
                .register_click(bounds, handler.clone());
        }

        // Register hover for visual feedback
        cx.interactions.register_hover_region(bounds);
    }
}
