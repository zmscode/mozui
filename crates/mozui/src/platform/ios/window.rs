use super::{IosDisplay, IosDisplayMetrics, IosMetalRenderer, IosRawWindow};
use crate::platform::ios::platform::IosPlatformInner;
use crate::{
    AnyWindowHandle, AtlasKey, AtlasTile, Bounds, Capslock, DevicePixels, DispatchEventResult,
    GpuSpecs, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    PlatformAtlas, PlatformDisplay, PlatformInput, PlatformInputHandler, PlatformWindow, Point,
    PromptButton, PromptLevel, RequestFrameOptions, Scene, Size, WindowAppearance,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowParams,
};
use parking_lot::Mutex;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, UiKitWindowHandle,
};
use std::{
    borrow::Cow,
    ffi::c_void,
    ptr::NonNull,
    rc::{Rc, Weak},
    sync::{Arc, Weak as SyncWeak},
};

pub(crate) type IosWindowStateRef = Arc<Mutex<IosWindowState>>;
pub(crate) type IosWindowStateWeak = SyncWeak<Mutex<IosWindowState>>;

pub(crate) struct IosWindowState {
    handle: AnyWindowHandle,
    platform: Weak<IosPlatformInner>,
    bounds: Bounds<Pixels>,
    display: Rc<IosDisplay>,
    title: Option<String>,
    sprite_atlas: Arc<IosAtlas>,
    input_handler: Option<PlatformInputHandler>,
    input_callback: Option<Box<dyn FnMut(PlatformInput) -> DispatchEventResult>>,
    active_status_change_callback: Option<Box<dyn FnMut(bool)>>,
    hover_status_change_callback: Option<Box<dyn FnMut(bool)>>,
    resize_callback: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    moved_callback: Option<Box<dyn FnMut()>>,
    request_frame_callback: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    should_close_handler: Option<Box<dyn FnMut() -> bool>>,
    hit_test_window_control_callback: Option<Box<dyn FnMut() -> Option<WindowControlArea>>>,
    appearance_changed_callback: Option<Box<dyn FnMut()>>,
    close_callback: Option<Box<dyn FnOnce()>>,
    background_appearance: WindowBackgroundAppearance,
    appearance: WindowAppearance,
    is_active: bool,
    is_fullscreen: bool,
    scale_factor: f32,
    mouse_position: Point<Pixels>,
    pressed_button: Option<MouseButton>,
    click_count: usize,
    raw_window: Option<IosRawWindow>,
    renderer: Option<IosMetalRenderer>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IosTouchPhase {
    Began,
    Moved,
    Ended,
    Cancelled,
}

impl IosWindowState {
    pub(crate) fn handle(&self) -> AnyWindowHandle {
        self.handle
    }

    fn new(
        handle: AnyWindowHandle,
        params: WindowParams,
        display: Rc<IosDisplay>,
        platform: Weak<IosPlatformInner>,
    ) -> Self {
        let metrics = display.metrics();
        let sprite_atlas = Arc::new(IosAtlas::new());

        Self {
            handle,
            platform,
            bounds: params.bounds,
            display,
            title: params
                .titlebar
                .as_ref()
                .and_then(|titlebar| titlebar.title.as_ref().map(|title| title.to_string())),
            sprite_atlas,
            input_handler: None,
            input_callback: None,
            active_status_change_callback: None,
            hover_status_change_callback: None,
            resize_callback: None,
            moved_callback: None,
            request_frame_callback: None,
            should_close_handler: None,
            hit_test_window_control_callback: None,
            appearance_changed_callback: None,
            close_callback: None,
            background_appearance: WindowBackgroundAppearance::Opaque,
            appearance: WindowAppearance::Light,
            is_active: false,
            is_fullscreen: false,
            scale_factor: metrics.scale_factor,
            mouse_position: Point::default(),
            pressed_button: None,
            click_count: 0,
            raw_window: None,
            renderer: None,
        }
    }

    #[allow(dead_code)]
    fn apply_display_metrics(
        &mut self,
        metrics: IosDisplayMetrics,
        appearance: Option<WindowAppearance>,
    ) -> DisplayMetricChanges {
        self.bounds.origin = metrics.bounds.origin;
        self.bounds.size = metrics.bounds.size;
        let size = self.bounds.size;
        self.scale_factor = metrics.scale_factor;
        if let Some(appearance) = appearance {
            self.appearance = appearance;
        }

        let device_size = self.device_size();
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.update_drawable_size(device_size);
        }

