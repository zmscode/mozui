use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use serde::Deserialize;

use crate::WebView;
use crate::assets::AssetServer;
use crate::ipc::{
    AsyncCommandConfig, AsyncCommandRegistry, CommandRegistry, CommandResult, IpcEnvelope, IpcError,
};
use crate::security::{Capabilities, MOZUI_ORIGIN, MOZUI_SCHEME};

const BRIDGE_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/bridge.min.js"));

/// Platform-aware CSP source for `default-src` and `connect-src`.
#[cfg(target_os = "windows")]
const CSP_SELF_SRC: &str = "https://mozui.localhost";
#[cfg(not(target_os = "windows"))]
const CSP_SELF_SRC: &str = "mozui:";

fn default_csp() -> String {
    format!(
        "default-src 'self' {src}; \
         script-src 'self' {src}; \
         style-src 'unsafe-inline'; \
         style-src-elem 'self' {src}; \
         img-src 'self' {src} data: blob:; \
         connect-src 'self' {src}; \
         object-src 'none'; \
         base-uri 'self'; \
         frame-src 'none'",
        src = CSP_SELF_SRC,
    )
}

/// Check if a URL matches a simple glob pattern.
/// Supports `*` (any non-`/` chars) and `**` (anything including `/`).
pub(crate) fn url_matches_pattern(url: &str, pattern: &str) -> bool {
    // Split pattern on "**" first
    let parts: Vec<&str> = pattern.split("**").collect();
    if parts.len() == 1 {
        // No "**", do simple * matching
        return simple_glob(url, pattern);
    }

    // Match first part as prefix, last part as suffix, middle parts anywhere
    let mut remaining = url;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // Must match as prefix
            if let Some(rest) = match_prefix(remaining, part) {
                remaining = rest;
            } else {
                return false;
            }
        } else if let Some(pos) = find_pattern(remaining, part) {
            remaining = &remaining[pos..];
        } else {
            return false;
        }
    }
    true
}

/// Simple glob: `*` matches any non-`/` sequence.
fn simple_glob(text: &str, pattern: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return text == pattern;
    }
    let mut remaining = text;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else if let Some(pos) = remaining.find(part) {
            // Ensure no `/` between current position and match
            if remaining[..pos].contains('/') {
                return false;
            }
            remaining = &remaining[pos + part.len()..];
        } else {
            return false;
        }
    }
    // If pattern ends with *, remaining can be anything without /
    if pattern.ends_with('*') {
        !remaining.contains('/')
    } else {
        remaining.is_empty()
    }
}

fn match_prefix<'a>(text: &'a str, pattern: &str) -> Option<&'a str> {
    if pattern.contains('*') {
        if simple_glob(text, pattern) {
            Some("")
        } else {
            // Try progressively longer prefixes
            for i in 0..=text.len() {
                if simple_glob(&text[..i], pattern) {
                    return Some(&text[i..]);
                }
            }
            None
        }
    } else if text.starts_with(pattern) {
        Some(&text[pattern.len()..])
    } else {
        None
    }
}

fn find_pattern(text: &str, pattern: &str) -> Option<usize> {
    if pattern.contains('*') {
        for i in 0..text.len() {
            if simple_glob(&text[i..], pattern) {
                return Some(text.len());
            }
        }
        None
    } else {
        text.find(pattern).map(|pos| pos + pattern.len())
    }
}

/// Builder for creating a `WebView` with IPC, security, and bridge injection.
pub struct WebViewBuilder {
    url: Option<String>,
    html: Option<String>,
    capabilities: Capabilities,
    csp: Option<String>,
    navigation_allowlist: Vec<String>,
    command_registry: CommandRegistry,
    async_registry: AsyncCommandRegistry,
    async_config: AsyncCommandConfig,
    asset_server: Option<AssetServer>,
    transparent: bool,
    user_agent: Option<String>,
    #[cfg(feature = "inspector")]
    devtools: bool,
    initialization_scripts: Vec<String>,
}

impl WebViewBuilder {
    pub fn new() -> Self {
        Self {
            url: None,
            html: None,
            capabilities: Capabilities::default(),
            csp: None,
            navigation_allowlist: Vec::new(),
            command_registry: CommandRegistry::new(),
            async_registry: AsyncCommandRegistry::new(),
            async_config: AsyncCommandConfig::default(),
            asset_server: None,
            transparent: false,
            user_agent: None,
            #[cfg(feature = "inspector")]
            devtools: false,
            initialization_scripts: Vec::new(),
        }
    }

    /// Set the URL to load.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set inline HTML content to load.
    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    /// Set the security capabilities for this webview.
    pub fn capabilities(mut self, caps: Capabilities) -> Self {
        self.capabilities = caps;
        self
    }

