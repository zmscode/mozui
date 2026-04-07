use crate::traits::{EventCallback, Platform, PlatformWindow, Screen, WindowOptions};
use mozui_events::{CursorStyle, MouseButton, Modifiers, PlatformEvent, ScrollDelta};
use mozui_style::{Point, Rect};

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSCursor, NSEventModifierFlags, NSEventType,
    NSPasteboard, NSPasteboardTypeString, NSScreen,
};
use objc2_foundation::NSString;

use super::window::MacWindow;

pub struct MacPlatform {
    _marker: std::marker::PhantomData<*const ()>,
}

impl MacPlatform {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl Platform for MacPlatform {
    fn run(&mut self, mut callback: EventCallback) -> ! {
        let mtm = MainThreadMarker::new().expect("Must be called from the main thread");

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);

        // Manual event loop: pump events and call our callback
        let distant_past = objc2_foundation::NSDate::distantPast();

        // Initial draw
        callback(PlatformEvent::RedrawRequested);

        loop {
            // Process all pending events
            loop {
                let event = app.nextEventMatchingMask_untilDate_inMode_dequeue(
                    objc2_app_kit::NSEventMask::Any,
                    Some(&distant_past),
                    unsafe { objc2_foundation::NSDefaultRunLoopMode },
                    true,
                );

                match event {
                    Some(event) => {
                        // Convert NSEvent to PlatformEvent
                        if let Some(platform_event) = ns_event_to_platform_event(&event) {
                            callback(platform_event);
                        }
                        // Let NSApp handle the event (for window management etc.)
                        app.sendEvent(&event);
                    }
                    None => break,
                }
            }

            // Request a redraw each frame
            callback(PlatformEvent::RedrawRequested);

            // Wait for the next event (blocks until one arrives, with a small timeout for animation)
            let timeout = objc2_foundation::NSDate::dateWithTimeIntervalSinceNow(1.0 / 60.0);
            let event = app.nextEventMatchingMask_untilDate_inMode_dequeue(
                objc2_app_kit::NSEventMask::Any,
                Some(&timeout),
                unsafe { objc2_foundation::NSDefaultRunLoopMode },
                true,
            );

            if let Some(event) = event {
                if let Some(platform_event) = ns_event_to_platform_event(&event) {
                    callback(platform_event);
                }
                app.sendEvent(&event);
            }
        }
    }

    fn open_window(&mut self, options: WindowOptions) -> Box<dyn PlatformWindow> {
        let mtm = MainThreadMarker::new().expect("Must be called from the main thread");
        let window = MacWindow::new(mtm, options);
        Box::new(window)
    }

    fn screens(&self) -> Vec<Screen> {
        let mtm = MainThreadMarker::new().expect("Must be called from the main thread");
        let screens = NSScreen::screens(mtm);
        screens
            .iter()
            .map(|screen| {
                let frame = screen.frame();
                let visible = screen.visibleFrame();
                let scale = screen.backingScaleFactor() as f32;
                Screen {
                    bounds: ns_rect_to_rect(frame),
                    work_area: ns_rect_to_rect(visible),
                    scale_factor: scale,
                }
            })
            .collect()
    }

    fn set_cursor(&self, cursor: CursorStyle) {
        let ns_cursor = match cursor {
            CursorStyle::Arrow => NSCursor::arrowCursor(),
            CursorStyle::Hand => NSCursor::pointingHandCursor(),
            CursorStyle::Text => NSCursor::IBeamCursor(),
            CursorStyle::Crosshair => NSCursor::crosshairCursor(),
            CursorStyle::NotAllowed => NSCursor::operationNotAllowedCursor(),
            #[allow(deprecated)]
            CursorStyle::ResizeNS => NSCursor::resizeUpDownCursor(),
            #[allow(deprecated)]
            CursorStyle::ResizeEW => NSCursor::resizeLeftRightCursor(),
            CursorStyle::ResizeNESW | CursorStyle::ResizeNWSE => NSCursor::crosshairCursor(),
        };
        ns_cursor.set();
    }

