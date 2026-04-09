use crate::traits::{
    EventCallback, FileDialogOptions, Platform, PlatformWindow, Screen, WindowOptions,
};
use mozui_events::{
    CursorStyle, Key, Modifiers, MouseButton, PlatformEvent, ScrollDelta, WindowId,
};
use mozui_style::{Point, Rect, Size};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use wayland_client::protocol::{wl_compositor, wl_keyboard, wl_pointer, wl_seat};
use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum, delegate_noop, globals,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

use super::window::WaylandWindow;

/// Collected state for dispatching Wayland events.
struct WaylandState {
    /// Pending events to forward to the mozui callback after dispatch.
    pending_events: Vec<(WindowId, PlatformEvent)>,
    /// Pointer position tracking.
    pointer_pos: Point,
    /// Keyboard modifiers tracking.
    modifiers: Modifiers,
    /// Current pointer serial (needed for drag-move).
    #[allow(dead_code)]
    pointer_serial: u32,
    /// Current keyboard serial.
    #[allow(dead_code)]
    keyboard_serial: u32,
    /// Window size from configure events.
    configured_size: Option<(u32, u32)>,
}

impl WaylandState {
    fn new() -> Self {
        Self {
            pending_events: Vec::new(),
            pointer_pos: Point::new(0.0, 0.0),
            modifiers: Modifiers::default(),
            pointer_serial: 0,
            keyboard_serial: 0,
            configured_size: None,
        }
    }
}

pub struct WaylandPlatform {
    connection: Connection,
    next_window_id: u64,
}

impl WaylandPlatform {
    pub fn new() -> Self {
        let connection =
            Connection::connect_to_env().expect("Failed to connect to Wayland display");
        Self {
            connection,
            next_window_id: 0,
        }
    }

    fn allocate_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        id
    }
}

impl Platform for WaylandPlatform {
    fn run(&mut self, mut callback: EventCallback) -> ! {
        let mut state = WaylandState::new();

        // Bind globals — registry_queue_init returns (GlobalList, EventQueue)
        let (global_list, mut event_queue) =
            globals::registry_queue_init::<WaylandState>(&self.connection)
                .expect("Failed to initialize Wayland registry");
        let qh = event_queue.handle();

        // Bind compositor
        let compositor: wl_compositor::WlCompositor = global_list
            .bind(&qh, 4..=6, ())
            .expect("wl_compositor not available");

        // Bind xdg_wm_base
        let wm_base: xdg_wm_base::XdgWmBase = global_list
            .bind(&qh, 1..=5, ())
            .expect("xdg_wm_base not available");

        // Bind seat (for input)
        if let Ok(seat) = global_list.bind::<wl_seat::WlSeat, _, _>(&qh, 1..=8, ()) {
            let _pointer: wl_pointer::WlPointer = seat.get_pointer(&qh, ());
            let _keyboard: wl_keyboard::WlKeyboard = seat.get_keyboard(&qh, ());
        }

        // Create surface and xdg_toplevel
        let surface = compositor.create_surface(&qh, ());
        let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
        let toplevel = xdg_surface.get_toplevel(&qh, ());
        toplevel.set_title("mozui".to_string());
        surface.commit();

        // Do an initial roundtrip to get the configure event
        event_queue
            .roundtrip(&mut state)
            .expect("Initial roundtrip failed");

        let window_id = WindowId::MAIN;
        let frame_duration = Duration::from_micros(16_667);

        // Initial draw
        callback(window_id, PlatformEvent::RedrawRequested);

        loop {
            let frame_start = Instant::now();

            // Dispatch pending Wayland events
            event_queue
                .dispatch_pending(&mut state)
                .expect("Wayland dispatch failed");

            // Also read from the socket
            if let Some(guard) = event_queue.prepare_read() {
                let _ = guard.read();
                event_queue
                    .dispatch_pending(&mut state)
                    .expect("Wayland dispatch failed");
            }

            // Forward collected events to the mozui callback
            for (wid, event) in state.pending_events.drain(..) {
                callback(wid, event);
            }

            // Handle configure size changes
            if let Some((w, h)) = state.configured_size.take() {
                if w > 0 && h > 0 {
                    callback(
                        window_id,
                        PlatformEvent::WindowResize {
                            size: Size::new(w as f32, h as f32),
                        },
                    );
                }
            }

            // Request redraw each frame
            callback(window_id, PlatformEvent::RedrawRequested);

            // Request a frame callback for vsync
            surface.frame(&qh, ());
            surface.commit();

            let elapsed = frame_start.elapsed();
            if elapsed < frame_duration {
                std::thread::sleep(frame_duration - elapsed);
            }
        }
    }

