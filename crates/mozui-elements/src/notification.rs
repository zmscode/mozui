use crate::{Element, InteractionMap};
use mozui_icons::IconName;
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::animation::{Animated, Transition};
use mozui_style::{Color, Corners, Fill, Shadow, Theme};
use mozui_text::FontSystem;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;
use taffy::prelude::*;

// gpui-component: w_112 = 28rem = 448px
const NOTIFICATION_WIDTH: f32 = 448.0;
// gpui-component: py_3p5 = 14px
const PADDING_Y: f32 = 14.0;
// gpui-component: px_4 = 16px
const PADDING_X: f32 = 16.0;
// gpui-component: gap_3 = 12px
const GAP: f32 = 12.0;
// gpui-component: pl_6 = 24px (left padding for text when icon is present)
const ICON_TEXT_INDENT: f32 = 24.0;
// gpui-component: icon size (default small = 16px)
const ICON_SIZE: f32 = 16.0;
// gpui-component: close button at top_1 (4px) right_1 (4px), xsmall = 16px
const CLOSE_SIZE: f32 = 16.0;
const CLOSE_OFFSET: f32 = 4.0;
// gpui-component: text_sm = 14px (0.875rem)
const TEXT_SM: f32 = 14.0;
// gpui-component: default margin from window edge = 16px
const MARGIN: f32 = 16.0;
/// gpui-component: gap_3 between stacked notifications = 12px
pub const STACK_GAP: f32 = 12.0;

/// Notification type/severity, each with a default icon and color.
///
/// Matches gpui-component's `NotificationType` enum with the same icon mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationType {
    /// Plain notification with no icon or accent color.
    Default,
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationType {
    /// Default icon for each type. Matches gpui-component:
    /// Info → Info, Success → CheckCircle, Warning → Warning, Error → XCircle
    /// Default has no icon.
    pub fn icon(&self) -> Option<IconName> {
        match self {
            Self::Default => None,
            Self::Info => Some(IconName::Info),
            Self::Success => Some(IconName::CheckCircle),
            Self::Warning => Some(IconName::Warning),
            Self::Error => Some(IconName::XCircle),
        }
    }

    /// Accent color for each type from theme semantic colors.
    /// Default uses the foreground color.
    pub fn color(&self, theme: &Theme) -> Color {
        match self {
            Self::Default => theme.muted_foreground,
            Self::Info => theme.info,
            Self::Success => theme.success,
            Self::Warning => theme.warning,
            Self::Error => theme.danger,
        }
    }
}

/// A toast notification matching gpui-component's `Notification` styling.
///
/// - 448px wide, 1px border, `shadow_md`, `radius_lg`
/// - Icon absolutely positioned at top-left inside padding
/// - Close button absolutely positioned top-right, visible on hover
/// - Stacks from top-right with 12px gap
///
/// ```rust,ignore
/// notification(&theme, NotificationType::Success, "File saved")
///     .description("Your changes have been saved successfully.")
///     .on_dismiss(move |cx| { /* remove notification */ })
/// ```
/// Animation duration in ms for notification entrance/exit (gpui-component: 250ms).
/// Use this when scheduling removal after exit animation:
/// `cx.set_timeout(Duration::from_millis(NOTIFICATION_ANIM_MS), ...)`
pub const NOTIFICATION_ANIM_MS: u64 = 250;
/// Slide distance in pixels for entrance/exit animation.
const SLIDE_DISTANCE: f32 = 45.0;

pub struct Notification {
    notification_type: NotificationType,
    title: Option<String>,
    message: String,
    icon_override: Option<IconName>,
    on_dismiss: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    bg: Color,
    fg: Color,
    accent_color: Color,
    border_color: Color,
    shadow: Shadow,
    corner_radius: f32,
    /// Vertical offset from top (for stacking multiple notifications).
    top_offset: f32,
    /// Titlebar height — notifications start below this.
    titlebar_height: f32,
    /// Baked-in animation: 0.0 = hidden, 1.0 = fully visible.
    /// Automatically starts entrance animation on construction.
    anim: Animated<f32>,
    /// Whether animation is disabled (always fully visible).
    no_anim: bool,
}

