use super::{
    IosDispatcher, IosDisplay, IosDisplayMetrics, IosKeyboardLayout, IosTouchPhase, IosWindow,
    IosWindowStateRef, IosWindowStateWeak, dispatch_active_status_change, dispatch_display_metrics,
    dispatch_request_frame, dispatch_touch_input, make_text_system,
};
use crate::{
    Action, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, DispatchEventResult,
    DummyKeyboardMapper, ForegroundExecutor, Keymap, Menu, MenuItem, OwnedMenu, PathPromptOptions,
    Pixels, Platform, PlatformDisplay, PlatformKeyboardLayout, PlatformKeyboardMapper,
    PlatformTextSystem, PlatformWindow, Point, RequestFrameOptions, Task, ThermalState,
    WindowAppearance, WindowParams,
};
use anyhow::{Result, anyhow};
use futures::channel::oneshot;
use parking_lot::Mutex;
use std::{
    cell::RefCell,
    ffi::c_void,
    path::{Path, PathBuf},
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
};

pub struct IosPlatform {
    _headless: bool,
    background_executor: BackgroundExecutor,
    foreground_executor: ForegroundExecutor,
    text_system: Arc<dyn PlatformTextSystem>,
    active_display: Rc<IosDisplay>,
    inner: Rc<IosPlatformInner>,
}

pub(crate) struct IosPlatformInner {
    state: Mutex<IosPlatformState>,
    windows: RefCell<Vec<IosWindowEntry>>,
    callbacks: RefCell<IosPlatformCallbacks>,
}

struct IosWindowEntry {
    handle: AnyWindowHandle,
    state: IosWindowStateWeak,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IosLifecycleState {
    Initialized,
    Launching,
    Active,
    Background,
}

struct IosPlatformState {
    launched: bool,
    lifecycle: IosLifecycleState,
    appearance: WindowAppearance,
    active_window: Option<AnyWindowHandle>,
    window_stack: Vec<AnyWindowHandle>,
    clipboard: Option<ClipboardItem>,
    thermal_state: ThermalState,
}

#[derive(Default)]
struct IosPlatformCallbacks {
    open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    quit: Option<Box<dyn FnMut()>>,
    reopen: Option<Box<dyn FnMut()>>,
    app_menu_action: Option<Box<dyn FnMut(&dyn Action)>>,
    will_open_app_menu: Option<Box<dyn FnMut()>>,
    validate_app_menu_command: Option<Box<dyn FnMut(&dyn Action) -> bool>>,
    keyboard_layout_change: Option<Box<dyn FnMut()>>,
    thermal_state_change: Option<Box<dyn FnMut()>>,
}

impl IosPlatformInner {
    fn new() -> Self {
        Self {
            state: Mutex::new(IosPlatformState {
                launched: false,
                lifecycle: IosLifecycleState::Initialized,
                appearance: WindowAppearance::Light,
                active_window: None,
                window_stack: Vec::new(),
                clipboard: None,
                thermal_state: ThermalState::Nominal,
            }),
            windows: RefCell::new(Vec::new()),
            callbacks: RefCell::new(IosPlatformCallbacks::default()),
        }
    }

    fn mark_launched(&self) -> bool {
        let mut state = self.state.lock();
        let was_launched = state.launched;
        state.launched = true;
        state.lifecycle = IosLifecycleState::Launching;
        was_launched
    }

    fn set_lifecycle_state(&self, lifecycle: IosLifecycleState) {
        self.state.lock().lifecycle = lifecycle;
    }

    #[allow(dead_code)]
    fn set_window_appearance(&self, appearance: WindowAppearance) {
        self.state.lock().appearance = appearance;
    }

    fn window_appearance(&self) -> WindowAppearance {
        self.state.lock().appearance
    }

    pub(crate) fn active_window(&self) -> Option<AnyWindowHandle> {
        self.state.lock().active_window
    }

    fn window_stack(&self) -> Vec<AnyWindowHandle> {
        self.state.lock().window_stack.clone()
    }

