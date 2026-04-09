use crate::traits::{
    EventCallback, FileDialogOptions, Platform, PlatformWindow, Screen, WindowOptions,
};
use mozui_events::{
    CursorStyle, Key, Modifiers, MouseButton, PlatformEvent, ScrollDelta, WindowId,
};
use mozui_style::{Point, Rect};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use super::window::WebWindow;

pub struct WebPlatform {
    next_window_id: u64,
}

impl WebPlatform {
    pub fn new() -> Self {
        Self { next_window_id: 0 }
    }

    fn get_or_create_canvas(title: &str) -> HtmlCanvasElement {
        let window = web_sys::window().expect("no global window");
        let document = window.document().expect("no document");

        // Try to find an existing canvas with id "mozui-canvas"
        if let Some(el) = document.get_element_by_id("mozui-canvas") {
            if let Ok(canvas) = el.dyn_into::<HtmlCanvasElement>() {
                return canvas;
            }
        }

        // Create a new canvas
        let canvas = document
            .create_element("canvas")
            .expect("failed to create canvas")
            .dyn_into::<HtmlCanvasElement>()
            .expect("not a canvas element");
        canvas.set_id("mozui-canvas");

        // Fill the viewport
        let style = canvas.style();
        let _ = style.set_property("width", "100vw");
        let _ = style.set_property("height", "100vh");
        let _ = style.set_property("display", "block");
        let _ = style.set_property("position", "fixed");
        let _ = style.set_property("top", "0");
        let _ = style.set_property("left", "0");

        document
            .body()
            .expect("no body")
            .append_child(&canvas)
            .expect("failed to append canvas");

        document.set_title(title);

        canvas
    }
}

impl WebPlatform {
    /// Set up the requestAnimationFrame loop and DOM event listeners.
    /// Returns normally — the RAF loop keeps the app alive via JavaScript.
    pub fn start_event_loop(&self, callback: EventCallback) {
        let window = web_sys::window().expect("no global window");
        let canvas = Self::get_or_create_canvas("mozui");

        // Set up the physical canvas size
        let dpr = window.device_pixel_ratio();
        let width = canvas.client_width() as f64;
        let height = canvas.client_height() as f64;
        canvas.set_width((width * dpr) as u32);
        canvas.set_height((height * dpr) as u32);

        // Shared callback wrapped in Rc<RefCell> for use in event closures
        let callback = Rc::new(RefCell::new(callback));

        // --- DOM event listeners ---
        register_mouse_events(&canvas, callback.clone());
        register_keyboard_events(&canvas, callback.clone());
        register_wheel_events(&canvas, callback.clone());
        register_resize_event(&canvas, callback.clone());

        // Make canvas focusable for keyboard events
        canvas.set_tab_index(0);
        let _ = canvas.focus();

        // --- requestAnimationFrame loop ---
        let cb_clone = callback.clone();
        let canvas_clone = canvas.clone();
        let raf_closure: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let raf_clone = raf_closure.clone();

        *raf_closure.borrow_mut() = Some(Closure::new(move || {
            // Update physical size to match CSS size
            let window = web_sys::window().unwrap();
            let dpr = window.device_pixel_ratio();
            let w = canvas_clone.client_width() as f64;
            let h = canvas_clone.client_height() as f64;
            let pw = (w * dpr) as u32;
            let ph = (h * dpr) as u32;
            if canvas_clone.width() != pw || canvas_clone.height() != ph {
                canvas_clone.set_width(pw);
                canvas_clone.set_height(ph);
            }

            // Trigger redraw
            if let Ok(mut cb) = cb_clone.try_borrow_mut() {
                cb(WindowId::MAIN, PlatformEvent::RedrawRequested);
            }

            // Schedule next frame
            if let Some(ref closure) = *raf_clone.borrow() {
                let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
            }
        }));

        // Start the animation loop
        if let Some(ref closure) = *raf_closure.borrow() {
            let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
        }

        // The RAF closure is self-referential via raf_clone — prevent drop
        std::mem::forget(raf_closure);
    }
}

impl Platform for WebPlatform {
    fn run(&mut self, callback: EventCallback) -> ! {
        self.start_event_loop(callback);
        // Satisfy -> ! for the trait. In practice, App::start calls
        // start_event_loop directly and never reaches this.
        panic!("WebPlatform::run() called — use App::start() for WASM targets");
    }

