mod assets;
mod builder;
mod ipc;
mod security;

pub use assets::AssetServer;
pub use builder::WebViewBuilder;
pub use ipc::{
    AsyncCommandConfig, AsyncCommandRegistry, CommandRegistry, CommandResult, IpcEnvelope,
    IpcError, IpcMessage, JsEmitEvent, NavigationBlockedEvent, NavigationEvent,
};
pub use security::{Capabilities, CommandAllowlist, MOZUI_ORIGIN, MOZUI_SCHEME, OriginAllowlist};

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::rc::Rc;

use wry::{
    Rect,
    dpi::{self, LogicalSize},
};

use mozui::{
    App, Bounds, ContentMask, DismissEvent, Element, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, GlobalElementId, InteractiveElement, IntoElement, LayoutId, MouseDownEvent,
    ParentElement as _, Pixels, Render, Size, Style, Styled as _, Window, canvas, div,
};

/// A webview based on wry WebView with typed IPC support.
pub struct WebView {
    focus_handle: FocusHandle,
    webview: Rc<wry::WebView>,
    visible: bool,
    bounds: Bounds<Pixels>,

    // IPC
    command_registry: CommandRegistry,
    ipc_queue: Rc<RefCell<VecDeque<IpcEnvelope>>>,

    // Navigation
    nav_queue: Rc<RefCell<VecDeque<String>>>,
    nav_blocked_queue: Rc<RefCell<VecDeque<String>>>,

    // Async commands
    async_registry: AsyncCommandRegistry,
    pending_async: HashMap<String, mozui::Task<()>>,
    async_config: AsyncCommandConfig,

    // Security
    capabilities: Capabilities,
}

impl Drop for WebView {
    fn drop(&mut self) {
        self.hide();
    }
}

impl WebView {
    /// Create a new WebView from a raw wry WebView (no IPC support).
    ///
    /// Prefer `WebViewBuilder` for full IPC and security support.
    pub fn new(webview: wry::WebView, _: &mut Window, cx: &mut App) -> Self {
        let _ = webview.set_bounds(Rect::default());

        Self {
            focus_handle: cx.focus_handle(),
            visible: true,
            bounds: Bounds::default(),
            webview: Rc::new(webview),
            command_registry: CommandRegistry::new(),
            ipc_queue: Rc::new(RefCell::new(VecDeque::new())),
            nav_queue: Rc::new(RefCell::new(VecDeque::new())),
            nav_blocked_queue: Rc::new(RefCell::new(VecDeque::new())),
            async_registry: AsyncCommandRegistry::new(),
            pending_async: HashMap::new(),
            async_config: AsyncCommandConfig::default(),
            capabilities: Capabilities::default(),
        }
    }

    /// Show the webview.
    pub fn show(&mut self) {
        let _ = self.webview.set_visible(true);
        self.visible = true;
    }

    /// Hide the webview.
    pub fn hide(&mut self) {
        _ = self.webview.focus_parent();
        _ = self.webview.set_visible(false);
        self.visible = false;
    }

    /// Get whether the webview is visible.
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Get the current bounds of the webview.
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Go back in the webview history.
    pub fn back(&mut self) -> anyhow::Result<()> {
        Ok(self.webview.evaluate_script("history.back();")?)
    }

    /// Go forward in the webview history.
    pub fn forward(&mut self) -> anyhow::Result<()> {
        Ok(self.webview.evaluate_script("history.forward();")?)
    }

    /// Reload the current page.
    pub fn reload(&mut self) -> anyhow::Result<()> {
        Ok(self.webview.evaluate_script("location.reload();")?)
    }

    /// Load a URL in the webview.
    pub fn load_url(&mut self, url: &str) {
        let _ = self.webview.load_url(url);
    }

    /// Returns the asset URL for a given path, correct for the current platform.
    pub fn asset_url(path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{MOZUI_ORIGIN}/{path}")
    }

    /// Push an event to the JS renderer.
    ///
    /// This calls `window.__mozui_dispatch()` with a validated JSON payload.
    /// Not gated by `script_execution` capability — only sends structured data.
    pub fn emit_to_js(&self, event: &str, payload: impl serde::Serialize) -> anyhow::Result<()> {
        let msg = serde_json::json!({
            "__mozui": true,
            "type": "event",
            "name": event,
            "payload": payload,
        });
        let json_str = serde_json::to_string(&msg)?;
        let safe = sanitize_for_evaluate(&json_str);
        Ok(self
            .webview
            .evaluate_script(&format!("window.__mozui_dispatch('{safe}')"))?)
    }

    /// Execute raw JS in the webview. Requires `script_execution` capability.
    pub fn evaluate_script(&self, script: &str) -> anyhow::Result<()> {
        if !self.capabilities.script_execution {
            return Err(anyhow::anyhow!("script_execution capability not enabled"));
        }
        Ok(self.webview.evaluate_script(script)?)
    }

    /// Get the raw wry webview.
    pub fn raw(&self) -> &wry::WebView {
        &self.webview
    }