    fn register_window(&self, handle: AnyWindowHandle, window: IosWindowStateWeak) {
        self.windows
            .borrow_mut()
            .retain(|entry| entry.state.upgrade().is_some());
        self.windows.borrow_mut().push(IosWindowEntry {
            handle,
            state: window,
        });
    }

    fn live_window_states(&self) -> Vec<IosWindowStateRef> {
        let mut windows = self.windows.borrow_mut();
        windows.retain(|entry| entry.state.upgrade().is_some());
        windows
            .iter()
            .filter_map(|entry| entry.state.upgrade())
            .collect()
    }

    #[allow(dead_code)]
    fn find_window_state(&self, handle: AnyWindowHandle) -> Option<IosWindowStateRef> {
        self.live_window_states().into_iter().find(|window| {
            let state = window.lock();
            state.handle() == handle
        })
    }

    fn sync_active_window_callbacks(
        &self,
        previous: Option<AnyWindowHandle>,
        next: Option<AnyWindowHandle>,
    ) {
        if previous == next {
            return;
        }

        for window in self.live_window_states() {
            let handle = window.lock().handle();
            if Some(handle) == previous {
                dispatch_active_status_change(&window, false);
            }
            if Some(handle) == next {
                dispatch_active_status_change(&window, true);
            }
        }
    }

    fn track_window(&self, handle: AnyWindowHandle, window: IosWindowStateWeak) {
        self.register_window(handle, window);

        let (previous, next) = {
            let mut state = self.state.lock();
            if !state.window_stack.contains(&handle) {
                state.window_stack.push(handle);
            }
            let previous = state.active_window;
            state.active_window = Some(handle);
            (previous, state.active_window)
        };

        self.sync_active_window_callbacks(previous, next);
    }

    pub(crate) fn activate_window(&self, handle: AnyWindowHandle) {
        let (previous, next) = {
            let mut state = self.state.lock();
            state.window_stack.retain(|existing| *existing != handle);
            state.window_stack.push(handle);
            let previous = state.active_window;
            state.active_window = Some(handle);
            (previous, state.active_window)
        };

        self.sync_active_window_callbacks(previous, next);
    }

    pub(crate) fn remove_window(&self, handle: AnyWindowHandle) {
        self.windows
            .borrow_mut()
            .retain(|entry| entry.handle != handle && entry.state.upgrade().is_some());

        let (previous, next) = {
            let mut state = self.state.lock();
            let previous = state.active_window;
            state.window_stack.retain(|existing| *existing != handle);
            if state.active_window == Some(handle) {
                state.active_window = state.window_stack.last().copied();
            }
            (previous, state.active_window)
        };

        self.sync_active_window_callbacks(previous, next);
    }

    fn read_clipboard(&self) -> Option<ClipboardItem> {
        self.state.lock().clipboard.clone()
    }

    fn write_clipboard(&self, item: ClipboardItem) {
        self.state.lock().clipboard = Some(item);
    }

    fn thermal_state(&self) -> ThermalState {
        self.state.lock().thermal_state
    }

    #[allow(dead_code)]
    pub(crate) fn set_thermal_state(&self, thermal_state: ThermalState) {
        self.state.lock().thermal_state = thermal_state;
        if let Some(callback) = self.callbacks.borrow_mut().thermal_state_change.as_mut() {
            callback();
        }
    }