    fn open_window(&mut self, options: WindowOptions) -> (WindowId, Box<dyn PlatformWindow>) {
        let canvas = Self::get_or_create_canvas(&options.title);
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        let window = WebWindow::new(
            canvas,
            (id.0 + 1) as u32,
            options.titlebar,
            options.titlebar_height,
        );
        (id, Box::new(window))
    }

    fn create_window(&self, options: WindowOptions) -> Box<dyn PlatformWindow> {
        let canvas = Self::get_or_create_canvas(&options.title);
        Box::new(WebWindow::new(
            canvas,
            1,
            options.titlebar,
            options.titlebar_height,
        ))
    }

    fn screens(&self) -> Vec<Screen> {
        let window = web_sys::window().expect("no global window");
        let screen = window.screen().expect("no screen");
        vec![Screen {
            bounds: Rect::new(
                0.0,
                0.0,
                screen.width().unwrap_or(1920) as f32,
                screen.height().unwrap_or(1080) as f32,
            ),
            work_area: Rect::new(
                0.0,
                0.0,
                screen.avail_width().unwrap_or(1920) as f32,
                screen.avail_height().unwrap_or(1080) as f32,
            ),
            scale_factor: window.device_pixel_ratio() as f32,
        }]
    }

    fn set_cursor(&self, cursor: CursorStyle) {
        let css_cursor = match cursor {
            CursorStyle::Arrow => "default",
            CursorStyle::Hand => "pointer",
            CursorStyle::Text => "text",
            CursorStyle::Crosshair => "crosshair",
            CursorStyle::NotAllowed => "not-allowed",
            CursorStyle::ResizeNS => "ns-resize",
            CursorStyle::ResizeEW => "ew-resize",
            CursorStyle::ResizeNESW => "nesw-resize",
            CursorStyle::ResizeNWSE => "nwse-resize",
        };
        if let Some(canvas) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("mozui-canvas"))
        {
            let _ = canvas
                .dyn_ref::<web_sys::HtmlElement>()
                .map(|el| el.style().set_property("cursor", css_cursor));
        }
    }

    fn clipboard_read(&self) -> Option<String> {
        // Web clipboard API is async — synchronous read is not possible.
        // For now, return None. A proper implementation would use
        // navigator.clipboard.readText() with a callback/signal.
        None
    }

    fn clipboard_write(&self, text: &str) {
        let window = web_sys::window().expect("no global window");
        let clipboard = window.navigator().clipboard();
        let _ = clipboard.write_text(text);
    }

    fn open_url(&self, url: &str) {
        if let Some(window) = web_sys::window() {
            let _ = window.open_with_url_and_target(url, "_blank");
        }
    }

    fn open_file_dialog(&self, _options: FileDialogOptions) -> Vec<PathBuf> {
        // File dialogs require async HTML input element interaction.
        // Not implementable synchronously on the web.
        Vec::new()
    }

    fn save_file_dialog(&self, _options: FileDialogOptions) -> Option<PathBuf> {
        None
    }
}

// ── DOM event registration ──────────────────────────────────────

type SharedCallback = Rc<RefCell<EventCallback>>;