    /// Process a single IPC envelope: look up command, run handler, send response.
    fn handle_ipc(&mut self, envelope: IpcEnvelope, cx: &mut mozui::Context<Self>) {
        match envelope.msg_type.as_str() {
            "invoke" => self.handle_invoke(envelope, cx),
            "emit" => self.handle_js_emit(envelope, cx),
            "cancel" => self.handle_cancel(envelope),
            _ => {}
        }
    }

    fn handle_invoke(&mut self, envelope: IpcEnvelope, cx: &mut mozui::Context<Self>) {
        let id = match &envelope.id {
            Some(id) => id.clone(),
            None => return,
        };
        let command = match &envelope.command {
            Some(cmd) => cmd.clone(),
            None => {
                self.send_error_response(&id, IpcError::unknown_command());
                return;
            }
        };

        // Both denied and unregistered return UNKNOWN_COMMAND (timing oracle prevention)
        let is_sync = self.command_registry.contains(&command);
        let is_async = self.async_registry.contains(&command);

        if !self.capabilities.commands.permits(&command) || (!is_sync && !is_async) {
            self.send_error_response(&id, IpcError::unknown_command());
            return;
        }

        let args = envelope.args.unwrap_or(serde_json::Value::Null);

        if is_sync {
            // Synchronous command path
            let result = {
                let handler = self.command_registry.get(&command);
                match handler {
                    Some(handler) => {
                        let handler_ptr = handler as *const ipc::CommandHandler;
                        let self_ptr = self as *mut WebView;
                        // SAFETY: The handler does not mutate the registry. We split
                        // the borrow so the handler can receive &mut WebView while
                        // we hold an immutable reference to the registry entry.
                        unsafe { (*handler_ptr)(args, &mut *self_ptr, &mut **cx) }
                    }
                    None => CommandResult::Immediate(Err(IpcError::unknown_command())),
                }
            };

            match result {
                CommandResult::Immediate(Ok(value)) => {
                    self.send_ok_response(&id, value);
                }
                CommandResult::Immediate(Err(error)) => {
                    self.send_error_response(&id, error);
                }
                CommandResult::Deferred(fut) => {
                    let task_key = format!("{command}:{id}");
                    let task = cx.spawn(async move |weak_entity, cx| {
                        let result = fut.await;
                        let _ = weak_entity.update(cx, |wv, _cx| match result {
                            Ok(value) => wv.send_ok_response(&id, value),
                            Err(error) => wv.send_error_response(&id, error),
                        });
                    });
                    self.pending_async.insert(task_key, task);
                }
            }
        } else {
            // Async command path
            self.handle_async_invoke(id, command, args, cx);
        }
    }

    fn handle_async_invoke(
        &mut self,
        id: String,
        command: String,
        args: serde_json::Value,
        cx: &mut mozui::Context<Self>,
    ) {
        // Global rate limit
        if self.pending_async.len() >= self.async_config.max_concurrent {
            self.send_error_response(
                &id,
                IpcError {
                    code: IpcError::RATE_LIMITED,
                    message: "too many concurrent async invocations".into(),
                },
            );
            return;
        }

        // Per-command rate limit
        if let Some(ref limits) = self.async_config.per_command_limit {
            if let Some(&limit) = limits.get(command.as_str()) {
                let count = self
                    .pending_async
                    .keys()
                    .filter(|k| k.starts_with(&format!("{command}:")))
                    .count();
                if count >= limit {
                    self.send_error_response(
                        &id,
                        IpcError {
                            code: IpcError::RATE_LIMITED,
                            message: format!("too many concurrent '{command}' invocations"),
                        },
                    );
                    return;
                }
            }
        }

        // Call handler to get the future (deserializes args synchronously)
        let handler = self.async_registry.get(&command);
        let fut = match handler {
            Some(handler) => {
                let handler_ptr = handler as *const ipc::AsyncCommandHandler;
                // SAFETY: handler_ptr is valid — registry is not mutated during the call.
                match unsafe { (*handler_ptr)(args) } {
                    Ok(f) => f,
                    Err(error) => {
                        self.send_error_response(&id, error);
                        return;
                    }
                }
            }
            None => {
                self.send_error_response(&id, IpcError::unknown_command());
                return;
            }
        };

        // Spawn the future and track the task
        let task_key = format!("{command}:{id}");
        let task = cx.spawn(async move |weak_entity, cx| {
            let result = fut.await;
            let _ = weak_entity.update(cx, |wv, _cx| match result {
                Ok(value) => wv.send_ok_response(&id, value),
                Err(error) => wv.send_error_response(&id, error),
            });
        });
        self.pending_async.insert(task_key, task);
    }

    fn handle_cancel(&mut self, envelope: IpcEnvelope) {
        let id = match &envelope.id {
            Some(id) => id.clone(),
            None => return,
        };

        // Task keys are stored as "{command}:{id}" — find and remove by id suffix.
        // Dropping the Task cancels the in-flight future.
        let key = self
            .pending_async
            .keys()
            .find(|k| k.ends_with(&format!(":{id}")))
            .cloned();

        if let Some(key) = key {
            self.pending_async.remove(&key);
        }
    }