    #[allow(dead_code)]
    pub(crate) fn emit_open_urls(&self, urls: Vec<String>) {
        if let Some(callback) = self.callbacks.borrow_mut().open_urls.as_mut() {
            callback(urls);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn emit_reopen(&self) {
        if let Some(callback) = self.callbacks.borrow_mut().reopen.as_mut() {
            callback();
        }
    }

    fn request_quit(&self) {
        if let Some(callback) = self.callbacks.borrow_mut().quit.as_mut() {
            callback();
        }
    }
}

impl IosPlatform {
    pub fn new(headless: bool) -> Self {
        let dispatcher = Arc::new(IosDispatcher::new());
        let background_executor = BackgroundExecutor::new(dispatcher.clone());
        let foreground_executor = ForegroundExecutor::new(dispatcher);
        let text_system = make_text_system();
        let active_display = Rc::new(IosDisplay::new());
        let inner = Rc::new(IosPlatformInner::new());

        Self {
            _headless: headless,
            background_executor,
            foreground_executor,
            text_system,
            active_display,
            inner,
        }
    }

    pub fn enter_background(&self) {
        self.inner
            .set_lifecycle_state(IosLifecycleState::Background);
        for window in self.inner.live_window_states() {
            window.lock().suspend_surface();
        }
    }

    pub fn enter_foreground(&self) {
        self.inner.set_lifecycle_state(IosLifecycleState::Active);
        for window in self.inner.live_window_states() {
            let mut state = window.lock();
            if let Err(error) = state.resume_surface() {
                log::error!("failed to resume iOS window surface after foregrounding: {error:#}");
            }
        }
        for window in self.inner.live_window_states() {
            dispatch_request_frame(
                &window,
                RequestFrameOptions {
                    require_presentation: true,
                    force_render: true,
                },
            );
        }
    }

    pub fn update_display_metrics(
        &self,
        metrics: IosDisplayMetrics,
        appearance: Option<WindowAppearance>,
    ) {
        self.active_display.update_metrics(metrics);
        let appearance = appearance.unwrap_or_else(|| self.inner.window_appearance());
        self.inner.set_window_appearance(appearance);

        for window in self.inner.live_window_states() {
            dispatch_display_metrics(&window, metrics, Some(appearance));
        }
    }

    pub fn attach_window_surface(
        &self,
        handle: AnyWindowHandle,
        ui_view: NonNull<c_void>,
        ui_view_controller: Option<NonNull<c_void>>,
    ) -> Result<()> {
        let window = self
            .inner
            .find_window_state(handle)
            .ok_or_else(|| anyhow!("unknown iOS window handle: {handle:?}"))?;
        window.lock().attach_surface(ui_view, ui_view_controller)?;
        dispatch_request_frame(
            &window,
            RequestFrameOptions {
                require_presentation: true,
                force_render: true,
            },
        );
        Ok(())
    }

    pub fn detach_window_surface(&self, handle: AnyWindowHandle) {
        if let Some(window) = self.inner.find_window_state(handle) {
            window.lock().detach_surface();
        }
    }

    pub fn attach_window_surface_raw(
        &self,
        handle: AnyWindowHandle,
        ui_view: *mut c_void,
        ui_view_controller: *mut c_void,
    ) -> Result<()> {
        let ui_view = NonNull::new(ui_view)
            .ok_or_else(|| anyhow!("attempted to attach a null UIView pointer"))?;
        let ui_view_controller = NonNull::new(ui_view_controller);
        self.attach_window_surface(handle, ui_view, ui_view_controller)
    }

    pub fn dispatch_touch(
        &self,
        handle: AnyWindowHandle,
        phase: IosTouchPhase,
        position: Point<Pixels>,
    ) -> Option<DispatchEventResult> {
        let window = self.inner.find_window_state(handle)?;
        dispatch_touch_input(&window, phase, position)
    }

    pub fn request_frame(&self, handle: AnyWindowHandle, options: RequestFrameOptions) {
        if let Some(window) = self.inner.find_window_state(handle) {
            dispatch_request_frame(&window, options);
        }
    }
}

impl Platform for IosPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        self.background_executor.clone()
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        self.foreground_executor.clone()
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.text_system.clone()
    }

    fn run(&self, on_finish_launching: Box<dyn 'static + FnOnce()>) {
        if self.inner.mark_launched() {
            log::warn!("IosPlatform::run called more than once; ignoring duplicate launch");
            return;
        }

        let inner = self.inner.clone();
        self.foreground_executor
            .spawn(async move {
                inner.set_lifecycle_state(IosLifecycleState::Active);
                on_finish_launching();
            })
            .detach();
    }

    fn quit(&self) {
        self.inner.request_quit();
    }

    fn restart(&self, _binary_path: Option<PathBuf>) {}

    fn activate(&self, _ignoring_other_apps: bool) {
        self.enter_foreground();
    }

    fn hide(&self) {
        self.enter_background();
    }

    fn hide_other_apps(&self) {}

