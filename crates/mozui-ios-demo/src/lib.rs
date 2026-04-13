use mozui::platform::ios::{IosDisplayMetrics, IosPlatform, IosTouchPhase};
use mozui::prelude::*;
use mozui::{
    AnyWindowHandle, App, Application, Bounds, ClickEvent, Div, Entity, Pixels, Point, QuitMode,
    RequestFrameOptions, TouchPhase, Window, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, div, hsla, point, px, size,
};
use mozui_components::{
    Root, Sizable,
    button::Button,
    checkbox::Checkbox,
    input::{Input, InputState},
    progress::Progress,
    radio::Radio,
    scroll::ScrollableElement,
    slider::{Slider, SliderState, SliderValue},
    switch::Switch as ComponentSwitch,
    theme::{Theme, ThemeMode},
};
use mozui_native::{NativeButton, NativeProgress, NativeSlider, NativeSwitch};
use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char, c_void},
    ptr::{self, NonNull},
    rc::Rc,
};

struct DemoRoot {
    button_taps: usize,
    component_checkbox_on: bool,
    component_radio_on: bool,
    component_switch_on: bool,
    input_state: Entity<InputState>,
    slider_state: Entity<SliderState>,
}

impl DemoRoot {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            button_taps: 0,
            component_checkbox_on: true,
            component_radio_on: true,
            component_switch_on: true,
            input_state: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Type here on iPhone…")
                    .default_value("")
            }),
            slider_state: cx.new(|_| SliderState::new().min(0.0).max(100.0).default_value(42.0)),
        }
    }

    fn increment_button_taps(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.button_taps += 1;
        cx.notify();
    }

    fn set_component_checkbox(
        &mut self,
        checked: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.component_checkbox_on = *checked;
        cx.notify();
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

    fn set_component_radio(
        &mut self,
        checked: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.component_radio_on = *checked;
        cx.notify();
    }
}

fn showcase_card(title: &'static str, subtitle: &'static str) -> Div {
    div()
        .w(px(280.0))
        .rounded(px(24.0))
        .border_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.18))
        .bg(hsla(0.60, 0.40, 0.17, 1.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .p(px(16.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .text_sm()
                        .text_color(hsla(0.0, 0.0, 1.0, 0.92))
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(hsla(0.0, 0.0, 1.0, 0.58))
                        .child(subtitle),
                ),
        )
}

fn showcase_row(label: &'static str) -> Div {
    div()
        .w_full()
        .rounded(px(18.0))
        .bg(hsla(0.0, 0.0, 1.0, 0.04))
        .border_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.08))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .p(px(14.0))
        .child(
            div()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 1.0, 0.54))
                .child(label),
        )
}

fn native_unavailable(name: &'static str, note: &'static str) -> Div {
    showcase_row(name).child(
        div()
            .rounded(px(14.0))
            .bg(hsla(0.0, 0.0, 1.0, 0.05))
            .border_1()
            .border_color(hsla(0.0, 0.0, 1.0, 0.08))
            .p(px(12.0))
            .text_xs()
            .text_color(hsla(0.0, 0.0, 1.0, 0.62))
            .child(note),
    )
}

