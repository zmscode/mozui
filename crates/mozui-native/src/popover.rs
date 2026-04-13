use cocoa::base::{id, nil};
use cocoa::foundation::NSRect;
use mozui::{
    Bounds, NativeAnchor, NativePopover, NativePopoverBehavior as CorePopoverBehavior,
    NativePopoverEdge as CorePopoverEdge, Window, point, px, size,
};
use objc::{msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_POPOVER_IDENTIFIER: AtomicUsize = AtomicUsize::new(1);

/// Edge to which the popover arrow points.
pub enum PopoverEdge {
    Top,
    Left,
    Bottom,
    Right,
}

/// Popover behavior when clicking outside.
pub enum PopoverBehavior {
    /// Stays open; must be closed programmatically.
    ApplicationDefined,
    /// Closes when clicking outside.
    Transient,
    /// Closes when interacting with another window.
    Semitransient,
}

/// Configuration for a native popover.
pub struct PopoverConfig {
    /// Size of the popover content area.
    pub width: f64,
    pub height: f64,
    /// Preferred edge for the popover arrow.
    pub edge: PopoverEdge,
    /// Close behavior.
    pub behavior: PopoverBehavior,
}

impl Default for PopoverConfig {
    fn default() -> Self {
        Self {
            width: 300.0,
            height: 200.0,
            edge: PopoverEdge::Bottom,
            behavior: PopoverBehavior::Transient,
        }
    }
}

/// Show a popover anchored to the window's content view.
///
/// `content_view` is an `NSView` to display inside the popover.
/// Pass `nil` to create an empty popover with the configured size.
pub fn show_popover(window: &Window, content_view: id, config: PopoverConfig) -> id {
    let ns_view = get_raw_ns_view(window);
    if ns_view == nil {
        return nil;
    }

    let bounds: NSRect = unsafe { msg_send![ns_view, bounds] };
    let identifier = format!(
        "mozui-native.popover.{}",
        NEXT_POPOVER_IDENTIFIER.fetch_add(1, Ordering::Relaxed)
    );

    let Some(handle) = window.show_native_popover(
        NativePopover::new(
            NativeAnchor::ContentBounds(Bounds::new(
                point(px(bounds.origin.x as f32), px(bounds.origin.y as f32)),
                size(px(bounds.size.width as f32), px(bounds.size.height as f32)),
            )),
            size(px(config.width as f32), px(config.height as f32)),
        )
        .edge(config.edge.into())
        .behavior(config.behavior.into())
        .host_identifier(identifier.clone()),
    ) else {
        return nil;
    };

    if content_view != nil {
        let _ = window.set_native_host_content(&identifier, content_view.cast());
    }

    window
        .raw_native_popover_ptr(handle)
        .map(|popover| popover as id)
        .unwrap_or(nil)
}

/// Close a popover.
pub fn close_popover(popover: id) {
    unsafe {
        let _: () = msg_send![popover, close];
    }
}

impl From<PopoverEdge> for CorePopoverEdge {
    fn from(value: PopoverEdge) -> Self {
        match value {
            PopoverEdge::Top => CorePopoverEdge::Top,
            PopoverEdge::Left => CorePopoverEdge::Left,
            PopoverEdge::Bottom => CorePopoverEdge::Bottom,
            PopoverEdge::Right => CorePopoverEdge::Right,
        }
    }
}

impl From<PopoverBehavior> for CorePopoverBehavior {
    fn from(value: PopoverBehavior) -> Self {
        match value {
            PopoverBehavior::ApplicationDefined => CorePopoverBehavior::ApplicationDefined,
            PopoverBehavior::Transient => CorePopoverBehavior::Transient,
            PopoverBehavior::Semitransient => CorePopoverBehavior::Semitransient,
        }
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