    /// Override the default Content Security Policy.
    /// If not called, a restrictive default CSP is injected.
    pub fn csp(mut self, policy: impl Into<String>) -> Self {
        self.csp = Some(policy.into());
        self
    }

    /// Add a URL pattern to the navigation allowlist.
    /// Supports `*` (single path segment) and `**` (any path).
    /// `mozui://` is always implicitly allowed.
    pub fn allow_navigation(mut self, pattern: impl Into<String>) -> Self {
        self.navigation_allowlist.push(pattern.into());
        self
    }

    /// Set the asset root directory. Files will be served via the `mozui://` custom protocol.
    pub fn assets(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.asset_server = Some(AssetServer::new(root));
        self
    }

    /// Set a custom-configured asset server.
    pub fn assets_with(mut self, server: AssetServer) -> Self {
        self.asset_server = Some(server);
        self
    }

    /// Register a typed IPC command handler.
    pub fn command<A, F>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        A: for<'de> Deserialize<'de> + 'static,
        F: Fn(A, &mut WebView, &mut mozui::App) -> Result<serde_json::Value, IpcError> + 'static,
    {
        self.command_registry.register(name, handler);
        self
    }

    /// Register a command handler that may return a deferred (async) result.
    ///
    /// The handler has access to `&mut WebView` and `&mut App` (like sync commands)
    /// but can return `CommandResult::Deferred` with a future for async operations
    /// like native file dialogs.
    pub fn deferred_command<A, F>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        A: for<'de> Deserialize<'de> + 'static,
        F: Fn(A, &mut WebView, &mut mozui::App) -> CommandResult + 'static,
    {
        self.command_registry.register_deferred(name, handler);
        self
    }

    /// Register a typed async IPC command handler.
    ///
    /// The handler receives deserialized args and returns a future.
    /// Results are delivered back to JS automatically.
    pub fn async_command<A, Fut, F>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        A: for<'de> Deserialize<'de> + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, IpcError>> + 'static,
        F: Fn(A) -> Fut + 'static,
    {
        self.async_registry.register(name, handler);
        self
    }

    /// Configure async command execution limits.
    pub fn async_config(mut self, config: AsyncCommandConfig) -> Self {
        self.async_config = config;
        self
    }

    /// Set whether the webview background is transparent.
    pub fn transparent(mut self, t: bool) -> Self {
        self.transparent = t;
        self
    }

    /// Set a custom user agent string.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Enable devtools (only effective with `inspector` feature).
    #[cfg(feature = "inspector")]
    pub fn devtools(mut self, enabled: bool) -> Self {
        self.devtools = enabled;
        self
    }

    /// Add a custom initialization script to run after the bridge is injected.
    pub fn initialization_script(mut self, script: impl Into<String>) -> Self {
        self.initialization_scripts.push(script.into());
        self
    }

