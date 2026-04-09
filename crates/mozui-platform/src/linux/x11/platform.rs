use crate::traits::{
    EventCallback, FileDialogOptions, Platform, PlatformWindow, Screen, WindowOptions,
};
use mozui_events::{
    CursorStyle, Key, Modifiers, MouseButton, PlatformEvent, ScrollDelta, WindowId,
};
use mozui_style::{Point, Rect, Size};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::xproto;
use x11rb::rust_connection::RustConnection;

use super::window::X11Window;

pub struct X11Platform {
    conn: Arc<RustConnection>,
    screen_num: usize,
    next_window_id: u64,
    window_map: HashMap<u32, WindowId>,
    wm_delete_window: u32,
    cursor_font: u32,
}

impl X11Platform {
    pub fn new() -> Self {
        let (conn, screen_num) = x11rb::connect(None).expect("Failed to connect to X11 server");
        let conn = Arc::new(conn);

        let wm_delete_window = xproto::intern_atom(&*conn, false, b"WM_DELETE_WINDOW")
            .expect("Failed to intern WM_DELETE_WINDOW")
            .reply()
            .expect("Failed to get WM_DELETE_WINDOW reply")
            .atom;

        // Open the X cursor font for set_cursor support
        let cursor_font = conn.generate_id().expect("Failed to generate cursor font ID");
        xproto::open_font(&*conn, cursor_font, b"cursor")
            .expect("Failed to open cursor font");
        conn.flush().expect("Failed to flush after opening cursor font");

        Self {
            conn,
            screen_num,
            next_window_id: 0,
            window_map: HashMap::new(),
            wm_delete_window,
            cursor_font,
        }
    }

    fn allocate_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        id
    }

    fn resolve_window_id(&self, x11_window: u32) -> WindowId {
        self.window_map
            .get(&x11_window)
            .copied()
            .unwrap_or(WindowId::MAIN)
    }
}

impl Platform for X11Platform {
    fn run(&mut self, mut callback: EventCallback) -> ! {
        let window_ids: Vec<WindowId> = self.window_map.values().copied().collect();
        let frame_duration = Duration::from_micros(16_667);

        for &wid in &window_ids {
            callback(wid, PlatformEvent::RedrawRequested);
        }

        loop {
            let frame_start = Instant::now();

            while let Ok(Some(event)) = self.conn.poll_for_event() {
                if let Some((x11_win, platform_event)) = self.translate_event(&event) {
                    let wid = self.resolve_window_id(x11_win);
                    callback(wid, platform_event);
                }
            }

            for &wid in &window_ids {
                callback(wid, PlatformEvent::RedrawRequested);
            }

            let elapsed = frame_start.elapsed();
            if elapsed < frame_duration {
                std::thread::sleep(frame_duration - elapsed);
            }
        }
    }

    fn open_window(&mut self, options: WindowOptions) -> (WindowId, Box<dyn PlatformWindow>) {
        let window = X11Window::new(self.conn.clone(), self.screen_num, &options);
        let id = self.allocate_window_id();
        self.window_map.insert(window.x11_window_id(), id);
        (id, Box::new(window))
    }

    fn create_window(&self, options: WindowOptions) -> Box<dyn PlatformWindow> {
        Box::new(X11Window::new(self.conn.clone(), self.screen_num, &options))
    }

    fn screens(&self) -> Vec<Screen> {
        let screen = &self.conn.setup().roots[self.screen_num];
        vec![Screen {
            bounds: Rect::new(
                0.0,
                0.0,
                screen.width_in_pixels as f32,
                screen.height_in_pixels as f32,
            ),
            work_area: Rect::new(
                0.0,
                0.0,
                screen.width_in_pixels as f32,
                screen.height_in_pixels as f32,
            ),
            scale_factor: 1.0,
        }]
    }

