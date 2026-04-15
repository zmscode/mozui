use super::{MacWindowState, events::platform_input_from_native, renderer};
use cocoa::{
    base::{BOOL, id, nil},
    foundation::{NSPoint, NSRect, NSSize},
};
use ctor::ctor;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use parking_lot::Mutex;
use std::{ffi::c_void, mem, ptr, sync::Arc};

use crate::{
    DevicePixels, DispatchEventResult, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    Pixels, PlatformAtlas, PlatformInput, PlatformSurface, Scene, Size, px, size,
};

const WINDOW_STATE_IVAR: &str = "windowStatePtr";

const NS_TRACKING_MOUSE_ENTERED_AND_EXITED: u64 = 0x01;
const NS_TRACKING_MOUSE_MOVED: u64 = 0x02;
const NS_TRACKING_ACTIVE_ALWAYS: u64 = 0x80;
const NS_TRACKING_IN_VISIBLE_RECT: u64 = 0x200;

static mut MOZUI_SURFACE_VIEW_CLASS: *const Class = ptr::null();

#[ctor]
unsafe fn build_mozui_surface_view_class() {
    unsafe {
        let mut decl = ClassDecl::new("MozuiSurfaceView", class!(NSView)).unwrap();

        decl.add_method(
            sel!(makeBackingLayer),
            make_backing_layer as extern "C" fn(&Object, Sel) -> id,
        );
        decl.add_method(
            sel!(wantsLayer),
            wants_layer as extern "C" fn(&Object, Sel) -> i8,
        );
        decl.add_method(
            sel!(isFlipped),
            is_flipped as extern "C" fn(&Object, Sel) -> i8,
        );
        decl.add_method(
            sel!(acceptsFirstResponder),
            accepts_first_responder as extern "C" fn(&Object, Sel) -> i8,
        );
        decl.add_method(
            sel!(wantsUpdateLayer),
            wants_update_layer as extern "C" fn(&Object, Sel) -> i8,
        );

        decl.add_method(
            sel!(mouseDown:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseUp:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(rightMouseDown:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(rightMouseUp:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(otherMouseDown:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(otherMouseUp:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseMoved:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseExited:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseDragged:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(rightMouseDragged:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(otherMouseDragged:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(scrollWheel:),
            handle_surface_view_event as extern "C" fn(&Object, Sel, id),
        );

        decl.add_method(
            sel!(updateTrackingAreas),
            update_tracking_areas as extern "C" fn(&Object, Sel),
        );

        decl.add_method(
            sel!(keyDown:),
            handle_surface_key_down as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(keyUp:),
            handle_surface_key_up as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(flagsChanged:),
            handle_surface_flags_changed as extern "C" fn(&Object, Sel, id),
        );

        decl.add_ivar::<*mut c_void>("metalLayerPtr");
        decl.add_ivar::<*mut c_void>(WINDOW_STATE_IVAR);

        MOZUI_SURFACE_VIEW_CLASS = decl.register();
    }
}

extern "C" fn make_backing_layer(this: &Object, _sel: Sel) -> id {
    unsafe {
        let layer_ptr: *mut c_void = *this.get_ivar("metalLayerPtr");
        if layer_ptr.is_null() {
            msg_send![class!(CALayer), layer]
        } else {
            layer_ptr as id
        }
    }
}

extern "C" fn wants_layer(_this: &Object, _sel: Sel) -> i8 {
    1
}

extern "C" fn is_flipped(_this: &Object, _sel: Sel) -> i8 {
    1
}

extern "C" fn accepts_first_responder(_this: &Object, _sel: Sel) -> i8 {
    1
}

extern "C" fn wants_update_layer(_this: &Object, _sel: Sel) -> i8 {
    1
}

fn get_window_state(view: &Object) -> Option<Arc<Mutex<MacWindowState>>> {
    unsafe {
        let raw: *mut c_void = *view.get_ivar(WINDOW_STATE_IVAR);
        if raw.is_null() {
            return None;
        }

        let rc: Arc<Mutex<MacWindowState>> = Arc::from_raw(raw as *mut Mutex<MacWindowState>);
        let clone = rc.clone();
        mem::forget(rc);
        Some(clone)
    }
}

fn get_main_native_view(window_state: &Arc<Mutex<MacWindowState>>) -> id {
    let lock = window_state.lock();
    lock.native_view.as_ptr() as id
}

fn transfer_first_responder_to_main_view(
    surface_view: &Object,
    window_state: &Arc<Mutex<MacWindowState>>,
) {
    let main_view = get_main_native_view(window_state);
    unsafe {
        let window: id = msg_send![surface_view, window];
        if window == nil {
            return;
        }
        let _: BOOL = msg_send![window, makeFirstResponder: main_view];
    }
}

extern "C" fn update_tracking_areas(this: &Object, _sel: Sel) {
    unsafe {
        let superclass = class!(NSView);
        let _: () = msg_send![super(this, superclass), updateTrackingAreas];

        let areas: id = msg_send![this, trackingAreas];
        let count: u64 = msg_send![areas, count];
        for i in (0..count).rev() {
            let area: id = msg_send![areas, objectAtIndex: i];
            let _: () = msg_send![this, removeTrackingArea: area];
        }

        let options: u64 = NS_TRACKING_MOUSE_ENTERED_AND_EXITED
            | NS_TRACKING_MOUSE_MOVED
            | NS_TRACKING_ACTIVE_ALWAYS
            | NS_TRACKING_IN_VISIBLE_RECT;
        let tracking_area: id = msg_send![class!(NSTrackingArea), alloc];
        let tracking_area: id = msg_send![
            tracking_area,
            initWithRect: NSRect::new(NSPoint::new(0., 0.), NSSize::new(0., 0.))
            options: options
            owner: this
            userInfo: nil
        ];
        let _: () = msg_send![this, addTrackingArea: tracking_area];
        let _: () = msg_send![tracking_area, release];
    }
}

extern "C" fn handle_surface_view_event(this: &Object, _sel: Sel, native_event: id) {
    let Some(window_state) = get_window_state(this) else {
        return;
    };

    let bounds: NSRect = unsafe { msg_send![this, bounds] };
    let view_height = px(bounds.size.height as f32);
    let event = unsafe {
        platform_input_from_native(
            native_event,
            Some(view_height),
            Some(this as *const _ as id),
        )
    };

    if let Some(mut event) = event {
        let is_mouse_down = matches!(&event, PlatformInput::MouseDown(_));

        match &mut event {
            PlatformInput::MouseDown(
                down @ MouseDownEvent {
                    button: MouseButton::Left,
                    modifiers: Modifiers { control: true, .. },
                    ..
                },
            ) => {
                *down = MouseDownEvent {
                    button: MouseButton::Right,
                    modifiers: Modifiers {
                        control: false,
                        ..down.modifiers
                    },
                    click_count: 1,
                    ..*down
                };
            }
            PlatformInput::MouseUp(
                up @ MouseUpEvent {
                    button: MouseButton::Left,
                    modifiers: Modifiers { control: true, .. },
                    ..
                },
            ) => {
                *up = MouseUpEvent {
                    button: MouseButton::Right,
                    modifiers: Modifiers {
                        control: false,
                        ..up.modifiers
                    },
                    ..*up
                };
            }
            _ => {}
        }

        let native_view_ptr = this as *const _ as *mut c_void;
        let mut lock = window_state.lock();
        if let Some(mut callback) = lock.surface_event_callback.take() {
            drop(lock);
            let _: DispatchEventResult = callback(native_view_ptr, event);
            window_state.lock().surface_event_callback = Some(callback);
        } else {
            drop(lock);
        }

        if is_mouse_down {
            transfer_first_responder_to_main_view(this, &window_state);
        }
    }
}

extern "C" fn handle_surface_key_down(this: &Object, _sel: Sel, native_event: id) {
    let Some(window_state) = get_window_state(this) else {
        return;
    };
    let main_view = get_main_native_view(&window_state);
    unsafe {
        let _: () = msg_send![main_view, keyDown: native_event];
    }
}

extern "C" fn handle_surface_key_up(this: &Object, _sel: Sel, native_event: id) {
    let Some(window_state) = get_window_state(this) else {
        return;
    };
    let main_view = get_main_native_view(&window_state);
    unsafe {
        let _: () = msg_send![main_view, keyUp: native_event];
    }
}

extern "C" fn handle_surface_flags_changed(this: &Object, _sel: Sel, native_event: id) {
    let Some(window_state) = get_window_state(this) else {
        return;
    };
    let main_view = get_main_native_view(&window_state);
    unsafe {
        let _: () = msg_send![main_view, flagsChanged: native_event];
    }
}

pub(crate) struct MozuiSurface {
    renderer: renderer::Renderer,
    native_view: id,
    has_window_state: bool,
}

impl MozuiSurface {
    pub fn new(context: renderer::Context, transparent: bool) -> Self {
        let renderer = renderer::Renderer::new(context, transparent);

        let native_view = unsafe {
            let view: id = msg_send![MOZUI_SURFACE_VIEW_CLASS, alloc];
            let view: id = msg_send![view, initWithFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(100.0, 100.0),
            )];

            let layer_ptr = renderer.layer_ptr() as *mut c_void;
            (*(view as *mut Object)).set_ivar::<*mut c_void>("metalLayerPtr", layer_ptr);
            (*(view as *mut Object)).set_ivar::<*mut c_void>(WINDOW_STATE_IVAR, ptr::null_mut());

            let _: () = msg_send![view, setWantsLayer: 1i8];
            view
        };

        Self {
            renderer,
            native_view,
            has_window_state: false,
        }
    }

    pub fn native_view_ptr(&self) -> *mut c_void {
        self.native_view as *mut c_void
    }

    pub fn draw(&mut self, scene: &Scene) {
        self.renderer.draw(scene);
    }

    pub fn update_drawable_size(&mut self, size: Size<DevicePixels>) {
        self.renderer.update_drawable_size(size);
    }

    pub fn content_size(&self) -> Size<Pixels> {
        unsafe {
            let frame: NSRect = msg_send![self.native_view, frame];
            size(px(frame.size.width as f32), px(frame.size.height as f32))
        }
    }

    pub fn set_contents_scale(&self, scale: f64) {
        unsafe {
            let layer: id = msg_send![self.native_view, layer];
            if layer != nil {
                let _: () = msg_send![layer, setContentsScale: scale];
            }
        }
    }

    pub fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.renderer.sprite_atlas().clone()
    }

    pub fn set_window_state(&mut self, raw_state_ptr: *const c_void) {
        unsafe {
            if self.has_window_state {
                let prev: *mut c_void = *(*self.native_view).get_ivar(WINDOW_STATE_IVAR);
                if !prev.is_null() {
                    let _drop = Arc::from_raw(prev as *mut Mutex<MacWindowState>);
                }
            }
            (*(self.native_view as *mut Object))
                .set_ivar::<*mut c_void>(WINDOW_STATE_IVAR, raw_state_ptr as *mut c_void);
            self.has_window_state = !raw_state_ptr.is_null();
        }
    }
}

impl PlatformSurface for MozuiSurface {
    fn native_view_ptr(&self) -> *mut c_void {
        self.native_view_ptr()
    }

    fn content_size(&self) -> Size<Pixels> {
        self.content_size()
    }

    fn set_contents_scale(&self, scale: f64) {
        self.set_contents_scale(scale);
    }

    fn update_drawable_size(&mut self, size: Size<DevicePixels>) {
        self.update_drawable_size(size);
    }

    fn draw(&mut self, scene: &Scene) {
        self.draw(scene);
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.sprite_atlas()
    }

    fn set_window_state(&mut self, raw_state_ptr: *const c_void) {
        self.set_window_state(raw_state_ptr);
    }
}

impl Drop for MozuiSurface {
    fn drop(&mut self) {
        unsafe {
            if self.native_view != nil {
                if self.has_window_state {
                    let raw: *mut c_void = *(*self.native_view).get_ivar(WINDOW_STATE_IVAR);
                    if !raw.is_null() {
                        let _drop = Arc::from_raw(raw as *mut Mutex<MacWindowState>);
                    }
                }
                let _: () = msg_send![self.native_view, removeFromSuperview];
                let _: () = msg_send![self.native_view, release];
            }
        }
    }
}
