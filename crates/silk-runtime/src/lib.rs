mod commands;
mod config;
mod ipc_router;
mod window_manager;

pub use config::{SilkConfig, WindowConfig};

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mozui::prelude::*;
use mozui::{
    App, Application, Entity, Render, TitlebarOptions, Window, WindowBounds, WindowOptions,
    current_platform, div, px, size,
};
use mozui_webview::{
    Capabilities, CommandAllowlist, JsEmitEvent, OriginAllowlist, WebView, WebViewBuilder,
};

use crate::commands::window::{CreateWindowArgs, EmitAllArgs, EmitToArgs, WindowLabelArgs};
use crate::ipc_router::IpcRouter;
use crate::window_manager::WindowManager;

const SILK_MAIN_BRIDGE: &str = include_str!("silk_main_bridge.js");
const SILK_RENDERER_BRIDGE: &str = include_str!("silk_renderer_bridge.js");

type SharedRouter = Rc<RefCell<IpcRouter>>;
type SharedWm = Rc<RefCell<WindowManager>>;

/// Boot the Silk runtime from a project directory.
///
/// Parses `silk.config.json`, opens a hidden main webview running the developer's
/// `main.js`, and waits for `Silk.createWindow()` calls to open visible windows.
pub fn boot(project_root: PathBuf) {
    let platform = current_platform(false);
    Application::with_platform(platform).run(move |cx| {
        boot_inner(project_root, cx);
    });
}

fn boot_inner(project_root: PathBuf, cx: &mut App) {
    let config = SilkConfig::load(&project_root).expect("failed to load silk.config.json");

    let app_dir = config.app_dir(&project_root);
    let main_path = config.main_path(&project_root);
    let main_js = std::fs::read_to_string(&main_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", main_path.display()));

    // devUrl only activates when SILK_DEV=1 is set (e.g. running alongside vite dev)
    let dev_url = if std::env::var("SILK_DEV").is_ok() {
        config.dev_url.clone()
    } else {
        None
    };

    let router: SharedRouter = Rc::new(RefCell::new(IpcRouter::new()));
    let wm: SharedWm = Rc::new(RefCell::new(WindowManager::new(
        app_dir.clone(),
        config.window,
        dev_url,
    )));

    // Main window must be "shown" for the platform to draw it (draining IPC).
    // We make it 1x1 with no titlebar to keep it invisible to the user.
    let main_opts = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(mozui::Bounds {
            origin: mozui::point(px(-10000.0), px(-10000.0)),
            size: size(px(1.0), px(1.0)),
        })),
        titlebar: Some(TitlebarOptions {
            title: Some("Silk Main".into()),
            appears_transparent: true,
            ..Default::default()
        }),
        focus: false,
        show: true,
        ..Default::default()
    };

    let r = Rc::clone(&router);
    let w = Rc::clone(&wm);
    let ad = app_dir.clone();

    cx.open_window(main_opts, move |window, cx| {
        cx.new(move |cx| SilkApp::new(&main_js, &ad, r, w, window, cx))
    })
    .expect("failed to open Silk main window");
}

/// The root entity for the Silk runtime. Lives in the hidden main window.
/// Owns the main webview and handles IPC routing between main and renderer windows.
struct SilkApp {
    main_webview: Entity<WebView>,
    router: SharedRouter,
    wm: SharedWm,
}

impl SilkApp {
    fn new(
        main_js: &str,
        app_dir: &Path,
        router: SharedRouter,
        wm: SharedWm,
        window: &mut Window,
        cx: &mut mozui::Context<Self>,
    ) -> Self {
        let main_webview = cx.new(|cx| {
            build_main_webview(
                main_js,
                app_dir,
                Rc::clone(&router),
                Rc::clone(&wm),
                window,
                cx,
            )
        });

        wm.borrow_mut().main_webview = Some(main_webview.clone());

        // Listen for __silk:response from main process JS
        cx.subscribe(&main_webview, Self::on_main_emit).detach();

        // Poll timer: the webview only drains its IPC queue during render().
        // Since the main webview has no user interaction, we must periodically
        // notify to trigger re-renders that process incoming IPC messages.
        cx.spawn(async move |_this, cx| {
            loop {
                cx.background_executor().timer(std::time::Duration::from_millis(16)).await;
                cx.update(|cx| cx.refresh_windows());
            }
        })
        .detach();

        Self {
            main_webview,
            router,
            wm,
        }
    }

