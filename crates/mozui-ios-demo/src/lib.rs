use mozui::platform::ios::{IosDisplayMetrics, IosPlatform, IosTouchPhase};
use mozui::prelude::*;
use mozui::{
    AnyWindowHandle, App, Application, Bounds, Pixels, Point, QuitMode, RequestFrameOptions,
    Window, WindowAppearance, WindowBackgroundAppearance, WindowBounds, WindowOptions, div, hsla,
    point, px, size,
};
use mozui_components::{
    switch::Switch as ComponentSwitch,
    theme::{Theme, ThemeMode},
};
use mozui_native::NativeSwitch;
use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char, c_void},
    ptr::{self, NonNull},
    rc::Rc,
};

struct DemoRoot {
    component_switch_on: bool,
}

impl DemoRoot {
    fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            component_switch_on: true,
        }
    }

    fn set_component_switch(
        &mut self,
        checked: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.component_switch_on = *checked;
        cx.notify();
    }
}

impl Render for DemoRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(hsla(0.58, 0.52, 0.12, 1.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.0))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .w(px(6.0))
                    .bg(hsla(0.03, 0.72, 0.58, 1.0)),
            )
            .child(
                div()
                    .w(px(220.0))
                    .h(px(220.0))
                    .rounded(px(28.0))
                    .border_1()
                    .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                    .bg(hsla(0.60, 0.40, 0.17, 1.0))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .p(px(20.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.96))
                                    .child("mozui iOS"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hsla(0.0, 0.0, 1.0, 0.6))
                                    .child("components + native"),
                            ),
                    )
                    .child(
                        div()
                            .w(px(72.0))
                            .h(px(72.0))
                            .rounded(px(36.0))
                            .bg(hsla(0.12, 0.72, 0.60, 1.0)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .w(px(140.0))
                                    .h(px(18.0))
                                    .rounded(px(9.0))
                                    .bg(hsla(0.0, 0.0, 1.0, 0.18)),
                            )
                            .child(
                                div()
                                    .w(px(110.0))
                                    .h(px(12.0))
                                    .rounded(px(6.0))
                                    .bg(hsla(0.12, 0.72, 0.60, 0.55)),
                            ),
                    )
                    .child(
                        div().w_full().pt(px(8.0)).child(
                            ComponentSwitch::new("ios-demo-components-switch")
                                .checked(self.component_switch_on)
                                .color(hsla(0.12, 0.72, 0.60, 1.0))
                                .on_click(cx.listener(Self::set_component_switch)),
                        ),
                    ),
            )
            .child(
                div()
                    .w(px(220.0))
                    .h(px(80.0))
                    .rounded(px(20.0))
                    .border_1()
                    .border_color(hsla(0.0, 0.0, 1.0, 0.18))
                    .bg(hsla(0.60, 0.40, 0.17, 1.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(56.0))
                            .h(px(34.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(NativeSwitch::new("ios-demo-native-switch").is_on(true)),
                    ),
            )
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MozuiIosHostMetrics {
    pub bounds_width: f32,
    pub bounds_height: f32,
    pub visible_x: f32,
    pub visible_y: f32,
    pub visible_width: f32,
    pub visible_height: f32,
    pub scale_factor: f32,
    pub appearance: i32,
}

#[derive(Clone, Copy)]
struct PendingSurface {
    ui_view: NonNull<c_void>,
    ui_view_controller: Option<NonNull<c_void>>,
}

#[derive(Default)]
struct DemoShared {
    primary_window: RefCell<Option<AnyWindowHandle>>,
    pending_surface: RefCell<Option<PendingSurface>>,
    pending_metrics: RefCell<Option<(IosDisplayMetrics, WindowAppearance)>>,
    last_error: RefCell<Option<CString>>,
}

impl DemoShared {
    fn set_error(&self, error: impl std::fmt::Display) {
        let message = CString::new(error.to_string())
            .unwrap_or_else(|_| CString::new("mozui-ios-demo error").expect("static string"));
        *self.last_error.borrow_mut() = Some(message);
    }

    fn clear_error(&self) {
        self.last_error.borrow_mut().take();
    }

    fn last_error_ptr(&self) -> *const c_char {
        self.last_error
            .borrow()
            .as_ref()
            .map_or(ptr::null(), |error| error.as_ptr())
    }
}

pub struct MozuiIosDemo {
    platform: Rc<IosPlatform>,
    shared: Rc<DemoShared>,
    _application: Application,
}

impl MozuiIosDemo {
    fn new() -> Box<Self> {
        let platform = Rc::new(IosPlatform::new(false));
        let shared = Rc::new(DemoShared::default());

        let app_platform = platform.clone();
        let app_shared = shared.clone();
        let application =
            Application::with_platform(app_platform.clone() as Rc<dyn mozui::Platform>)
                .with_quit_mode(QuitMode::Explicit);
        application.clone().run(move |cx: &mut App| {
            mozui_components::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);

            let initial_window_bounds =
                app_shared
                    .pending_metrics
                    .borrow()
                    .map(|(metrics, _appearance)| {
                        WindowBounds::Windowed(Bounds {
                            origin: point(px(0.0), px(0.0)),
                            size: metrics.bounds.size,
                        })
                    });

            let options = WindowOptions {
                window_bounds: initial_window_bounds,
                focus: true,
                show: true,
                window_background: WindowBackgroundAppearance::Opaque,
                ..Default::default()
            };

            let window_handle = cx
                .open_window(options, |_window, cx| cx.new(DemoRoot::new))
                .expect("failed to open iOS demo window");
            let any_handle: AnyWindowHandle = window_handle.into();
            *app_shared.primary_window.borrow_mut() = Some(any_handle);

            if let Some((metrics, appearance)) = *app_shared.pending_metrics.borrow() {
                app_platform.update_display_metrics(metrics, Some(appearance));
            }

            if let Some(surface) = *app_shared.pending_surface.borrow() {
                if let Err(error) = app_platform.attach_window_surface(
                    any_handle,
                    surface.ui_view,
                    surface.ui_view_controller,
                ) {
                    app_shared.set_error(error);
                }
            }
        });

        Box::new(Self {
            platform,
            shared,
            _application: application,
        })
    }

    fn update_metrics(&self, metrics: MozuiIosHostMetrics) {
        let metrics = (
            IosDisplayMetrics {
                bounds: Bounds::new(
                    point(px(0.0), px(0.0)),
                    size(px(metrics.bounds_width), px(metrics.bounds_height)),
                ),
                visible_bounds: Bounds::new(
                    point(px(metrics.visible_x), px(metrics.visible_y)),
                    size(px(metrics.visible_width), px(metrics.visible_height)),
                ),
                scale_factor: metrics.scale_factor.max(1.0),
            },
            decode_appearance(metrics.appearance),
        );

        *self.shared.pending_metrics.borrow_mut() = Some(metrics);
        self.platform
            .update_display_metrics(metrics.0, Some(metrics.1));
    }

    fn attach_view(
        &self,
        ui_view: *mut c_void,
        ui_view_controller: *mut c_void,
    ) -> Result<(), String> {
        let ui_view =
            NonNull::new(ui_view).ok_or_else(|| String::from("received null UIView pointer"))?;
        let ui_view_controller = NonNull::new(ui_view_controller);
        let pending_surface = PendingSurface {
            ui_view,
            ui_view_controller,
        };
        *self.shared.pending_surface.borrow_mut() = Some(pending_surface);

        if let Some(handle) = *self.shared.primary_window.borrow() {
            self.platform
                .attach_window_surface(handle, ui_view, ui_view_controller)
                .map_err(|error| error.to_string())?;
        }

        Ok(())
    }

    fn detach_view(&self) {
        self.shared.pending_surface.borrow_mut().take();
        if let Some(handle) = *self.shared.primary_window.borrow() {
            self.platform.detach_window_surface(handle);
        }
    }

    fn handle_touch(&self, phase: IosTouchPhase, position: Point<Pixels>) {
        let Some(handle) = *self.shared.primary_window.borrow() else {
            return;
        };

        let _ = self.platform.dispatch_touch(handle, phase, position);
    }

    fn request_frame(&self) {
        let Some(handle) = *self.shared.primary_window.borrow() else {
            return;
        };

        self.platform.request_frame(
            handle,
            RequestFrameOptions {
                require_presentation: true,
                force_render: false,
            },
        );
    }
}

fn decode_appearance(raw: i32) -> WindowAppearance {
    match raw {
        1 => WindowAppearance::Dark,
        _ => WindowAppearance::Light,
    }
}

unsafe fn demo_from_ptr<'a>(demo: *mut MozuiIosDemo) -> Option<&'a MozuiIosDemo> {
    NonNull::new(demo).map(|demo| unsafe { demo.as_ref() })
}

#[unsafe(no_mangle)]
pub extern "C" fn mozui_ios_demo_new() -> *mut MozuiIosDemo {
    Box::into_raw(MozuiIosDemo::new())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_free(demo: *mut MozuiIosDemo) {
    if let Some(demo) = NonNull::new(demo) {
        unsafe {
            drop(Box::from_raw(demo.as_ptr()));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_attach_view(
    demo: *mut MozuiIosDemo,
    ui_view: *mut c_void,
    ui_view_controller: *mut c_void,
) -> bool {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return false;
    };
    demo.shared.clear_error();
    match demo.attach_view(ui_view, ui_view_controller) {
        Ok(()) => true,
        Err(error) => {
            demo.shared.set_error(error);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_detach_view(demo: *mut MozuiIosDemo) {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return;
    };
    demo.detach_view();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_update_metrics(
    demo: *mut MozuiIosDemo,
    metrics: MozuiIosHostMetrics,
) {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return;
    };
    demo.update_metrics(metrics);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_handle_touch(
    demo: *mut MozuiIosDemo,
    x: f32,
    y: f32,
    phase: i32,
) {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return;
    };

    let phase = match phase {
        0 => IosTouchPhase::Began,
        1 => IosTouchPhase::Moved,
        2 => IosTouchPhase::Ended,
        _ => IosTouchPhase::Cancelled,
    };

    demo.handle_touch(phase, Point::new(px(x), px(y)));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_request_frame(demo: *mut MozuiIosDemo) {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return;
    };
    demo.request_frame();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_enter_foreground(demo: *mut MozuiIosDemo) {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return;
    };
    demo.platform.enter_foreground();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_enter_background(demo: *mut MozuiIosDemo) {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return;
    };
    demo.platform.enter_background();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_last_error(demo: *mut MozuiIosDemo) -> *const c_char {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return CStr::from_bytes_with_nul(b"null demo pointer\0")
            .expect("static string")
            .as_ptr();
    };
    demo.shared.last_error_ptr()
}