impl Render for DemoRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let slider_value = match self.slider_state.read(cx).value() {
            SliderValue::Single(value) => value,
            SliderValue::Range(_, value) => value,
        };

        div()
            .size_full()
            .relative()
            .bg(hsla(0.58, 0.52, 0.12, 1.0))
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
                    .size_full()
                    .overflow_y_scrollbar()
                    .child(
                        div()
                            .w_full()
                            .flex_col()
                            .items_center()
                            .gap(px(16.0))
                            .p(px(24.0))
                            .pb(px(120.0))
                            .child(
                                showcase_card("mozui iOS", "components + native showcase").child(
                                    div()
                                        .text_xs()
                                        .text_color(hsla(0.0, 0.0, 1.0, 0.62))
                                        .child(
                                            "Swipe to scroll, tap to interact, and compare each pair directly.",
                                        ),
                                ),
                            )
                            .child(
                                showcase_card("Button", "mozui-components above, mozui-native below")
                                    .child(
                                        showcase_row("mozui-components").child(
                                            Button::new("ios-demo-button")
                                                .label(format!("Button taps: {}", self.button_taps))
                                                .on_click(cx.listener(Self::increment_button_taps)),
                                        ),
                                    )
                                    .child(
                                        showcase_row("mozui-native").child(
                                            div()
                                                .h(px(36.0))
                                                .flex()
                                                .items_center()
                                                .child(NativeButton::new(
                                                    "ios-demo-native-button",
                                                    "Native UIButton",
                                                )),
                                        ),
                                    ),
                            )
                            .child(
                                showcase_card("Checkbox", "Component checkbox with current iOS native gap")
                                    .child(
                                        showcase_row("mozui-components").child(
                                            Checkbox::new("ios-demo-checkbox")
                                                .label("Checkbox")
                                                .checked(self.component_checkbox_on)
                                                .on_click(cx.listener(Self::set_component_checkbox)),
                                        ),
                                    )
                                    .child(native_unavailable(
                                        "mozui-native",
                                        "No native iOS checkbox is exposed in mozui-native yet.",
                                    )),
                            )
                            .child(
                                showcase_card("Radio", "Component radio with current iOS native gap")
                                    .child(
                                        showcase_row("mozui-components").child(
                                            Radio::new("ios-demo-radio")
                                                .label("Radio")
                                                .checked(self.component_radio_on)
                                                .on_click(cx.listener(Self::set_component_radio)),
                                        ),
                                    )
                                    .child(native_unavailable(
                                        "mozui-native",
                                        "UIKit has no built-in radio control and mozui-native does not have NativeRadio yet.",
                                    )),
                            )
                            .child(
                                showcase_card("Switch", "Component switch above, native UISwitch below")
                                    .child(
                                        showcase_row("mozui-components").child(
                                            ComponentSwitch::new("ios-demo-components-switch")
                                                .checked(self.component_switch_on)
                                                .color(hsla(0.12, 0.72, 0.60, 1.0))
                                                .on_click(cx.listener(Self::set_component_switch)),
                                        ),
                                    )
                                    .child(
                                        showcase_row("mozui-native").child(
                                            div()
                                                .h(px(36.0))
                                                .flex()
                                                .items_center()
                                                .child(
                                                    NativeSwitch::new("ios-demo-native-switch")
                                                        .is_on(true),
                                                ),
                                        ),
                                    ),
                            )
                            .child(
                                showcase_card("Slider", "Component slider above, native UISlider below")
                                    .child(
                                        showcase_row("mozui-components")
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(hsla(0.0, 0.0, 1.0, 0.62))
                                                    .child(format!("Value: {:.0}", slider_value)),
                                            )
                                            .child(Slider::new(&self.slider_state).horizontal()),
                                    )
                                    .child(
                                        showcase_row("mozui-native").child(
                                            div()
                                                .h(px(36.0))
                                                .flex()
                                                .items_center()
                                                .child(
                                                    NativeSlider::new("ios-demo-native-slider")
                                                        .range(0.0, 100.0)
                                                        .value(slider_value.into()),
                                                ),
                                        ),
                                    ),
                            )
                            .child(
                                showcase_card("Progress", "Component progress above, native UIProgressView below")
                                    .child(
                                        showcase_row("mozui-components").child(
                                            Progress::new("ios-demo-progress")
                                                .with_size(mozui_components::Size::Small)
                                                .value(slider_value),
                                        ),
                                    )
                                    .child(
                                        showcase_row("mozui-native").child(
                                            div()
                                                .h(px(12.0))
                                                .flex()
                                                .items_center()
                                                .child(
                                                    NativeProgress::new("ios-demo-native-progress")
                                                        .range(0.0, 100.0)
                                                        .value(slider_value.into()),
                                                ),
                                        ),
                                    ),
                            )
                            .child(
                                showcase_card("Input", "Component input with current iOS native gap")
                                    .child(
                                        showcase_row("mozui-components").child(
                                            Input::new(&self.input_state)
                                                .with_size(mozui_components::Size::Small),
                                        ),
                                    )
                                    .child(native_unavailable(
                                        "mozui-native",
                                        "NativeTextField is still macOS-only in mozui-native. The iOS text bridge currently powers the components input instead.",
                                    )),
                            )
                            .child(
                                showcase_card("Status", "Current iOS-native coverage in mozui-native")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.62))
                                            .child(
                                                "iOS-native showcase coverage now includes UIButton, UISlider, UIProgressView, and UISwitch.",
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(hsla(0.0, 0.0, 1.0, 0.48))
                                            .child(
                                                "Next native ports should be text field, picker/select, stepper, and any custom radio behavior we want to define.",
                                            ),
                                    ),
                            ),
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
                .open_window(options, |window, cx| {
                    let demo_root = cx.new(|cx| DemoRoot::new(window, cx));
                    cx.new(|cx| Root::new(demo_root, window, cx))
                })
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

    fn handle_scroll(&self, position: Point<Pixels>, delta: Point<Pixels>, phase: TouchPhase) {
        let Some(handle) = *self.shared.primary_window.borrow() else {
            return;
        };

        let _ = self
            .platform
            .dispatch_scroll(handle, position, delta, phase);
    }

    fn insert_text(&self, text: &str) -> bool {
        let Some(handle) = *self.shared.primary_window.borrow() else {
            return false;
        };
        self.platform.insert_text(handle, text)
    }

    fn delete_backward(&self) -> bool {
        let Some(handle) = *self.shared.primary_window.borrow() else {
            return false;
        };
        self.platform.delete_backward(handle)
    }

    fn accepts_text_input(&self) -> bool {
        let Some(handle) = *self.shared.primary_window.borrow() else {
            return false;
        };
        self.platform.accepts_text_input(handle)
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
pub unsafe extern "C" fn mozui_ios_demo_handle_scroll(
    demo: *mut MozuiIosDemo,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    phase: i32,
) {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return;
    };

    let phase = match phase {
        0 => TouchPhase::Started,
        2 => TouchPhase::Ended,
        _ => TouchPhase::Moved,
    };

    demo.handle_scroll(Point::new(px(x), px(y)), Point::new(px(dx), px(dy)), phase);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_insert_text(
    demo: *mut MozuiIosDemo,
    text: *const c_char,
) -> bool {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return false;
    };
    let Some(text) = NonNull::new(text.cast_mut()) else {
        return false;
    };
    let Ok(text) = unsafe { CStr::from_ptr(text.as_ptr()) }.to_str() else {
        return false;
    };
    demo.insert_text(text)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_delete_backward(demo: *mut MozuiIosDemo) -> bool {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return false;
    };
    demo.delete_backward()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mozui_ios_demo_accepts_text_input(demo: *mut MozuiIosDemo) -> bool {
    let Some(demo) = (unsafe { demo_from_ptr(demo) }) else {
        return false;
    };
    demo.accepts_text_input()
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