fn register_mouse_events(canvas: &HtmlCanvasElement, callback: SharedCallback) {
    // Mouse move
    {
        let cb = callback.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            let pos = Point::new(event.offset_x() as f32, event.offset_y() as f32);
            let mods = js_modifiers(&event);
            if let Ok(mut cb) = cb.try_borrow_mut() {
                cb(
                    WindowId::MAIN,
                    PlatformEvent::MouseMove {
                        position: pos,
                        modifiers: mods,
                    },
                );
            }
        });
        canvas
            .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Mouse down
    {
        let cb = callback.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            let pos = Point::new(event.offset_x() as f32, event.offset_y() as f32);
            let mods = js_modifiers(&event);
            let button = match event.button() {
                0 => MouseButton::Left,
                2 => MouseButton::Right,
                _ => MouseButton::Left,
            };
            if let Ok(mut cb) = cb.try_borrow_mut() {
                cb(
                    WindowId::MAIN,
                    PlatformEvent::MouseDown {
                        button,
                        position: pos,
                        modifiers: mods,
                    },
                );
            }
        });
        canvas
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Mouse up
    {
        let cb = callback.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            let pos = Point::new(event.offset_x() as f32, event.offset_y() as f32);
            let mods = js_modifiers(&event);
            let button = match event.button() {
                0 => MouseButton::Left,
                2 => MouseButton::Right,
                _ => MouseButton::Left,
            };
            if let Ok(mut cb) = cb.try_borrow_mut() {
                cb(
                    WindowId::MAIN,
                    PlatformEvent::MouseUp {
                        button,
                        position: pos,
                        modifiers: mods,
                    },
                );
            }
        });
        canvas
            .add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Prevent context menu on right-click
    {
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            event.prevent_default();
        });
        canvas
            .add_event_listener_with_callback("contextmenu", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn register_keyboard_events(canvas: &HtmlCanvasElement, callback: SharedCallback) {
    // Key down
    {
        let cb = callback.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::KeyboardEvent| {
            event.prevent_default();
            let key = js_key_to_key(&event);
            let mods = js_key_modifiers(&event);
            let is_repeat = event.repeat();
            if let Ok(mut cb) = cb.try_borrow_mut() {
                cb(
                    WindowId::MAIN,
                    PlatformEvent::KeyDown {
                        key,
                        modifiers: mods,
                        is_repeat,
                    },
                );
            }
        });
        canvas
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Key up
    {
        let cb = callback.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::KeyboardEvent| {
            let key = js_key_to_key(&event);
            let mods = js_key_modifiers(&event);
            if let Ok(mut cb) = cb.try_borrow_mut() {
                cb(
                    WindowId::MAIN,
                    PlatformEvent::KeyUp {
                        key,
                        modifiers: mods,
                    },
                );
            }
        });
        canvas
            .add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn register_wheel_events(canvas: &HtmlCanvasElement, callback: SharedCallback) {
    let cb = callback.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::WheelEvent| {
        event.prevent_default();
        let pos = Point::new(event.offset_x() as f32, event.offset_y() as f32);
        let mods = js_key_modifiers_from_wheel(&event);
        let dx = event.delta_x() as f32;
        let dy = event.delta_y() as f32;
        let delta = match event.delta_mode() {
            0 => ScrollDelta::Pixels(dx, dy), // DOM_DELTA_PIXEL
            1 => ScrollDelta::Lines(dx, dy),  // DOM_DELTA_LINE
            _ => ScrollDelta::Pixels(dx, dy),
        };
        if let Ok(mut cb) = cb.try_borrow_mut() {
            cb(
                WindowId::MAIN,
                PlatformEvent::ScrollWheel {
                    delta,
                    position: pos,
                    modifiers: mods,
                },
            );
        }
    });

    // Use non-passive listener so we can preventDefault
    let options = web_sys::AddEventListenerOptions::new();
    options.set_passive(false);
    canvas
        .add_event_listener_with_callback_and_add_event_listener_options(
            "wheel",
            closure.as_ref().unchecked_ref(),
            &options,
        )
        .unwrap();
    closure.forget();
}

fn register_resize_event(_canvas: &HtmlCanvasElement, callback: SharedCallback) {
    let cb = callback;
    let closure = Closure::<dyn FnMut()>::new(move || {
        if let Ok(mut cb) = cb.try_borrow_mut() {
            cb(WindowId::MAIN, PlatformEvent::RedrawRequested);
        }
    });
    web_sys::window()
        .unwrap()
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

// ── Key translation ─────────────────────────────────────────────

fn js_key_to_key(event: &web_sys::KeyboardEvent) -> Key {
    let key = event.key();
    match key.as_str() {
        "Enter" => Key::Enter,
        "Escape" => Key::Escape,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        " " => Key::Space,
        "ArrowUp" => Key::ArrowUp,
        "ArrowDown" => Key::ArrowDown,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        _ => {
            let chars: Vec<char> = key.chars().collect();
            if chars.len() == 1 && !chars[0].is_control() {
                Key::Character(chars[0])
            } else {
                Key::Unknown
            }
        }
    }
}

fn js_modifiers(event: &web_sys::MouseEvent) -> Modifiers {
    Modifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key(),
    }
}

fn js_key_modifiers(event: &web_sys::KeyboardEvent) -> Modifiers {
    Modifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key(),
    }
}

fn js_key_modifiers_from_wheel(event: &web_sys::WheelEvent) -> Modifiers {
    Modifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key(),
    }
}