    fn open_window(&mut self, options: WindowOptions) -> (WindowId, Box<dyn PlatformWindow>) {
        let mut state = WaylandState::new();

        let (global_list, mut event_queue) =
            globals::registry_queue_init::<WaylandState>(&self.connection)
                .expect("Failed to initialize Wayland registry");
        let qh = event_queue.handle();

        let compositor: wl_compositor::WlCompositor = global_list
            .bind(&qh, 4..=6, ())
            .expect("wl_compositor not available");
        let wm_base: xdg_wm_base::XdgWmBase = global_list
            .bind(&qh, 1..=5, ())
            .expect("xdg_wm_base not available");

        let surface = compositor.create_surface(&qh, ());
        let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
        let toplevel = xdg_surface.get_toplevel(&qh, ());
        toplevel.set_title(options.title.clone());
        surface.commit();

        event_queue
            .roundtrip(&mut state)
            .expect("Roundtrip failed");

        let display_ptr = std::ptr::null_mut();
        let id = self.allocate_window_id();
        let window = WaylandWindow::new(surface, display_ptr, &options);
        (id, Box::new(window))
    }

    fn create_window(&self, options: WindowOptions) -> Box<dyn PlatformWindow> {
        let mut state = WaylandState::new();

        let (global_list, mut event_queue) =
            globals::registry_queue_init::<WaylandState>(&self.connection)
                .expect("Failed to initialize Wayland registry");
        let qh = event_queue.handle();

        let compositor: wl_compositor::WlCompositor = global_list
            .bind(&qh, 4..=6, ())
            .expect("wl_compositor not available");
        let wm_base: xdg_wm_base::XdgWmBase = global_list
            .bind(&qh, 1..=5, ())
            .expect("xdg_wm_base not available");

        let surface = compositor.create_surface(&qh, ());
        let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
        let toplevel = xdg_surface.get_toplevel(&qh, ());
        toplevel.set_title(options.title.clone());
        surface.commit();

        event_queue
            .roundtrip(&mut state)
            .expect("Roundtrip failed");

        let display_ptr = std::ptr::null_mut();
        Box::new(WaylandWindow::new(surface, display_ptr, &options))
    }

    fn screens(&self) -> Vec<Screen> {
        // Wayland doesn't expose screen geometry directly to clients.
        // Would need wl_output binding for proper support.
        vec![Screen {
            bounds: Rect::new(0.0, 0.0, 1920.0, 1080.0),
            work_area: Rect::new(0.0, 0.0, 1920.0, 1080.0),
            scale_factor: 1.0,
        }]
    }

    fn set_cursor(&self, _cursor: CursorStyle) {
        // Wayland cursor setting requires wl_pointer.set_cursor with a cursor surface.
        // Full implementation is Phase 3.
    }

    fn clipboard_read(&self) -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }

    fn clipboard_write(&self, text: &str) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    }

    fn open_url(&self, url: &str) {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }

    fn open_file_dialog(&self, _options: FileDialogOptions) -> Vec<PathBuf> {
        Vec::new()
    }

    fn save_file_dialog(&self, _options: FileDialogOptions) -> Option<PathBuf> {
        None
    }
}

// ── Wayland protocol dispatch implementations ─────────────────

delegate_noop!(WaylandState: ignore wl_compositor::WlCompositor);