    fn on_main_emit(
        &mut self,
        _entity: Entity<WebView>,
        event: &JsEmitEvent,
        cx: &mut mozui::Context<Self>,
    ) {
        if event.name == "__silk:response" {
            if let Some(payload) = &event.payload {
                self.handle_main_response(payload, cx);
            }
        }
    }

    /// Route a __silk:response from main.js back to the originating renderer.
    fn handle_main_response(&mut self, payload: &serde_json::Value, cx: &mut mozui::Context<Self>) {
        let correlation_id = match payload.get("correlationId").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                log::warn!("[silk] response missing correlationId");
                return;
            }
        };

        let forward = match self.router.borrow_mut().resolve(&correlation_id) {
            Some(f) => f,
            None => {
                log::warn!("[silk] no pending forward for correlationId={correlation_id}");
                return;
            }
        };

        #[cfg(feature = "ipc-tracing")]
        log::debug!(
            "[silk] routing response: corr={correlation_id} src={} invoke={}",
            forward.source_window, forward.original_invoke_id
        );

        let renderer_wv = {
            let wm = self.wm.borrow();
            match wm.get(&forward.source_window) {
                Some(m) => m.webview.clone(),
                None => {
                    log::warn!("[silk] window '{}' not found in wm", forward.source_window);
                    return;
                }
            }
        };

        let ok = payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        let invoke_id = forward.original_invoke_id.clone();

        let response_payload = serde_json::json!({
            "invokeId": invoke_id,
            "ok": ok,
            "payload": if ok {
                payload.get("payload").cloned().unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            },
            "error": if !ok {
                payload.get("error").cloned().unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            },
        });

        renderer_wv.update(cx, |wv, _cx| {
            if let Err(e) = wv.emit_to_js("__silk:invoke-response", &response_payload) {
                log::error!("[silk] emit_to_js error: {e}");
            }
        });
    }

}

impl Render for SilkApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut mozui::Context<Self>) -> impl IntoElement {
        div().size_full().child(self.main_webview.clone())
    }
}

