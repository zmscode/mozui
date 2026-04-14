mod support;

use mozui::prelude::*;
use mozui::{Context, Entity, Render, Window, div, px, size};
use mozui_webview::{IpcMessage, NavigationEvent, WebView, WebViewBuilder};
use serde::Deserialize;
use support::run_plain_example;

fn main() {
    run_plain_example(
        "WebView IPC",
        size(px(800.0), px(600.0)),
        |window, cx| cx.new(|cx| WebViewIpcExample::new(window, cx)),
    );
}

#[derive(Deserialize)]
struct GreetArgs {
    name: String,
}

struct WebViewIpcExample {
    webview: Entity<WebView>,
    last_command: String,
    nav_count: usize,
}

impl WebViewIpcExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let webview = cx.new(|cx| {
            WebViewBuilder::new()
                .html(INLINE_HTML)
                .command("greet", |args: GreetArgs, _wv, _cx| {
                    Ok(serde_json::json!({
                        "message": format!("Hello from Rust, {}!", args.name)
                    }))
                })
                .command("get_time", |_: (), _wv, _cx| {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    Ok(serde_json::json!({ "unix": now }))
                })
                .build(window, cx)
        });

        cx.subscribe(&webview, |this, _, event: &IpcMessage, cx| {
            this.last_command = event.command.clone();
            cx.notify();
        })
        .detach();

        cx.subscribe(&webview, |this, _, _: &NavigationEvent, cx| {
            this.nav_count += 1;
            cx.notify();
        })
        .detach();

        Self {
            webview,
            last_command: "(none)".into(),
            nav_count: 0,
        }
    }
}

impl Render for WebViewIpcExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.webview.clone())
    }
}

const INLINE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    background: #0a0a0a; color: #e0e0e0;
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; height: 100vh; gap: 16px;
  }
  h1 { font-size: 22px; font-weight: 600; color: #fff; }
  .subtitle { font-size: 13px; color: #888; margin-bottom: 12px; }
  button {
    padding: 10px 20px; border: 1px solid #333; border-radius: 8px;
    background: #1a1a1a; color: #e0e0e0; font-size: 14px;
    cursor: pointer; transition: all 0.15s;
  }
  button:hover { background: #252525; border-color: #555; }
  button:active { background: #111; }
  #result {
    margin-top: 12px; padding: 12px 20px; border-radius: 8px;
    background: #111; border: 1px solid #222;
    font-family: "SF Mono", Menlo, monospace; font-size: 13px;
    color: #96f; min-width: 300px; text-align: center;
    min-height: 20px;
  }
  .buttons { display: flex; gap: 10px; }
</style>
</head>
<body>
  <h1>mozui IPC Bridge</h1>
  <p class="subtitle">JS ↔ Rust round-trip via typed commands</p>
  <div class="buttons">
    <button onclick="callGreet()">Greet</button>
    <button onclick="callTime()">Get Time</button>
    <button onclick="callUnknown()">Unknown Cmd</button>
  </div>
  <div id="result"></div>
  <script>
    var el = document.getElementById('result');
    function show(text, color) {
      el.style.color = color || '#96f';
      el.textContent = text;
    }

    async function callGreet() {
      try {
        var res = await window.mozui.invoke('greet', { name: 'World' });
        show(res.message);
      } catch (e) { show('Error: ' + e.message, '#f66'); }
    }

    async function callTime() {
      try {
        var res = await window.mozui.invoke('get_time');
        show('Unix timestamp: ' + res.unix);
      } catch (e) { show('Error: ' + e.message, '#f66'); }
    }

    async function callUnknown() {
      try {
        await window.mozui.invoke('does_not_exist');
        show('Unexpected success', '#f66');
      } catch (e) { show('Expected error: ' + e.code + ' — ' + e.message, '#fa0'); }
    }
  </script>
</body>
</html>"#;