/// Create a new entrance animation for a notification.
/// Store this in your notification list and pass it to the notification builder.
/// Call `.set(0.0)` on it to trigger the exit animation.
pub fn notification_anim(animation_flag: Rc<Cell<bool>>) -> Animated<f32> {
    let transition = Transition::new(Duration::from_millis(NOTIFICATION_ANIM_MS))
        .custom_bezier(0.4, 0.0, 0.2, 1.0);
    let anim = Animated::new(0.0, transition, animation_flag);
    anim.set(1.0); // start entrance animation
    anim
}

/// Create a notification toast with baked-in animation support.
///
/// Pass an `Animated<f32>` from `notification_anim()` to animate entrance/exit.
/// The notification reads the animation progress and computes opacity + slide
/// automatically — no manual wiring needed.
///
/// ```rust,ignore
/// // On spawn:
/// let anim = notification_anim(cx.animation_flag());
/// // Store (id, type, title, message, anim) in your list.
///
/// // On render:
/// notification(&theme, NotificationType::Success, "Saved!")
///     .title("File saved")
///     .anim(anim.clone())
///     .on_dismiss(move |cx| { anim.set(0.0); /* schedule removal */ });
/// ```
pub fn notification(
    theme: &Theme,
    notification_type: NotificationType,
    message: impl Into<String>,
) -> Notification {
    let accent = notification_type.color(theme);
    // Default: no animation (fully visible immediately).
    // Call .anim() to attach a persisted animation handle.
    let dummy_flag = Rc::new(Cell::new(false));
    let anim = Animated::new(1.0, Transition::new(Duration::ZERO), dummy_flag);
    Notification {
        notification_type,
        title: None,
        message: message.into(),
        icon_override: None,
        on_dismiss: None,
        bg: theme.popover,
        fg: theme.popover_foreground,
        accent_color: accent,
        border_color: theme.border,
        shadow: theme.shadow_md,
        corner_radius: theme.radius_lg,
        top_offset: 0.0,
        titlebar_height: 38.0,
        anim,
        no_anim: true,
    }
}

impl Notification {
    /// Set an optional bold title above the message.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Alias for the message field — set the description/body text.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.message = desc.into();
        self
    }

    /// Override the default icon for this notification type.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon_override = Some(icon);
        self
    }

    /// Handler called when the close button is clicked.
    pub fn on_dismiss(mut self, handler: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    /// Set the vertical offset from the top edge (for stacking).
    /// Each notification adds its measured height + STACK_GAP.
    pub fn top_offset(mut self, offset: f32) -> Self {
        self.top_offset = offset;
        self
    }

    /// Set the titlebar height (notifications start below it).
    pub fn titlebar_height(mut self, height: f32) -> Self {
        self.titlebar_height = height;
        self
    }

    /// Attach a persisted animation handle (from `notification_anim()`).
    /// The notification reads the animation progress to compute opacity and slide.
    pub fn anim(mut self, anim: Animated<f32>) -> Self {
        self.anim = anim;
        self.no_anim = false;
        self
    }

    /// Disable entrance/exit animations. The notification appears instantly.
    pub fn no_anim(mut self) -> Self {
        self.no_anim = true;
        self.anim.set_immediate(1.0);
        self
    }
}

impl Element for Notification {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        // gpui-component layout:
        // h_flex (relative) [w=448, border, bg, rounded, shadow, py=14, px=16, gap=12]
        //   div (absolute) [py=14, left=16] → icon
        //   v_flex (flex_1, overflow_hidden, pl=24 when icon)
        //     div [text_sm, font_semibold] → title (optional)
        //     div [text_sm] → message
        //   div (absolute) [top=4, right=4] → close button

        let has_icon = self.icon_override.is_some() || self.notification_type.icon().is_some();