/// Build the hidden main process webview with built-in commands.
fn build_main_webview(
    main_js: &str,
    app_dir: &Path,
    router: SharedRouter,
    wm: SharedWm,
    window: &mut Window,
    cx: &mut App,
) -> WebView {
    let wm_create = Rc::clone(&wm);
    let wm_close = Rc::clone(&wm);
    let wm_emit = Rc::clone(&wm);
    let wm_emit_all = Rc::clone(&wm);
    let router_create = Rc::clone(&router);
    let wm_subscribe = Rc::clone(&wm);
    let ad = app_dir.to_path_buf();

    WebViewBuilder::new()
        .html("<!DOCTYPE html><html><head></head><body></body></html>")
        .capabilities(Capabilities {
            commands: CommandAllowlist::All,
            script_execution: true,
            js_emit_allowlist: None,
            dev_mode: cfg!(debug_assertions),
            ipc_origins: OriginAllowlist::Any,
            ..Default::default()
        })
        .initialization_script(SILK_MAIN_BRIDGE)
        .initialization_script(main_js)
        // __silk:create-window
        .command(
            "__silk:create-window",
            move |args: CreateWindowArgs, _wv, cx| {
                let label = args.label.clone();
                let app_dir = ad.clone();
                let w = Rc::clone(&wm_create);
                let router = Rc::clone(&router_create);
                let wm_sub = Rc::clone(&wm_subscribe);

                // If devUrl is configured and the requested URL is relative, use devUrl
                let url = {
                    let wm = w.borrow();
                    let raw = args.url.clone();
                    match (&wm.dev_url, &raw) {
                        (Some(dev), None) => Some(dev.clone()),
                        (Some(dev), Some(u)) if !u.starts_with("http://") && !u.starts_with("https://") => {
                            Some(dev.clone())
                        }
                        _ => raw,
                    }
                };

                let transparent_titlebar = args.titlebar_style.as_deref()
                    .map(|s| matches!(s, "hidden" | "hiddenInset"))
                    .unwrap_or(false);

                let opts = {
                    let wm = w.borrow();
                    wm.build_window_options(
                        args.title,
                        args.width,
                        args.height,
                        args.min_width,
                        args.min_height,
                        args.resizable,
                        args.titlebar_style,
                        args.traffic_light_position,
                        cx,
                    )
                };

                // Defer window creation to avoid arena conflicts during render
                let label_for_defer = label.clone();
                cx.defer(move |cx| {
                    let label = label_for_defer;
                    let label_inner = label.clone();
                    let w_inner = Rc::clone(&w);

                    match cx.open_window(opts, move |window, cx| {
                        cx.new(move |cx| {
                            let webview = cx.new(|cx| {
                                build_renderer_webview(url.as_deref(), &app_dir, transparent_titlebar, window, cx)
                            });

                            w_inner.borrow_mut().register(label_inner, webview.clone());

                            RendererRoot { webview }
                        })
                    }) {
                        Ok(_) => {
                            // Subscribe to renderer webview's JsEmitEvent for __silk:invoke
                            let wm_borrow = wm_sub.borrow();
                            if let Some(managed) = wm_borrow.get(&label) {
                                let renderer_wv = managed.webview.clone();
                                let router = Rc::clone(&router);
                                let wm_for_invoke = Rc::clone(&wm_sub);
                                let source_label = label.clone();
                                drop(wm_borrow);

                                cx.subscribe(
                                    &renderer_wv,
                                    move |_entity, event: &JsEmitEvent, cx| {
                                        if event.name != "__silk:invoke" {
                                            return;
                                        }
                                        let payload = match &event.payload {
                                            Some(p) => p,
                                            None => return,
                                        };

                                        let invoke_id = match payload
                                            .get("invokeId")
                                            .and_then(|v| v.as_str())
                                        {
                                            Some(id) => id.to_string(),
                                            None => return,
                                        };
                                        let command = match payload
                                            .get("command")
                                            .and_then(|v| v.as_str())
                                        {
                                            Some(c) => c.to_string(),
                                            None => return,
                                        };
                                        let args = payload
                                            .get("args")
                                            .cloned()
                                            .unwrap_or(serde_json::Value::Null);

                                        let correlation_id = router
                                            .borrow_mut()
                                            .create_forward(source_label.clone(), invoke_id);

                                        let wm = wm_for_invoke.borrow();
                                        if let Some(main_wv) = &wm.main_webview {
                                            let _ = main_wv.update(cx, |wv, _cx| {
                                                let _ = wv.emit_to_js(
                                                    "__silk:forward",
                                                    serde_json::json!({
                                                        "correlationId": correlation_id,
                                                        "command": command,
                                                        "args": args,
                                                    }),
                                                );
                                            });
                                        }
                                    },
                                )
                                .detach();
                            }
                        }
                        Err(e) => {
                            log::error!("failed to create window: {e}");
                        }
                    }
                });

                Ok(serde_json::json!({ "label": label }))
            },
        )
        // __silk:close-window
        .command(
            "__silk:close-window",
            move |args: WindowLabelArgs, _wv, _cx| {
                wm_close.borrow_mut().remove(&args.label);
                Ok(serde_json::json!(true))
            },
        )
        // __silk:emit-to
        .command("__silk:emit-to", move |args: EmitToArgs, _wv, cx| {
            let wm = wm_emit.borrow();
            if let Some(managed) = wm.get(&args.label) {
                let _ = managed.webview.update(cx, |wv, _cx| {
                    let _ =
                        wv.emit_to_js(&args.event, args.payload.unwrap_or(serde_json::Value::Null));
                });
            }
            Ok(serde_json::json!(true))
        })
        // __silk:emit-all
        .command("__silk:emit-all", move |args: EmitAllArgs, _wv, cx| {
            let wm = wm_emit_all.borrow();
            for managed in wm.renderer_windows.values() {
                let _ = managed.webview.update(cx, |wv, _cx| {
                    let _ = wv.emit_to_js(
                        &args.event,
                        args.payload.clone().unwrap_or(serde_json::Value::Null),
                    );
                });
            }
            Ok(serde_json::json!(true))
        })
        // __silk:quit
        .command("__silk:quit", |_: (), _wv, cx| {
            cx.quit();
            Ok(serde_json::json!(true))
        })
        .build(window, cx)
}