impl Dispatch<wayland_client::protocol::wl_surface::WlSurface, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wayland_client::protocol::wl_surface::WlSurface,
        _event: wayland_client::protocol::wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wayland_client::protocol::wl_callback::WlCallback, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wayland_client::protocol::wl_callback::WlCallback,
        _event: wayland_client::protocol::wl_callback::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wayland_client::protocol::wl_registry::WlRegistry, globals::GlobalListContents>
    for WaylandState
{
    fn event(
        _state: &mut Self,
        _proxy: &wayland_client::protocol::wl_registry::WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &globals::GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        proxy: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        proxy: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            proxy.ack_configure(serial);
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states: _,
            } => {
                if width > 0 && height > 0 {
                    state.configured_size = Some((width as u32, height as u32));
                }
            }
            xdg_toplevel::Event::Close => {
                state
                    .pending_events
                    .push((WindowId::MAIN, PlatformEvent::WindowCloseRequested));
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.pointer_pos = Point::new(surface_x as f32, surface_y as f32);
                state.pending_events.push((
                    WindowId::MAIN,
                    PlatformEvent::MouseMove {
                        position: state.pointer_pos,
                        modifiers: state.modifiers,
                    },
                ));
            }
            wl_pointer::Event::Button {
                button,
                state: btn_state,
                serial,
                ..
            } => {
                state.pointer_serial = serial;
                let mouse_button = match button {
                    272 => MouseButton::Left,
                    273 => MouseButton::Right,
                    274 => MouseButton::Middle,
                    _ => MouseButton::Left,
                };
                let event = if btn_state == WEnum::Value(wl_pointer::ButtonState::Pressed) {
                    PlatformEvent::MouseDown {
                        button: mouse_button,
                        position: state.pointer_pos,
                        modifiers: state.modifiers,
                    }
                } else {
                    PlatformEvent::MouseUp {
                        button: mouse_button,
                        position: state.pointer_pos,
                        modifiers: state.modifiers,
                    }
                };
                state.pending_events.push((WindowId::MAIN, event));
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                let delta = match axis {
                    WEnum::Value(wl_pointer::Axis::VerticalScroll) => {
                        ScrollDelta::Pixels(0.0, value as f32)
                    }
                    WEnum::Value(wl_pointer::Axis::HorizontalScroll) => {
                        ScrollDelta::Pixels(value as f32, 0.0)
                    }
                    _ => return,
                };
                state.pending_events.push((
                    WindowId::MAIN,
                    PlatformEvent::ScrollWheel {
                        delta,
                        position: state.pointer_pos,
                        modifiers: state.modifiers,
                    },
                ));
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Key {
                key,
                state: key_state,
                serial,
                ..
            } => {
                state.keyboard_serial = serial;
                let mozui_key = evdev_to_key(key);
                let event = if key_state == WEnum::Value(wl_keyboard::KeyState::Pressed) {
                    PlatformEvent::KeyDown {
                        key: mozui_key,
                        modifiers: state.modifiers,
                        is_repeat: false,
                    }
                } else {
                    PlatformEvent::KeyUp {
                        key: mozui_key,
                        modifiers: state.modifiers,
                    }
                };
                state.pending_events.push((WindowId::MAIN, event));
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched: _,
                mods_locked: _,
                ..
            } => {
                state.modifiers = Modifiers {
                    shift: mods_depressed & 0x01 != 0,
                    ctrl: mods_depressed & 0x04 != 0,
                    alt: mods_depressed & 0x08 != 0,
                    meta: mods_depressed & 0x40 != 0,
                };
            }
            wl_keyboard::Event::Enter { serial, .. } => {
                state.keyboard_serial = serial;
                state
                    .pending_events
                    .push((WindowId::MAIN, PlatformEvent::WindowFocused));
            }
            wl_keyboard::Event::Leave { .. } => {
                state
                    .pending_events
                    .push((WindowId::MAIN, PlatformEvent::WindowBlurred));
            }
            _ => {}
        }
    }
}

// ── Key translation (evdev keycodes) ───────────────────────────

fn evdev_to_key(keycode: u32) -> Key {
    match keycode {
        1 => Key::Escape,
        14 => Key::Backspace,
        15 => Key::Tab,
        28 => Key::Enter,
        57 => Key::Space,
        102 => Key::Home,
        104 => Key::PageUp,
        107 => Key::End,
        109 => Key::PageDown,
        103 => Key::ArrowUp,
        105 => Key::ArrowLeft,
        106 => Key::ArrowRight,
        108 => Key::ArrowDown,
        111 => Key::Delete,
        59 => Key::F1,
        60 => Key::F2,
        61 => Key::F3,
        62 => Key::F4,
        63 => Key::F5,
        64 => Key::F6,
        65 => Key::F7,
        66 => Key::F8,
        67 => Key::F9,
        68 => Key::F10,
        87 => Key::F11,
        88 => Key::F12,
        16 => Key::Character('q'),
        17 => Key::Character('w'),
        18 => Key::Character('e'),
        19 => Key::Character('r'),
        20 => Key::Character('t'),
        21 => Key::Character('y'),
        22 => Key::Character('u'),
        23 => Key::Character('i'),
        24 => Key::Character('o'),
        25 => Key::Character('p'),
        30 => Key::Character('a'),
        31 => Key::Character('s'),
        32 => Key::Character('d'),
        33 => Key::Character('f'),
        34 => Key::Character('g'),
        35 => Key::Character('h'),
        36 => Key::Character('j'),
        37 => Key::Character('k'),
        38 => Key::Character('l'),
        44 => Key::Character('z'),
        45 => Key::Character('x'),
        46 => Key::Character('c'),
        47 => Key::Character('v'),
        48 => Key::Character('b'),
        49 => Key::Character('n'),
        50 => Key::Character('m'),
        2 => Key::Character('1'),
        3 => Key::Character('2'),
        4 => Key::Character('3'),
        5 => Key::Character('4'),
        6 => Key::Character('5'),
        7 => Key::Character('6'),
        8 => Key::Character('7'),
        9 => Key::Character('8'),
        10 => Key::Character('9'),
        11 => Key::Character('0'),
        _ => Key::Unknown,
    }
}
