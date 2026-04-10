use crate::traits::{PlatformWindow, TitlebarStyle, WindowOptions};
use mozui_style::{Rect, Size};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle, XcbDisplayHandle, XcbWindowHandle,
};
use std::num::NonZeroU32;
use x11rb::connection::Connection;
use x11rb::protocol::xproto;
use x11rb::rust_connection::RustConnection;

pub struct X11Window {
    conn: std::sync::Arc<RustConnection>,
    screen_num: usize,
    window_id: u32,
    width: u32,
    height: u32,
    titlebar_height: f32,
    titlebar_style: TitlebarStyle,
}

impl X11Window {
    pub fn new(
        conn: std::sync::Arc<RustConnection>,
        screen_num: usize,
        options: &WindowOptions,
    ) -> Self {
        let screen = &conn.setup().roots[screen_num];
        let window_id = conn
            .generate_id()
            .expect("Failed to generate X11 window ID");
        let width = options.size.width as u32;
        let height = options.size.height as u32;

        let values = xproto::CreateWindowAux::new()
            .event_mask(
                xproto::EventMask::EXPOSURE
                    | xproto::EventMask::STRUCTURE_NOTIFY
                    | xproto::EventMask::POINTER_MOTION
                    | xproto::EventMask::BUTTON_PRESS
                    | xproto::EventMask::BUTTON_RELEASE
                    | xproto::EventMask::KEY_PRESS
                    | xproto::EventMask::KEY_RELEASE
                    | xproto::EventMask::FOCUS_CHANGE
                    | xproto::EventMask::ENTER_WINDOW
                    | xproto::EventMask::LEAVE_WINDOW,
            )
            .background_pixel(screen.black_pixel);

        xproto::create_window(
            &*conn,
            x11rb::COPY_DEPTH_FROM_PARENT,
            window_id,
            screen.root,
            0,
            0,
            width as u16,
            height as u16,
            0,
            xproto::WindowClass::INPUT_OUTPUT,
            0,
            &values,
        )
        .expect("Failed to create X11 window");

        // Set window title via _NET_WM_NAME (UTF-8) and WM_NAME (legacy)
        xproto::change_property(
            &*conn,
            xproto::PropMode::REPLACE,
            window_id,
            xproto::AtomEnum::WM_NAME,
            xproto::AtomEnum::STRING,
            8,
            options.title.as_bytes().len() as u32,
            options.title.as_bytes(),
        )
        .expect("Failed to set WM_NAME");

        // Intern _NET_WM_NAME and UTF8_STRING atoms for modern WMs
        if let (Ok(net_wm_name), Ok(utf8_string)) = (
            xproto::intern_atom(&*conn, false, b"_NET_WM_NAME"),
            xproto::intern_atom(&*conn, false, b"UTF8_STRING"),
        ) {
            if let (Ok(net_wm_name), Ok(utf8_string)) = (net_wm_name.reply(), utf8_string.reply()) {
                let _ = xproto::change_property(
                    &*conn,
                    xproto::PropMode::REPLACE,
                    window_id,
                    net_wm_name.atom,
                    utf8_string.atom,
                    8,
                    options.title.as_bytes().len() as u32,
                    options.title.as_bytes(),
                );
            }
        }

        // Register for WM_DELETE_WINDOW protocol
        if let Ok(wm_protocols) = xproto::intern_atom(&*conn, false, b"WM_PROTOCOLS") {
            if let Ok(wm_delete) = xproto::intern_atom(&*conn, false, b"WM_DELETE_WINDOW") {
                if let (Ok(protocols), Ok(delete)) = (wm_protocols.reply(), wm_delete.reply()) {
                    let _ = xproto::change_property(
                        &*conn,
                        xproto::PropMode::REPLACE,
                        window_id,
                        protocols.atom,
                        xproto::AtomEnum::ATOM,
                        32,
                        1,
                        &delete.atom.to_ne_bytes(),
                    );
                }
            }
        }

        // Set min/max size hints via WM_NORMAL_HINTS
        if options.min_size.is_some() || options.max_size.is_some() {
            // WM_NORMAL_HINTS format: 18 × i32 (flags + geometry hints)
            // Flags: PMinSize = 1<<4 (16), PMaxSize = 1<<5 (32)
            let mut hints = [0i32; 18];
            if let Some(min) = options.min_size {
                hints[0] |= 16; // PMinSize
                hints[5] = min.width as i32;
                hints[6] = min.height as i32;
            }
            if let Some(max) = options.max_size {
                hints[0] |= 32; // PMaxSize
                hints[7] = max.width as i32;
                hints[8] = max.height as i32;
            }
            let hints_bytes: Vec<u8> = hints.iter().flat_map(|v| v.to_ne_bytes()).collect();
            let _ = xproto::change_property(
                &*conn,
                xproto::PropMode::REPLACE,
                window_id,
                xproto::AtomEnum::WM_NORMAL_HINTS,
                xproto::AtomEnum::WM_SIZE_HINTS,
                32,
                18,
                &hints_bytes,
            );
        }

        // Map (show) the window
        if options.visible {
            xproto::map_window(&*conn, window_id).expect("Failed to map window");
        }

        conn.flush().expect("Failed to flush X11 connection");

        Self {
            conn,
            screen_num,
            window_id,
            width,
            height,
            titlebar_height: options.titlebar_height,
            titlebar_style: options.titlebar,
        }
    }