        DisplayMetricChanges {
            size,
            scale_factor: self.scale_factor,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn attach_surface(
        &mut self,
        ui_view: NonNull<c_void>,
        ui_view_controller: Option<NonNull<c_void>>,
    ) -> anyhow::Result<()> {
        let raw_window = IosRawWindow::new(ui_view, ui_view_controller);
        let device_size = self.device_size();
        let background_appearance = self.background_appearance;

        if let Some(renderer) = self.renderer.as_mut() {
            renderer.replace_surface(raw_window, device_size, background_appearance)?;
        } else {
            self.renderer = Some(IosMetalRenderer::new(
                raw_window,
                device_size,
                background_appearance,
            )?);
        }

        self.raw_window = Some(raw_window);
        if let Some(renderer) = self.renderer.as_ref() {
            self.sprite_atlas
                .set_delegate(renderer.sprite_atlas() as Arc<dyn PlatformAtlas>);
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn detach_surface(&mut self) {
        self.raw_window = None;
        self.sprite_atlas.clear_delegate();
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.suspend_surface();
        }
    }

    pub(crate) fn suspend_surface(&mut self) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.suspend_surface();
        }
    }

    pub(crate) fn resume_surface(&mut self) -> anyhow::Result<()> {
        let Some(raw_window) = self.raw_window else {
            return Ok(());
        };

        let device_size = self.device_size();
        let background_appearance = self.background_appearance;

        if let Some(renderer) = self.renderer.as_mut() {
            renderer.replace_surface(raw_window, device_size, background_appearance)?;
        } else {
            self.renderer = Some(IosMetalRenderer::new(
                raw_window,
                device_size,
                background_appearance,
            )?);
        }

        if let Some(renderer) = self.renderer.as_ref() {
            self.sprite_atlas
                .set_delegate(renderer.sprite_atlas() as Arc<dyn PlatformAtlas>);
        }

        Ok(())
    }