    fn set_cursor(&self, cursor: CursorStyle) {
        let glyph: u16 = match cursor {
            CursorStyle::Arrow => 68,      // XC_left_ptr
            CursorStyle::Hand => 60,       // XC_hand2
            CursorStyle::Text => 152,      // XC_xterm
            CursorStyle::Crosshair => 34,  // XC_crosshair
            CursorStyle::NotAllowed => 0,  // XC_X_cursor (closest match)
            CursorStyle::ResizeNS => 116,  // XC_sb_v_double_arrow
            CursorStyle::ResizeEW => 108,  // XC_sb_h_double_arrow
            CursorStyle::ResizeNESW => 12, // XC_bottom_left_corner
            CursorStyle::ResizeNWSE => 14, // XC_bottom_right_corner
        };
        if let Ok(cursor_id) = self.conn.generate_id() {
            let _ = xproto::create_glyph_cursor(
                &*self.conn,
                cursor_id,
                self.cursor_font,
                self.cursor_font,
                glyph,
                glyph + 1,
                0, 0, 0,           // foreground: black
                0xFFFF, 0xFFFF, 0xFFFF, // background: white
            );
            for &x11_win in self.window_map.keys() {
                let values = xproto::ChangeWindowAttributesAux::new().cursor(cursor_id);
                let _ = xproto::change_window_attributes(&*self.conn, x11_win, &values);
            }
            let _ = self.conn.flush();
            let _ = xproto::free_cursor(&*self.conn, cursor_id);
        }
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

// ── X11 event translation ──────────────────────────────────────

impl X11Platform {
    fn translate_event(&self, event: &x11rb::protocol::Event) -> Option<(u32, PlatformEvent)> {
        use x11rb::protocol::Event;

        match event {
            Event::MotionNotify(e) => {
                let pos = Point::new(e.event_x as f32, e.event_y as f32);
                let mods = x11_state_to_modifiers(e.state);
                Some((
                    e.event,
                    PlatformEvent::MouseMove {
                        position: pos,
                        modifiers: mods,
                    },
                ))
            }
            Event::ButtonPress(e) => {
                let pos = Point::new(e.event_x as f32, e.event_y as f32);
                let mods = x11_state_to_modifiers(e.state);
                match e.detail {
                    1 | 2 | 3 => {
                        let button = match e.detail {
                            1 => MouseButton::Left,
                            2 => MouseButton::Middle,
                            3 => MouseButton::Right,
                            _ => unreachable!(),
                        };
                        Some((
                            e.event,
                            PlatformEvent::MouseDown {
                                button,
                                position: pos,
                                modifiers: mods,
                            },
                        ))
                    }
                    4 => Some((
                        e.event,
                        PlatformEvent::ScrollWheel {
                            delta: ScrollDelta::Lines(0.0, -1.0),
                            position: pos,
                            modifiers: mods,
                        },
                    )),
                    5 => Some((
                        e.event,
                        PlatformEvent::ScrollWheel {
                            delta: ScrollDelta::Lines(0.0, 1.0),
                            position: pos,
                            modifiers: mods,
                        },
                    )),
                    6 => Some((
                        e.event,
                        PlatformEvent::ScrollWheel {
                            delta: ScrollDelta::Lines(-1.0, 0.0),
                            position: pos,
                            modifiers: mods,
                        },
                    )),
                    7 => Some((
                        e.event,
                        PlatformEvent::ScrollWheel {
                            delta: ScrollDelta::Lines(1.0, 0.0),
                            position: pos,
                            modifiers: mods,
                        },
                    )),
                    _ => None,
                }
            }
            Event::ButtonRelease(e) => {
                let pos = Point::new(e.event_x as f32, e.event_y as f32);
                let mods = x11_state_to_modifiers(e.state);
                let button = match e.detail {
                    1 => MouseButton::Left,
                    2 => MouseButton::Middle,
                    3 => MouseButton::Right,
                    _ => return None,
                };
                Some((
                    e.event,
                    PlatformEvent::MouseUp {
                        button,
                        position: pos,
                        modifiers: mods,
                    },
                ))
            }
            Event::KeyPress(e) => {
                let mods = x11_state_to_modifiers(e.state);
                let key = x11_keycode_to_key(e.detail);
                Some((
                    e.event,
                    PlatformEvent::KeyDown {
                        key,
                        modifiers: mods,
                        is_repeat: false,
                    },
                ))
            }
            Event::KeyRelease(e) => {
                let mods = x11_state_to_modifiers(e.state);
                let key = x11_keycode_to_key(e.detail);
                Some((
                    e.event,
                    PlatformEvent::KeyUp {
                        key,
                        modifiers: mods,
                    },
                ))
            }
            Event::ConfigureNotify(e) => Some((
                e.window,
                PlatformEvent::WindowResize {
                    size: Size::new(e.width as f32, e.height as f32),
                },
            )),
            Event::FocusIn(e) => Some((e.event, PlatformEvent::WindowFocused)),
            Event::FocusOut(e) => Some((e.event, PlatformEvent::WindowBlurred)),
            Event::Expose(e) => {
                if e.count == 0 {
                    Some((e.window, PlatformEvent::RedrawRequested))
                } else {
                    None
                }
            }
            Event::ClientMessage(e) => {
                let data = e.data.as_data32();
                if data[0] == self.wm_delete_window {
                    Some((e.window, PlatformEvent::WindowCloseRequested))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

// ── Input helpers ──────────────────────────────────────────────

fn x11_state_to_modifiers(state: xproto::KeyButMask) -> Modifiers {
    Modifiers {
        shift: state.contains(xproto::KeyButMask::SHIFT),
        ctrl: state.contains(xproto::KeyButMask::CONTROL),
        alt: state.contains(xproto::KeyButMask::MOD1),
        meta: state.contains(xproto::KeyButMask::MOD4),
    }
}

fn x11_keycode_to_key(keycode: u8) -> Key {
    match keycode {
        9 => Key::Escape,
        22 => Key::Backspace,
        23 => Key::Tab,
        36 => Key::Enter,
        65 => Key::Space,
        110 => Key::Home,
        112 => Key::PageUp,
        115 => Key::End,
        117 => Key::PageDown,
        111 => Key::ArrowUp,
        113 => Key::ArrowLeft,
        114 => Key::ArrowRight,
        116 => Key::ArrowDown,
        119 => Key::Delete,
        67 => Key::F1,
        68 => Key::F2,
        69 => Key::F3,
        70 => Key::F4,
        71 => Key::F5,
        72 => Key::F6,
        73 => Key::F7,
        74 => Key::F8,
        75 => Key::F9,
        76 => Key::F10,
        95 => Key::F11,
        96 => Key::F12,
        24 => Key::Character('q'),
        25 => Key::Character('w'),
        26 => Key::Character('e'),
        27 => Key::Character('r'),
        28 => Key::Character('t'),
        29 => Key::Character('y'),
        30 => Key::Character('u'),
        31 => Key::Character('i'),
        32 => Key::Character('o'),
        33 => Key::Character('p'),
        38 => Key::Character('a'),
        39 => Key::Character('s'),
        40 => Key::Character('d'),
        41 => Key::Character('f'),
        42 => Key::Character('g'),
        43 => Key::Character('h'),
        44 => Key::Character('j'),
        45 => Key::Character('k'),
        46 => Key::Character('l'),
        52 => Key::Character('z'),
        53 => Key::Character('x'),
        54 => Key::Character('c'),
        55 => Key::Character('v'),
        56 => Key::Character('b'),
        57 => Key::Character('n'),
        58 => Key::Character('m'),
        10 => Key::Character('1'),
        11 => Key::Character('2'),
        12 => Key::Character('3'),
        13 => Key::Character('4'),
        14 => Key::Character('5'),
        15 => Key::Character('6'),
        16 => Key::Character('7'),
        17 => Key::Character('8'),
        18 => Key::Character('9'),
        19 => Key::Character('0'),
        _ => Key::Unknown,
    }
}