    pub fn x11_window_id(&self) -> u32 {
        self.window_id
    }

    /// Update cached size (called from event loop on ConfigureNotify).
    #[allow(dead_code)]
    pub fn update_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn scale_factor_from_xrdb(&self) -> f32 {
        if let Ok(db) = x11rb::resource_manager::new_from_default(&*self.conn) {
            if let Some(dpi_str) = db.get_string("Xft.dpi", "Xft.Dpi") {
                if let Ok(dpi) = dpi_str.parse::<f32>() {
                    return dpi / 96.0;
                }
            }
        }
        1.0
    }
}

impl PlatformWindow for X11Window {
    fn bounds(&self) -> Rect {
        if let Ok(geom) = xproto::get_geometry(&*self.conn, self.window_id) {
            if let Ok(geom) = geom.reply() {
                return Rect::new(
                    geom.x as f32,
                    geom.y as f32,
                    geom.width as f32,
                    geom.height as f32,
                );
            }
        }
        Rect::new(0.0, 0.0, self.width as f32, self.height as f32)
    }

    fn set_bounds(&mut self, bounds: Rect) {
        let values = xproto::ConfigureWindowAux::new()
            .x(bounds.origin.x as i32)
            .y(bounds.origin.y as i32)
            .width(bounds.size.width as u32)
            .height(bounds.size.height as u32);
        let _ = xproto::configure_window(&*self.conn, self.window_id, &values);
        let _ = self.conn.flush();
    }