        // Text column children
        let mut text_children = Vec::new();
        let text_max_width = NOTIFICATION_WIDTH - PADDING_X * 2.0
            - if has_icon { ICON_TEXT_INDENT } else { 0.0 }
            - CLOSE_SIZE - CLOSE_OFFSET; // leave room for close button

        // Title (optional, bold)
        if let Some(ref title) = self.title {
            let style = mozui_text::TextStyle {
                font_size: TEXT_SM,
                color: self.fg,
                weight: mozui_text::FontWeight::Bold,
                ..Default::default()
            };
            let m = mozui_text::measure_text(title, &style, Some(text_max_width), font_system);
            text_children.push(engine.new_leaf(Style {
                size: taffy::Size {
                    width: length(m.width),
                    height: length(m.height),
                },
                ..Default::default()
            }));
        }

        // Message
        let msg_style = mozui_text::TextStyle {
            font_size: TEXT_SM,
            color: self.fg,
            ..Default::default()
        };
        let msg_m =
            mozui_text::measure_text(&self.message, &msg_style, Some(text_max_width), font_system);
        text_children.push(engine.new_leaf(Style {
            size: taffy::Size {
                width: length(msg_m.width),
                height: length(msg_m.height),
            },
            ..Default::default()
        }));

        // Text column — flex_1, with pl=24 when icon present
        let text_col = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                overflow: taffy::Point {
                    x: taffy::Overflow::Hidden,
                    y: taffy::Overflow::Hidden,
                },
                padding: taffy::Rect {
                    left: if has_icon {
                        length(ICON_TEXT_INDENT)
                    } else {
                        zero()
                    },
                    right: zero(),
                    top: zero(),
                    bottom: zero(),
                },
                ..Default::default()
            },
            &text_children,
        );

        // Main content container — h_flex with padding
        // Icon and close button are absolutely positioned, so not flex children.
        // The only flex child is the text column.
        let content = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                position: Position::Relative,
                align_items: Some(AlignItems::FlexStart),
                gap: taffy::Size {
                    width: length(GAP),
                    height: zero(),
                },
                padding: taffy::Rect {
                    left: length(PADDING_X),
                    right: length(PADDING_X),
                    top: length(PADDING_Y),
                    bottom: length(PADDING_Y),
                },
                size: taffy::Size {
                    width: length(NOTIFICATION_WIDTH),
                    height: auto(),
                },
                ..Default::default()
            },
            &[text_col],
        );

        // Absolute-positioned wrapper (top-right, with titlebar offset)
        // gpui-component: margins.top = TITLE_BAR_HEIGHT + 16px, margins.right = 16px
        let top = self.titlebar_height + MARGIN + self.top_offset;
        engine.new_with_children(
            Style {
                display: Display::Flex,
                position: Position::Absolute,
                inset: taffy::Rect {
                    top: length(top),
                    right: length(MARGIN),
                    left: auto(),
                    bottom: auto(),
                },
                ..Default::default()
            },
            &[content],
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
        let progress = if self.no_anim { 1.0 } else { self.anim.get() };
        let opacity = progress;
        let dy = -SLIDE_DISTANCE * (1.0 - progress); // slides down from -45px to 0

        // Helper: offset a rect by the slide_y animation offset
        let offset_rect = |r: mozui_style::Rect| -> mozui_style::Rect {
            mozui_style::Rect::new(r.origin.x, r.origin.y + dy, r.size.width, r.size.height)
        };

        // Helper: apply opacity to a color
        let fade = |c: Color| -> Color { c.with_alpha(c.a * opacity) };

        // Don't paint at all if fully transparent
        if opacity < 0.001 {
            // Still consume layout indices
            *index += 1; // wrapper
            *index += 1; // content
            *index += 1; // text_col
            if self.title.is_some() {
                *index += 1; // title
            }
            *index += 1; // message
            return;
        }

        // Absolute wrapper
        let _wrapper = layouts[*index];
        *index += 1;

        // Content container
        let content_layout = layouts[*index];
        *index += 1;
        let content_bounds = offset_rect(mozui_style::Rect::new(
            content_layout.x,
            content_layout.y,
            content_layout.width,
            content_layout.height,
        ));

        // Draw notification background with border and shadow
        // gpui-component: shadow_none when opacity < 0.85
        let shadow = if opacity < 0.85 {
            None
        } else {
            Some(self.shadow)
        };
        draw_list.push(DrawCommand::Rect {
            bounds: content_bounds,
            background: Fill::Solid(fade(self.bg)),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(mozui_renderer::Border {
                width: 1.0,
                color: fade(self.border_color),
            }),
            shadow,
        });

        // Register hover region on entire notification for close button visibility
        interactions.register_hover_region(content_bounds);
        let notification_hovered = interactions.is_hovered(content_bounds);

        // Draw icon — absolutely positioned, vertically centered with the first line of text.
        let resolved_icon = self.icon_override.or_else(|| self.notification_type.icon());
        if let Some(icon_name) = resolved_icon {
            let first_line_height = TEXT_SM * 1.4; // approximate line-height
            let icon_x = content_bounds.origin.x + PADDING_X;
            let icon_y =
                content_bounds.origin.y + PADDING_Y + (first_line_height - ICON_SIZE) / 2.0;
            let icon_bounds = mozui_style::Rect::new(icon_x, icon_y, ICON_SIZE, ICON_SIZE);
            draw_list.push(DrawCommand::Icon {
                name: icon_name,
                weight: mozui_icons::IconWeight::Regular,
                bounds: icon_bounds,
                color: fade(self.accent_color),
                size_px: ICON_SIZE,
            });
        }

        // Text column
        let _text_col = layouts[*index];
        *index += 1;

        // Title (optional)
        if self.title.is_some() {
            let title_layout = layouts[*index];
            *index += 1;
            let title_bounds = offset_rect(mozui_style::Rect::new(
                title_layout.x,
                title_layout.y,
                title_layout.width,
                title_layout.height,
            ));
            draw_list.push(DrawCommand::Text {
                text: self.title.as_ref().unwrap().clone(),
                bounds: title_bounds,
                font_size: TEXT_SM,
                color: fade(self.fg),
                weight: 600,
                italic: false,
            });
        }

        // Message
        let msg_layout = layouts[*index];
        *index += 1;
        let msg_bounds = offset_rect(mozui_style::Rect::new(
            msg_layout.x,
            msg_layout.y,
            msg_layout.width,
            msg_layout.height,
        ));
        draw_list.push(DrawCommand::Text {
            text: self.message.clone(),
            bounds: msg_bounds,
            font_size: TEXT_SM,
            color: fade(self.fg),
            weight: 400,
            italic: false,
        });

        // Close button — absolutely positioned top-right
        let close_x =
            content_bounds.origin.x + content_bounds.size.width - CLOSE_OFFSET - CLOSE_SIZE;
        let close_y = content_bounds.origin.y + CLOSE_OFFSET;
        let close_bounds = mozui_style::Rect::new(close_x, close_y, CLOSE_SIZE, CLOSE_SIZE);

        // Only render close button when notification is hovered (gpui-component: group_hover)
        if notification_hovered {
            let close_icon_hovered = interactions.is_hovered(close_bounds);
            let close_color = if close_icon_hovered {
                fade(self.fg)
            } else {
                fade(self.fg.with_alpha(0.5))
            };
            draw_list.push(DrawCommand::Icon {
                name: IconName::X,
                weight: mozui_icons::IconWeight::Regular,
                bounds: close_bounds,
                color: close_color,
                size_px: CLOSE_SIZE,
            });

            interactions.register_hover_region(close_bounds);
        }

        // Register click handler for close button (always, even when hidden)
        if let Some(ref handler) = self.on_dismiss {
            let handler_ptr = handler.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            interactions.register_click(
                close_bounds,
                Box::new(move |cx| unsafe { (*handler_ptr)(cx) }),
            );
        }
    }
}