    fn handle_js_emit(&mut self, envelope: IpcEnvelope, cx: &mut mozui::Context<Self>) {
        let name = match &envelope.name {
            Some(n) => n.clone(),
            None => return,
        };

        // Check JS emit allowlist — None means allow all (dev default)
        if let Some(ref allowlist) = self.capabilities.js_emit_allowlist {
            if !allowlist.iter().any(|&allowed| allowed == name) {
                return;
            }
        }

        #[cfg(feature = "ipc-tracing")]
        eprintln!("[wv] emitting JsEmitEvent name={name}");
        cx.emit(JsEmitEvent {
            name,
            payload: envelope.payload,
        });
    }

    pub(crate) fn send_ok_response(&self, id: &str, payload: serde_json::Value) {
        let msg = serde_json::json!({
            "__mozui": true,
            "id": id,
            "type": "response",
            "ok": true,
            "payload": payload,
        });
        if let Ok(json_str) = serde_json::to_string(&msg) {
            let safe = sanitize_for_evaluate(&json_str);
            let _ = self
                .webview
                .evaluate_script(&format!("window.__mozui_dispatch('{safe}')"));
        }
    }

    pub(crate) fn send_error_response(&self, id: &str, error: IpcError) {
        let msg = serde_json::json!({
            "__mozui": true,
            "id": id,
            "type": "response",
            "ok": false,
            "error": error,
        });
        if let Ok(json_str) = serde_json::to_string(&msg) {
            let safe = sanitize_for_evaluate(&json_str);
            let _ = self
                .webview
                .evaluate_script(&format!("window.__mozui_dispatch('{safe}')"));
        }
    }
}

/// Escape U+2028/U+2029 line terminators and single quotes for evaluate_script.
fn sanitize_for_evaluate(s: &str) -> String {
    s.replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
        .replace('\'', "\\'")
}

impl Deref for WebView {
    type Target = wry::WebView;

    fn deref(&self) -> &Self::Target {
        &self.webview
    }
}

impl Focusable for WebView {
    fn focus_handle(&self, _cx: &mozui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for WebView {}
impl EventEmitter<IpcMessage> for WebView {}
impl EventEmitter<NavigationEvent> for WebView {}
impl EventEmitter<NavigationBlockedEvent> for WebView {}
impl EventEmitter<JsEmitEvent> for WebView {}

impl Render for WebView {
    fn render(
        &mut self,
        window: &mut mozui::Window,
        cx: &mut mozui::Context<Self>,
    ) -> impl IntoElement {
        // Drain IPC queue
        let envelopes: Vec<IpcEnvelope> = self.ipc_queue.borrow_mut().drain(..).collect();
        for envelope in envelopes {
            // Emit raw IpcMessage for untyped subscribers
            if let (Some(cmd), Some(args)) = (&envelope.command, &envelope.args) {
                cx.emit(IpcMessage {
                    command: cmd.clone(),
                    args: args.clone(),
                });
            }

            self.handle_ipc(envelope, cx);
        }

        // Drain navigation queues
        for url in self.nav_queue.borrow_mut().drain(..) {
            cx.emit(NavigationEvent { url });
        }
        for url in self.nav_blocked_queue.borrow_mut().drain(..) {
            cx.emit(NavigationBlockedEvent { url });
        }

        let view = cx.entity().clone();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .child({
                let view = cx.entity().clone();
                canvas(
                    move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            .child(WebViewElement::new(self.webview.clone(), view, window, cx))
    }
}

/// A webview element can display a wry webview.
pub struct WebViewElement {
    parent: Entity<WebView>,
    view: Rc<wry::WebView>,
}

impl WebViewElement {
    /// Create a new webview element from a wry WebView.
    pub fn new(
        view: Rc<wry::WebView>,
        parent: Entity<WebView>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self {
        Self { view, parent }
    }
}

impl IntoElement for WebViewElement {
    type Element = WebViewElement;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for WebViewElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&mozui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            flex_shrink: 1.,
            ..Default::default()
        };

        let id = window.request_layout(style, [], cx);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&mozui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if !self.parent.read(cx).visible() {
            return;
        }

        let _ = self.view.set_bounds(Rect {
            size: dpi::Size::Logical(LogicalSize {
                width: bounds.size.width.into(),
                height: bounds.size.height.into(),
            }),
            position: dpi::Position::Logical(dpi::LogicalPosition::new(
                bounds.origin.x.into(),
                bounds.origin.y.into(),
            )),
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&mozui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        // Let WebKit manage cursor within webview bounds — prevents mozui's
        // per-frame cursor reset from fighting with CSS cursor styles.
        window.set_cursor_passthrough(bounds);

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            let webview = self.view.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, _, _, _| {
                if !bounds.contains(&event.position) {
                    let _ = webview.focus_parent();
                }
            });
        });
    }
}