    fn clipboard_read(&self) -> Option<String> {
        let pasteboard = NSPasteboard::generalPasteboard();
        let nstype = unsafe { NSPasteboardTypeString };
        pasteboard.stringForType(nstype).map(|s| s.to_string())
    }

    fn clipboard_write(&self, text: &str) {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        let ns_string = NSString::from_str(text);
        let nstype = unsafe { NSPasteboardTypeString };
        let _ = pasteboard.setString_forType(&ns_string, nstype);
    }
}

/// Convert an NSEvent into a PlatformEvent, if applicable.
fn ns_event_to_platform_event(event: &objc2_app_kit::NSEvent) -> Option<PlatformEvent> {
    let event_type = event.r#type();
    let modifiers = ns_flags_to_modifiers(event.modifierFlags());

    match event_type {
        NSEventType::LeftMouseDown => {
            let pos = mouse_position_in_window(event);
            Some(PlatformEvent::MouseDown {
                button: MouseButton::Left,
                position: pos,
                modifiers,
            })
        }
        NSEventType::LeftMouseUp => {
            let pos = mouse_position_in_window(event);
            Some(PlatformEvent::MouseUp {
                button: MouseButton::Left,
                position: pos,
                modifiers,
            })
        }
        NSEventType::RightMouseDown => {
            let pos = mouse_position_in_window(event);
            Some(PlatformEvent::MouseDown {
                button: MouseButton::Right,
                position: pos,
                modifiers,
            })
        }
        NSEventType::RightMouseUp => {
            let pos = mouse_position_in_window(event);
            Some(PlatformEvent::MouseUp {
                button: MouseButton::Right,
                position: pos,
                modifiers,
            })
        }
        NSEventType::MouseMoved
        | NSEventType::LeftMouseDragged
        | NSEventType::RightMouseDragged => {
            let pos = mouse_position_in_window(event);
            Some(PlatformEvent::MouseMove {
                position: pos,
                modifiers,
            })
        }
        NSEventType::ScrollWheel => {
            let pos = mouse_position_in_window(event);
            let dx = event.scrollingDeltaX() as f32;
            let dy = event.scrollingDeltaY() as f32;
            let has_precise = event.hasPreciseScrollingDeltas();
            let delta = if has_precise {
                ScrollDelta::Pixels(dx, dy)
            } else {
                ScrollDelta::Lines(dx, dy)
            };
            Some(PlatformEvent::ScrollWheel {
                delta,
                position: pos,
                modifiers,
            })
        }
        _ => None,
    }
}

/// Get mouse position in window coordinates (flipped to top-left origin).
fn mouse_position_in_window(event: &objc2_app_kit::NSEvent) -> Point {
    let mtm = MainThreadMarker::new().expect("Must be on main thread");
    let loc = event.locationInWindow();
    // NSEvent locationInWindow origin is bottom-left, we need top-left
    if let Some(window) = event.window(mtm) {
        let frame = window.contentRectForFrameRect(window.frame());
        let height = frame.size.height as f32;
        Point::new(loc.x as f32, height - loc.y as f32)
    } else {
        Point::new(loc.x as f32, loc.y as f32)
    }
}

fn ns_rect_to_rect(r: objc2_foundation::NSRect) -> Rect {
    Rect::new(
        r.origin.x as f32,
        r.origin.y as f32,
        r.size.width as f32,
        r.size.height as f32,
    )
}

fn ns_flags_to_modifiers(flags: NSEventModifierFlags) -> Modifiers {
    Modifiers {
        shift: flags.contains(NSEventModifierFlags::Shift),
        ctrl: flags.contains(NSEventModifierFlags::Control),
        alt: flags.contains(NSEventModifierFlags::Option),
        meta: flags.contains(NSEventModifierFlags::Command),
    }
}
