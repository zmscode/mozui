use crate::traits::{EventCallback, Platform, PlatformWindow, Screen, WindowOptions};
use mozui_events::{CursorStyle, Modifiers, MouseButton, PlatformEvent, ScrollDelta, WindowId};
use mozui_style::{Point, Rect};

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSCursor, NSEventModifierFlags, NSEventType,
    NSPasteboard, NSPasteboardTypeString, NSScreen,
};
use objc2_foundation::NSString;

use std::collections::HashMap;

use super::window::MacWindow;

pub struct MacPlatform {
    next_window_id: u64,
    /// Maps NSWindow pointer address to WindowId for event routing.
    window_map: HashMap<usize, WindowId>,
}

impl MacPlatform {
    pub fn new() -> Self {
        Self {
            next_window_id: 0,
            window_map: HashMap::new(),
        }
    }

    fn allocate_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        id
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

        // Take ownership of window map for event routing
        let window_map = std::mem::take(&mut self.window_map);

        // Initial draw — send to all windows
        let window_ids: Vec<WindowId> = window_map.values().copied().collect();
        for &wid in &window_ids {
            callback(wid, PlatformEvent::RedrawRequested);
        }

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
                        // Convert NSEvent to PlatformEvent and route to correct window
                        if let Some(platform_event) = ns_event_to_platform_event(&event) {
                            let wid = resolve_window_id(&event, &window_map, mtm);
                            callback(wid, platform_event);
                        }
                        // Let NSApp handle the event (for window management etc.)
                        app.sendEvent(&event);
                    }
                    None => break,
                }
            }

            // Request a redraw each frame — send to all windows
            for &wid in &window_ids {
                callback(wid, PlatformEvent::RedrawRequested);
            }

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
                    let wid = resolve_window_id(&event, &window_map, mtm);
                    callback(wid, platform_event);
                }
                app.sendEvent(&event);
            }
        }
    }

    fn open_window(&mut self, options: WindowOptions) -> (WindowId, Box<dyn PlatformWindow>) {
        let mtm = MainThreadMarker::new().expect("Must be called from the main thread");
        let window = MacWindow::new(mtm, options);
        let id = self.allocate_window_id();
        // Register the NSWindow pointer for event routing
        let ns_window_ptr = window.ns_window_ptr();
        self.window_map.insert(ns_window_ptr, id);
        (id, Box::new(window))
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

/// Resolve which window an NSEvent belongs to, returning its WindowId.
/// Falls back to MAIN if the event has no associated window.
fn resolve_window_id(
    event: &objc2_app_kit::NSEvent,
    window_map: &HashMap<usize, WindowId>,
    mtm: MainThreadMarker,
) -> WindowId {
    if let Some(ns_window) = event.window(mtm) {
        let ptr = Retained::as_ptr(&ns_window) as usize;
        if let Some(&id) = window_map.get(&ptr) {
            return id;
        }
    }
    WindowId::MAIN
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
        NSEventType::KeyDown => {
            let key = ns_key_to_key(event);
            let is_repeat = event.isARepeat();
            Some(PlatformEvent::KeyDown {
                key,
                modifiers,
                is_repeat,
            })
        }
        NSEventType::KeyUp => {
            let key = ns_key_to_key(event);
            Some(PlatformEvent::KeyUp { key, modifiers })
        }
        _ => None,
    }
}

fn ns_key_to_key(event: &objc2_app_kit::NSEvent) -> mozui_events::Key {
    use mozui_events::Key;

    let keycode = event.keyCode();
    match keycode {
        36 => Key::Enter,
        53 => Key::Escape,
        48 => Key::Tab,
        51 => Key::Backspace,
        117 => Key::Delete,
        49 => Key::Space,
        126 => Key::ArrowUp,
        125 => Key::ArrowDown,
        123 => Key::ArrowLeft,
        124 => Key::ArrowRight,
        115 => Key::Home,
        119 => Key::End,
        116 => Key::PageUp,
        121 => Key::PageDown,
        122 => Key::F1,
        120 => Key::F2,
        99 => Key::F3,
        118 => Key::F4,
        96 => Key::F5,
        97 => Key::F6,
        98 => Key::F7,
        100 => Key::F8,
        101 => Key::F9,
        109 => Key::F10,
        103 => Key::F11,
        111 => Key::F12,
        _ => {
            // Try to get character from the event
            if let Some(chars) = event.characters() {
                if let Some(ch) = chars.to_string().chars().next() {
                    if !ch.is_control() {
                        return Key::Character(ch);
                    }
                }
            }
            Key::Unknown
        }
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