    /// Build the WebView, wiring up IPC, bridge injection, security, CSP, and navigation.
    ///
    /// `window` must be a mozui Window (implements `HasWindowHandle`).
    pub fn build(self, window: &mut mozui::Window, cx: &mut mozui::App) -> WebView {
        let ipc_queue: Rc<RefCell<VecDeque<IpcEnvelope>>> = Rc::new(RefCell::new(VecDeque::new()));
        let nav_queue: Rc<RefCell<VecDeque<String>>> = Rc::new(RefCell::new(VecDeque::new()));
        let nav_blocked_queue: Rc<RefCell<VecDeque<String>>> =
            Rc::new(RefCell::new(VecDeque::new()));

        let max_payload = self.capabilities.max_payload_bytes;
        let ipc_origins = self.capabilities.ipc_origins.clone();
        let dev_mode = self.capabilities.dev_mode;

        // CSP: use custom or generate default
        let csp = self.csp.unwrap_or_else(default_csp);

        // Build bridge init script with platform origin/scheme + CSP meta tag injected
        let csp_meta = format!("<meta http-equiv=\"Content-Security-Policy\" content=\"{csp}\">");
        let origin_script = format!(
            "window.__mozui_origin = \"{MOZUI_ORIGIN}\";\n\
             window.__mozui_scheme = \"{MOZUI_SCHEME}\";\n"
        );

        // Wire up wry builder
        let mut wry_builder = wry::WebViewBuilder::new();

        // Inject CSP meta tag as early initialization script
        wry_builder = wry_builder.with_initialization_script(format!(
            "document.head.insertAdjacentHTML('afterbegin', '{csp_meta}');"
        ));

        // Inject bridge JS (before any user scripts)
        wry_builder =
            wry_builder.with_initialization_script(format!("{origin_script}\n{BRIDGE_JS}"));

        // Inject user initialization scripts
        for script in &self.initialization_scripts {
            wry_builder = wry_builder.with_initialization_script(script.clone());
        }

        // Custom protocol for asset serving
        if let Some(mut asset_server) = self.asset_server {
            // Inject CSP into HTML responses served by the asset server
            asset_server = asset_server.with_csp(csp.clone());
            wry_builder = wry_builder
                .with_custom_protocol("mozui".into(), move |_webview_id, request| {
                    asset_server.serve(&request).map(Into::into)
                });
        }

        // Set content
        if let Some(url) = &self.url {
            wry_builder = wry_builder.with_url(url);
        } else if let Some(html) = &self.html {
            wry_builder = wry_builder.with_html(html);
        }

        // Appearance
        wry_builder = wry_builder.with_transparent(self.transparent);
        if let Some(ua) = &self.user_agent {
            wry_builder = wry_builder.with_user_agent(ua);
        }

        // DevTools
        #[cfg(feature = "inspector")]
        {
            wry_builder = wry_builder.with_devtools(self.devtools);
        }

        // IPC handler: validate origin, check payload size, push to queue
        let queue_for_ipc = Rc::clone(&ipc_queue);
        wry_builder = wry_builder.with_ipc_handler(move |request: http::Request<String>| {
            let origin = request
                .headers()
                .get("Origin")
                .and_then(|v| v.to_str().ok());

            if !ipc_origins.permits(origin) {
                #[cfg(feature = "ipc-tracing")]
                eprintln!("[wv:ipc] REJECTED origin={origin:?}");
                return;
            }
            #[cfg(feature = "ipc-tracing")]
            if let Ok(env) = serde_json::from_str::<serde_json::Value>(request.body()) {
                let typ = env.get("type").and_then(|v| v.as_str()).unwrap_or("?");
                let cmd = env.get("command").and_then(|v| v.as_str())
                    .or_else(|| env.get("name").and_then(|v| v.as_str()))
                    .unwrap_or("?");
                eprintln!("[wv:ipc] type={typ} cmd/name={cmd} origin={origin:?}");
            }

            if !dev_mode {
                if let Some(o) = origin {
                    if o.starts_with("http://localhost") {
                        return;
                    }
                }
            }

            let body = request.body();

            if body.len() > max_payload {
                return;
            }

            let envelope: IpcEnvelope = match serde_json::from_str(body) {
                Ok(e) => e,
                Err(_) => return,
            };

            queue_for_ipc.borrow_mut().push_back(envelope);
        });

        // Navigation handler: allowlist with glob matching
        let nav_allowlist = self.navigation_allowlist.clone();
        let external_nav = self.capabilities.external_navigation;
        let nav_q = Rc::clone(&nav_queue);
        let nav_blocked_q = Rc::clone(&nav_blocked_queue);
        wry_builder = wry_builder.with_navigation_handler(move |url: String| {
            // mozui:// always allowed
            if url.starts_with("mozui://") || url.starts_with("https://mozui.localhost") {
                nav_q.borrow_mut().push_back(url);
                return true;
            }

            // about:blank always allowed (initial load)
            if url == "about:blank" {
                return true;
            }

            // Block dangerous schemes unconditionally
            if url.starts_with("javascript:") || url.starts_with("data:") {
                nav_blocked_q.borrow_mut().push_back(url);
                return false;
            }

            // Dev mode: allow localhost
            if dev_mode && url.starts_with("http://localhost") {
                nav_q.borrow_mut().push_back(url);
                return true;
            }

            // Check allowlist patterns
            for pattern in &nav_allowlist {
                if url_matches_pattern(&url, pattern) {
                    nav_q.borrow_mut().push_back(url);
                    return true;
                }
            }

            // External navigation allowed?
            if external_nav {
                nav_q.borrow_mut().push_back(url);
                return true;
            }

            nav_blocked_q.borrow_mut().push_back(url);
            false
        });

        // Build the wry webview as a child of mozui's native view so it doesn't
        // replace the content view (which owns the Metal renderer and event handlers).
        let wry_webview = wry_builder
            .build_as_child(window)
            .expect("failed to build wry WebView");

        let _ = wry_webview.set_bounds(wry::Rect::default());

        WebView {
            focus_handle: cx.focus_handle(),
            webview: Rc::new(wry_webview),
            visible: true,
            bounds: mozui::Bounds::default(),
            command_registry: self.command_registry,
            ipc_queue,
            nav_queue,
            nav_blocked_queue,
            async_registry: self.async_registry,
            pending_async: std::collections::HashMap::new(),
            async_config: self.async_config,
            capabilities: self.capabilities,
        }
    }
}