    fn unhide_other_apps(&self) {}

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        vec![self.active_display.clone()]
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.active_display.clone())
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        self.inner.active_window()
    }

    fn window_stack(&self) -> Option<Vec<AnyWindowHandle>> {
        Some(self.inner.window_stack())
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        params: WindowParams,
    ) -> anyhow::Result<Box<dyn PlatformWindow>> {
        let window = IosWindow::new(
            handle,
            params,
            self.active_display.clone(),
            Rc::downgrade(&self.inner),
        );
        self.inner.track_window(handle, window.weak_state());
        Ok(Box::new(window))
    }

    fn window_appearance(&self) -> WindowAppearance {
        self.inner.window_appearance()
    }

    fn open_url(&self, url: &str) {
        log::debug!("iOS open_url requested before UIApplication bridge is implemented: {url}");
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.inner.callbacks.borrow_mut().open_urls = Some(callback);
    }

    fn register_url_scheme(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Err(anyhow!(
            "runtime URL scheme registration is not supported on iOS"
        )))
    }

    fn prompt_for_paths(
        &self,
        _options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (tx, rx) = oneshot::channel();
        tx.send(Err(anyhow!("path prompts are not implemented for iOS yet")))
            .ok();
        rx
    }

    fn prompt_for_new_path(
        &self,
        _directory: &Path,
        _suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let (tx, rx) = oneshot::channel();
        tx.send(Err(anyhow!(
            "new-path prompts are not implemented for iOS yet"
        )))
        .ok();
        rx
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }

    fn reveal_path(&self, _path: &Path) {}

    fn open_with_system(&self, _path: &Path) {}

    fn on_quit(&self, callback: Box<dyn FnMut()>) {
        self.inner.callbacks.borrow_mut().quit = Some(callback);
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.inner.callbacks.borrow_mut().reopen = Some(callback);
    }

    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {}

    fn get_menus(&self) -> Option<Vec<OwnedMenu>> {
        None
    }

    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {}

    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn Action)>) {
        self.inner.callbacks.borrow_mut().app_menu_action = Some(callback);
    }

    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        self.inner.callbacks.borrow_mut().will_open_app_menu = Some(callback);
    }

    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        self.inner.callbacks.borrow_mut().validate_app_menu_command = Some(callback);
    }

    fn thermal_state(&self) -> ThermalState {
        self.inner.thermal_state()
    }

    fn on_thermal_state_change(&self, callback: Box<dyn FnMut()>) {
        self.inner.callbacks.borrow_mut().thermal_state_change = Some(callback);
    }

    fn compositor_name(&self) -> &'static str {
        "UIKit"
    }

    fn app_path(&self) -> Result<PathBuf> {
        std::env::current_exe()
            .and_then(|path| {
                path.parent().map(PathBuf::from).ok_or_else(|| {
                    std::io::Error::other("current executable has no parent directory")
                })
            })
            .map_err(Into::into)
    }

    fn path_for_auxiliary_executable(&self, _name: &str) -> Result<PathBuf> {
        Err(anyhow!(
            "auxiliary executable lookup is not implemented for iOS"
        ))
    }

    fn set_cursor_style(&self, _style: CursorStyle) {}

    fn should_auto_hide_scrollbars(&self) -> bool {
        true
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        self.inner.read_clipboard()
    }

    fn write_to_clipboard(&self, item: ClipboardItem) {
        self.inner.write_clipboard(item);
    }

    fn write_credentials(&self, _url: &str, _username: &str, _password: &[u8]) -> Task<Result<()>> {
        Task::ready(Err(anyhow!(
            "iOS credentials support is not implemented yet"
        )))
    }

    fn read_credentials(&self, _url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        Task::ready(Err(anyhow!(
            "iOS credentials support is not implemented yet"
        )))
    }

    fn delete_credentials(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Err(anyhow!(
            "iOS credentials support is not implemented yet"
        )))
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(IosKeyboardLayout)
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(DummyKeyboardMapper)
    }

    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        self.inner.callbacks.borrow_mut().keyboard_layout_change = Some(callback);
    }
}