/// Build a renderer webview with the silk renderer bridge and asset server.
fn build_renderer_webview(
    url: Option<&str>,
    asset_root: &Path,
    transparent: bool,
    window: &mut Window,
    cx: &mut App,
) -> WebView {
    let resolved = url.unwrap_or("mozui://localhost/index.html");
    let is_dev_server = resolved.starts_with("http://localhost")
        || resolved.starts_with("http://127.0.0.1");

    let csp = if is_dev_server {
        // Dev server: allow localhost + WebSocket (HMR) + inline styles (Vite injects them)
        "default-src 'self' http://localhost:* ws://localhost:* mozui:; \
         script-src 'self' 'unsafe-inline' http://localhost:* mozui:; \
         style-src 'self' 'unsafe-inline' http://localhost:* mozui:; \
         img-src 'self' http://localhost:* mozui: data: blob:; \
         connect-src 'self' http://localhost:* ws://localhost:* mozui:; \
         object-src 'none'; \
         base-uri 'self'; \
         frame-src 'none'"
            .to_string()
    } else {
        // Production: restrictive CSP
        "default-src 'self' mozui:; \
         script-src 'self' mozui:; \
         style-src 'self' 'unsafe-inline' mozui:; \
         img-src 'self' mozui: data: blob:; \
         connect-src 'self' mozui:; \
         object-src 'none'; \
         base-uri 'self'; \
         frame-src 'none'"
            .to_string()
    };

    let mut builder = WebViewBuilder::new()
        .capabilities(Capabilities {
            commands: CommandAllowlist::All,
            js_emit_allowlist: None,
            dev_mode: cfg!(debug_assertions),
            ipc_origins: if cfg!(debug_assertions) {
                OriginAllowlist::Any
            } else {
                OriginAllowlist::MozuiOnly
            },
            ..Default::default()
        })
        .csp(csp)
        .transparent(transparent)
        .assets(asset_root)
        .initialization_script(SILK_RENDERER_BRIDGE)
        // Built-in commands — handled directly in Rust, no main process round-trip
        .command("__silk:fs-read-text", commands::fs::read_text)
        .command("__silk:fs-write-text", commands::fs::write_text)
        .command("__silk:fs-exists", commands::fs::exists)
        .command("__silk:fs-mkdir", commands::fs::mkdir)
        .command("__silk:fs-remove", commands::fs::remove)
        .command("__silk:clipboard-read", commands::clipboard::read)
        .command("__silk:clipboard-write", commands::clipboard::write)
        .deferred_command("__silk:dialog-open", commands::dialog::open)
        .deferred_command("__silk:dialog-save", commands::dialog::save)
        .deferred_command("__silk:dialog-message", commands::dialog::message)
        .command("__silk:shell-open", commands::shell::open);

    if resolved.starts_with("http://")
        || resolved.starts_with("https://")
        || resolved.starts_with("mozui://")
    {
        builder = builder.url(resolved);
    } else {
        let path = resolved.trim_start_matches("./");
        builder = builder.url(&format!("mozui://localhost/{path}"));
    }

    builder.build(window, cx)
}

/// Root view for renderer windows.
struct RendererRoot {
    webview: Entity<WebView>,
}

impl Render for RendererRoot {
    fn render(&mut self, _window: &mut Window, _cx: &mut mozui::Context<Self>) -> impl IntoElement {
        div().size_full().child(self.webview.clone())
    }
}