    fn content_size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor_from_xrdb()
    }

    fn is_focused(&self) -> bool {
        if let Ok(reply) = xproto::get_input_focus(&*self.conn) {
            if let Ok(focus) = reply.reply() {
                return focus.focus == self.window_id;
            }
        }
        false
    }

    fn is_visible(&self) -> bool {
        if let Ok(reply) = xproto::get_window_attributes(&*self.conn, self.window_id) {
            if let Ok(attrs) = reply.reply() {
                return attrs.map_state == xproto::MapState::VIEWABLE;
            }
        }
        false
    }

    fn is_maximized(&self) -> bool {
        false
    }

    fn set_title(&mut self, title: &str) {
        let _ = xproto::change_property(
            &*self.conn,
            xproto::PropMode::REPLACE,
            self.window_id,
            xproto::AtomEnum::WM_NAME,
            xproto::AtomEnum::STRING,
            8,
            title.as_bytes().len() as u32,
            title.as_bytes(),
        );
        let _ = self.conn.flush();
    }

    fn minimize(&mut self) {
        let screen = &self.conn.setup().roots[self.screen_num];
        let data = xproto::ClientMessageData::from([3u32, 0, 0, 0, 0]);
        if let Ok(wm_change_state) = xproto::intern_atom(&*self.conn, false, b"WM_CHANGE_STATE") {
            if let Ok(atom) = wm_change_state.reply() {
                let event = xproto::ClientMessageEvent::new(32, self.window_id, atom.atom, data);
                let _ = xproto::send_event(
                    &*self.conn,
                    false,
                    screen.root,
                    xproto::EventMask::SUBSTRUCTURE_REDIRECT
                        | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                    event,
                );
                let _ = self.conn.flush();
            }
        }
    }

    fn maximize(&mut self) {
        let screen = &self.conn.setup().roots[self.screen_num];
        if let (Ok(net_wm_state), Ok(max_h), Ok(max_v)) = (
            xproto::intern_atom(&*self.conn, false, b"_NET_WM_STATE"),
            xproto::intern_atom(&*self.conn, false, b"_NET_WM_STATE_MAXIMIZED_HORZ"),
            xproto::intern_atom(&*self.conn, false, b"_NET_WM_STATE_MAXIMIZED_VERT"),
        ) {
            if let (Ok(state), Ok(h), Ok(v)) = (net_wm_state.reply(), max_h.reply(), max_v.reply())
            {
                let data = xproto::ClientMessageData::from([1u32, h.atom, v.atom, 1, 0]);
                let event = xproto::ClientMessageEvent::new(32, self.window_id, state.atom, data);
                let _ = xproto::send_event(
                    &*self.conn,
                    false,
                    screen.root,
                    xproto::EventMask::SUBSTRUCTURE_REDIRECT
                        | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                    event,
                );
                let _ = self.conn.flush();
            }
        }
    }

    fn close(&mut self) {
        let _ = xproto::destroy_window(&*self.conn, self.window_id);
        let _ = self.conn.flush();
    }

    fn request_redraw(&self) {
        let event = xproto::ExposeEvent {
            response_type: x11rb::protocol::xproto::EXPOSE_EVENT,
            sequence: 0,
            window: self.window_id,
            x: 0,
            y: 0,
            width: self.width as u16,
            height: self.height as u16,
            count: 0,
        };
        let _ = xproto::send_event(
            &*self.conn,
            false,
            self.window_id,
            xproto::EventMask::EXPOSURE,
            event,
        );
        let _ = self.conn.flush();
    }

    fn begin_drag_move(&self) {
        let screen = &self.conn.setup().roots[self.screen_num];
        if let Ok(moveresize) = xproto::intern_atom(&*self.conn, false, b"_NET_WM_MOVERESIZE") {
            if let Ok(atom) = moveresize.reply() {
                let data = xproto::ClientMessageData::from([0u32, 0, 8, 1, 1]);
                let event = xproto::ClientMessageEvent::new(32, self.window_id, atom.atom, data);
                let _ = xproto::send_event(
                    &*self.conn,
                    false,
                    screen.root,
                    xproto::EventMask::SUBSTRUCTURE_REDIRECT
                        | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                    event,
                );
                let _ = self.conn.flush();
            }
        }
    }

    fn titlebar_height(&self) -> f32 {
        self.titlebar_height
    }

    fn titlebar_style(&self) -> TitlebarStyle {
        self.titlebar_style
    }
}

impl HasWindowHandle for X11Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = XcbWindowHandle::new(
            NonZeroU32::new(self.window_id).expect("X11 window ID must not be zero"),
        );
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Xcb(handle)) })
    }
}

impl HasDisplayHandle for X11Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let handle = XcbDisplayHandle::new(None, self.screen_num as i32);
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Xcb(handle)) })
    }
}