    fn device_size(&self) -> Size<DevicePixels> {
        Size {
            width: DevicePixels((self.bounds.size.width.0 * self.scale_factor).round() as i32),
            height: DevicePixels((self.bounds.size.height.0 * self.scale_factor).round() as i32),
        }
    }
}

pub(crate) struct IosWindow(IosWindowStateRef);

struct DisplayMetricChanges {
    size: Size<Pixels>,
    scale_factor: f32,
}

pub(crate) fn dispatch_request_frame(window: &IosWindowStateRef, options: RequestFrameOptions) {
    let mut callback = {
        let mut state = window.lock();
        state.request_frame_callback.take()
    };

    if let Some(callback) = callback.as_mut() {
        callback(options);
    }

    if let Some(callback) = callback {
        let mut state = window.lock();
        if state.request_frame_callback.is_none() {
            state.request_frame_callback = Some(callback);
        }
    }
}

pub(crate) fn dispatch_active_status_change(window: &IosWindowStateRef, is_active: bool) {
    let mut callback = {
        let mut state = window.lock();
        if state.is_active == is_active {
            return;
        }
        state.is_active = is_active;
        state.active_status_change_callback.take()
    };

    if let Some(callback) = callback.as_mut() {
        callback(is_active);
    }

    if let Some(callback) = callback {
        let mut state = window.lock();
        if state.active_status_change_callback.is_none() {
            state.active_status_change_callback = Some(callback);
        }
    }
}

pub(crate) fn dispatch_display_metrics(
    window: &IosWindowStateRef,
    metrics: IosDisplayMetrics,
    appearance: Option<WindowAppearance>,
) {
    let (changes, mut moved_callback, mut resize_callback, mut appearance_callback) = {
        let mut state = window.lock();
        let previous_origin = state.bounds.origin;
        let previous_size = state.bounds.size;
        let previous_scale = state.scale_factor;
        let previous_appearance = state.appearance;

        let changes = state.apply_display_metrics(metrics, appearance);
        let moved = previous_origin != state.bounds.origin;
        let resized = previous_size != state.bounds.size || previous_scale != state.scale_factor;
        let appearance_changed = previous_appearance != state.appearance;

        (
            DisplayMetricChanges {
                size: changes.size,
                scale_factor: changes.scale_factor,
            },
            if moved {
                state.moved_callback.take()
            } else {
                None
            },
            if resized {
                state.resize_callback.take()
            } else {
                None
            },
            if appearance_changed {
                state.appearance_changed_callback.take()
            } else {
                None
            },
        )
    };

    if let Some(callback) = moved_callback.as_mut() {
        callback();
    }

    if let Some(callback) = resize_callback.as_mut() {
        callback(changes.size, changes.scale_factor);
    }

    if let Some(callback) = appearance_callback.as_mut() {
        callback();
    }

    {
        let mut state = window.lock();
        if state.moved_callback.is_none() {
            state.moved_callback = moved_callback;
        }
        if state.resize_callback.is_none() {
            state.resize_callback = resize_callback;
        }
        if state.appearance_changed_callback.is_none() {
            state.appearance_changed_callback = appearance_callback;
        }
    }

    dispatch_request_frame(
        window,
        RequestFrameOptions {
            require_presentation: true,
            force_render: true,
        },
    );
}

pub(crate) fn dispatch_platform_input(
    window: &IosWindowStateRef,
    input: PlatformInput,
) -> Option<DispatchEventResult> {
    let mut callback = {
        let mut state = window.lock();
        match &input {
            PlatformInput::MouseMove(event) => {
                state.mouse_position = event.position;
                state.pressed_button = event.pressed_button;
            }
            PlatformInput::MouseDown(event) => {
                state.mouse_position = event.position;
                state.pressed_button = Some(event.button);
                state.click_count = event.click_count.max(1);
            }
            PlatformInput::MouseUp(event) => {
                state.mouse_position = event.position;
                state.pressed_button = None;
                state.click_count = event.click_count.max(1);
            }
            _ => {}
        }
        state.input_callback.take()
    };

    let result = callback.as_mut().map(|callback| callback(input));

    if let Some(callback) = callback {
        let mut state = window.lock();
        if state.input_callback.is_none() {
            state.input_callback = Some(callback);
        }
    }

    dispatch_request_frame(
        window,
        RequestFrameOptions {
            require_presentation: true,
            force_render: false,
        },
    );

    result
}

pub(crate) fn dispatch_touch_input(
    window: &IosWindowStateRef,
    phase: IosTouchPhase,
    position: Point<Pixels>,
) -> Option<DispatchEventResult> {
    let (modifiers, click_count) = {
        let mut state = window.lock();
        state.mouse_position = position;
        match phase {
            IosTouchPhase::Began => {
                state.click_count = 1;
                state.pressed_button = Some(MouseButton::Left);
            }
            IosTouchPhase::Moved => {
                state.pressed_button = Some(MouseButton::Left);
            }
            IosTouchPhase::Ended | IosTouchPhase::Cancelled => {
                state.pressed_button = None;
            }
        }
        (Modifiers::default(), state.click_count.max(1))
    };

    let input = match phase {
        IosTouchPhase::Began => PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position,
            modifiers,
            click_count,
            first_mouse: false,
        }),
        IosTouchPhase::Moved => PlatformInput::MouseMove(MouseMoveEvent {
            position,
            pressed_button: Some(MouseButton::Left),
            modifiers,
        }),
        IosTouchPhase::Ended | IosTouchPhase::Cancelled => PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position,
            modifiers,
            click_count,
        }),
    };

    dispatch_platform_input(window, input)
}

impl IosWindow {
    pub(crate) fn new(
        handle: AnyWindowHandle,
        params: WindowParams,
        display: Rc<IosDisplay>,
        platform: Weak<IosPlatformInner>,
    ) -> Self {
        Self(Arc::new(Mutex::new(IosWindowState::new(
            handle, params, display, platform,
        ))))
    }

    pub(crate) fn weak_state(&self) -> IosWindowStateWeak {
        Arc::downgrade(&self.0)
    }
}

impl Drop for IosWindow {
    fn drop(&mut self) {
        let mut state = self.0.lock();
        if let Some(renderer) = state.renderer.as_mut() {
            renderer.destroy();
        }
        if let Some(platform) = state.platform.upgrade() {
            platform.remove_window(state.handle);
        }
        state.close_callback.take();
    }
}

impl HasWindowHandle for IosWindow {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let state = self.0.lock();
        let raw_window = state.raw_window.ok_or(HandleError::Unavailable)?;
        let mut handle = UiKitWindowHandle::new(raw_window.ui_view());
        handle.ui_view_controller = raw_window.ui_view_controller();
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(handle.into()) })
    }
}

impl HasDisplayHandle for IosWindow {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(DisplayHandle::uikit())
    }
}

impl PlatformWindow for IosWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.0.lock().bounds
    }

    fn is_maximized(&self) -> bool {
        false
    }

    fn window_bounds(&self) -> WindowBounds {
        WindowBounds::Windowed(self.bounds())
    }

    fn content_size(&self) -> Size<Pixels> {
        self.bounds().size
    }

    fn resize(&mut self, size: Size<Pixels>) {
        let (scale_factor, mut resize_callback) = {
            let mut state = self.0.lock();
            state.bounds.size = size;
            let scale_factor = state.scale_factor;
            let device_size = state.device_size();
            if let Some(renderer) = state.renderer.as_mut() {
                renderer.update_drawable_size(device_size);
            }
            (scale_factor, state.resize_callback.take())
        };

        if let Some(callback) = resize_callback.as_mut() {
            callback(size, scale_factor);
        }

        {
            let mut state = self.0.lock();
            if state.resize_callback.is_none() {
                state.resize_callback = resize_callback;
            }
        }

        dispatch_request_frame(
            &self.0,
            RequestFrameOptions {
                require_presentation: true,
                force_render: false,
            },
        );
    }

    fn scale_factor(&self) -> f32 {
        self.0.lock().scale_factor
    }

    fn appearance(&self) -> WindowAppearance {
        self.0.lock().appearance
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.0.lock().display.clone())
    }

    fn mouse_position(&self) -> Point<Pixels> {
        self.0.lock().mouse_position
    }

    fn modifiers(&self) -> Modifiers {
        Modifiers::default()
    }

    fn capslock(&self) -> Capslock {
        Capslock::default()
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.0.lock().input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.0.lock().input_handler.take()
    }

    fn prompt(
        &self,
        _level: PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<futures::channel::oneshot::Receiver<usize>> {
        None
    }

    fn activate(&self) {
        let state = self.0.lock();
        if let Some(platform) = state.platform.upgrade() {
            platform.activate_window(state.handle);
        }
    }

    fn is_active(&self) -> bool {
        self.0.lock().is_active
    }

    fn is_hovered(&self) -> bool {
        false
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        self.0.lock().background_appearance
    }

    fn set_title(&mut self, title: &str) {
        self.0.lock().title = Some(title.to_owned());
    }

    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance) {
        let mut state = self.0.lock();
        state.background_appearance = background_appearance;
        if let Some(renderer) = state.renderer.as_mut() {
            renderer.update_transparency(background_appearance);
        }
    }

    fn minimize(&self) {}

    fn zoom(&self) {}

    fn toggle_fullscreen(&self) {
        {
            let mut state = self.0.lock();
            state.is_fullscreen = !state.is_fullscreen;
        }
        dispatch_request_frame(
            &self.0,
            RequestFrameOptions {
                require_presentation: true,
                force_render: true,
            },
        );
    }

    fn is_fullscreen(&self) -> bool {
        self.0.lock().is_fullscreen
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.0.lock().request_frame_callback = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> DispatchEventResult>) {
        self.0.lock().input_callback = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.lock().active_status_change_callback = Some(callback);
    }

    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.lock().hover_status_change_callback = Some(callback);
    }

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.0.lock().resize_callback = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().moved_callback = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.lock().should_close_handler = Some(callback);
    }

    fn on_hit_test_window_control(&self, callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {
        self.0.lock().hit_test_window_control_callback = Some(callback);
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.0.lock().close_callback = Some(callback);
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().appearance_changed_callback = Some(callback);
    }

    fn draw(&self, scene: &Scene) {
        if let Some(renderer) = self.0.lock().renderer.as_mut() {
            renderer.draw(scene);
        }
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.0.lock().sprite_atlas.clone() as Arc<dyn PlatformAtlas>
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        false
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        self.0
            .lock()
            .renderer
            .as_ref()
            .map(|renderer| renderer.gpu_specs())
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {}
}

pub(crate) struct IosAtlasState {
    delegate: Option<Arc<dyn PlatformAtlas>>,
}

pub(crate) struct IosAtlas(Mutex<IosAtlasState>);

impl IosAtlas {
    pub(crate) fn new() -> Self {
        Self(Mutex::new(IosAtlasState { delegate: None }))
    }

    fn set_delegate(&self, delegate: Arc<dyn PlatformAtlas>) {
        self.0.lock().delegate = Some(delegate);
    }

    fn clear_delegate(&self) {
        self.0.lock().delegate = None;
    }
}

impl PlatformAtlas for IosAtlas {
    fn get_or_insert_with<'a>(
        &self,
        key: &AtlasKey,
        build: &mut dyn FnMut() -> anyhow::Result<Option<(Size<DevicePixels>, Cow<'a, [u8]>)>>,
    ) -> anyhow::Result<Option<AtlasTile>> {
        let delegate = self.0.lock().delegate.clone();
        match delegate {
            Some(delegate) => delegate.get_or_insert_with(key, build),
            None => Ok(None),
        }
    }

    fn remove(&self, key: &AtlasKey) {
        let delegate = self.0.lock().delegate.clone();
        if let Some(delegate) = delegate {
            delegate.remove(key);
        }
    }
}
