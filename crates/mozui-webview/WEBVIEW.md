# mozui-webview: Full-Stack Native App Platform

## Vision

mozui-webview should feel like **Electron for people who care about performance and security** — and like **Tauri without the friction**. Concretely:

- **Electron DX:** TypeScript-first renderer, typed IPC with `invoke()` and `listen()`, hot reload in dev, zero ceremony to call Rust from JS
- **Tauri power:** system webview (no bundled Chromium), tiny binaries, full Rust backend, explicit capability/security model
- **mozui native:** the webview is a first-class mozui entity — it composes with signals, layouts, and the rest of the component tree, not bolted on as a foreign object

The renderer is a full web app. The Rust side is the platform. The bridge between them is typed, validated, and secure by default.

---

## Security Model

Security is opt-out, not opt-in. Every feature defaults to the most restrictive posture.

### Threat Surface

| Surface | Threat | Mitigation |
|---|---|---|
| IPC handler | Malicious JS calls arbitrary commands | Command allowlist; unregistered commands return error |
| IPC arguments | Prototype pollution, oversized payloads | Serde deserialization as schema validation; payload size limit |
| Navigation | Open redirect to arbitrary URL | URL allowlist with glob patterns; explicit deny for `javascript:`/`data:` before glob |
| `evaluate_script` | Arbitrary code injection from Rust side | Gate behind `script_execution` capability; `emit_to_js` not gated (validated JSON only) |
| Custom protocol | Path traversal in asset server | Canonicalize paths; jail to asset root; `.ts` source files not served |
| DevTools | Exposes internals in production | DevTools gated behind `inspector` feature flag only |
| Origin spoofing | Non-mozui origin calls IPC | Validate `Origin` header; platform-correct origin constant per target |
| JS → Rust data | Unvalidated data enters Rust | All command args deserialized via serde; reject on failure |
| Bridge globals | Overwrite `window.mozui` to intercept/hijack bridge | `Object.defineProperty` + `Object.freeze` — non-writable, non-configurable |
| Error message disclosure | `IpcError::internal` leaks Rust error strings to renderer | Internal errors return generic message; real error logged server-side only |
| JS event injection | `mozui.emit()` fires arbitrary events into mozui entity system | `js_emit_allowlist` in `Capabilities`; unknown names silently dropped |
| `CommandAllowlist::All` in production | All commands callable if developer forgets to restrict | Deprecated in release builds; compile-time warning emitted |
| `debug_assertions` as security boundary | Release build with debug profile ships with localhost open | Explicit `dev_mode: bool` in `Capabilities`; no security decisions from compiler flags |
| Timing oracle | `UNAUTHORIZED` reveals a command exists in the registry | Both allowlist-denied and unregistered commands return `UNKNOWN_COMMAND` |
| CSS exfiltration | `style-src 'unsafe-inline'` allows CSS data exfil | `style-src-elem 'self'` added; `unsafe-inline` restricted to inline attributes only |
| JS line terminators | U+2028/U+2029 in error messages break `evaluate_script` | `sanitize_for_evaluate()` escapes both before any `evaluate_script` call |

### Capability System

Commands and sensitive APIs are gated by explicit capabilities declared at build time. This is similar to Tauri's permission system but lighter-weight.

```rust
pub struct Capabilities {
    /// Which commands JS is allowed to invoke. See `CommandAllowlist`.
    pub commands: CommandAllowlist,
    /// Allow raw `evaluate_script` calls from Rust. Default: false.
    /// `emit_to_js` bypasses this gate — it calls `__mozui_dispatch` with
    /// validated JSON only, never arbitrary code.
    pub script_execution: bool,
    /// Allow navigations to external (non-mozui://) URLs not in the allowlist.
    pub external_navigation: bool,
    /// Maximum IPC payload size in bytes. Default: 1MB.
    pub max_payload_bytes: usize,
    /// Which origins may send IPC messages. Default: mozui:// only.
    pub ipc_origins: OriginAllowlist,
    /// Which event names JS is allowed to emit via `mozui.emit()`.
    /// None = allow all (acceptable in development); Some([...]) = allowlist.
    /// Unknown event names are silently dropped — never error-responded,
    /// to avoid revealing the event surface to a probing renderer.
    pub js_emit_allowlist: Option<Vec<&'static str>>,
    /// Enable development-mode relaxations: localhost IPC origin allowed,
    /// localhost navigation allowed. Must be set explicitly — never inferred
    /// from compiler flags. Default: false.
    pub dev_mode: bool,
}

pub enum CommandAllowlist {
    /// Allow all registered commands.
    /// DEPRECATED in release builds — emits a compile-time warning.
    /// Set `CommandAllowlist::Only` explicitly before shipping.
    #[cfg_attr(
        not(debug_assertions),
        deprecated = "CommandAllowlist::All in release builds allows every registered \
                      command from the renderer. Set CommandAllowlist::Only explicitly."
    )]
    All,
    /// Allow only the named commands. All others return UNKNOWN_COMMAND.
    Only(Vec<&'static str>),
}

pub enum OriginAllowlist {
    /// Only the platform-correct mozui:// (macOS/Linux) or
    /// https://mozui.localhost (Windows) origin. Production default.
    MozuiOnly,
    /// Exact-match against the HTTP Origin header.
    /// Example: `["http://localhost:5173"]` for a Vite/Bun dev server.
    Custom(Vec<String>),
    /// Any origin. Use only for fully trusted, sandboxed content.
    Any,
}
```

Default `Capabilities`:

```rust
impl Default for Capabilities {
    fn default() -> Self {
        Self {
            #[allow(deprecated)]  // All is the correct default for development
            commands: CommandAllowlist::All,
            script_execution: false,
            external_navigation: false,
            max_payload_bytes: 1024 * 1024,
            ipc_origins: OriginAllowlist::MozuiOnly,
            js_emit_allowlist: None,
            dev_mode: false,
        }
    }
}
```

### Content Security Policy

`WebViewBuilder` auto-injects a CSP unless explicitly overridden. The default CSP:

```
default-src 'self' mozui:;
script-src 'self' 'nonce-{RANDOM_PER_SESSION}';
style-src 'unsafe-inline';
style-src-elem 'self' mozui:;
img-src 'self' mozui: data: blob:;
connect-src 'self' mozui:;
object-src 'none';
base-uri 'self';
frame-src 'none';
```

Key choices:
- `mozui:` scheme is always allowed (custom protocol for local assets)
- `nonce` for scripts — generated fresh each session, injected into the initialization script
- No `unsafe-eval` — eliminates a class of XSS escalation
- `frame-src 'none'` — no iframes by default (can opt in)
- `object-src 'none'` — no plugins
- `style-src 'unsafe-inline'` retained for inline `style=""` attributes (required by most frameworks), but `style-src-elem 'self'` separately restricts `<style>` blocks and `<link rel="stylesheet">` to same-origin only — this closes the CSS exfiltration vector via external stylesheets while preserving attribute-level inline styles

CSP is delivered via `with_initialization_script` as a meta tag injected before any content loads, and also via custom protocol response headers when serving local assets.

---

## IPC Architecture

### Protocol: JSON-RPC-like over wry IPC

All messages share a common envelope. This keeps the protocol uniform whether it's a command invocation or an event.

**JS → Rust (invoke):**
```json
{
  "__mozui": true,
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "type": "invoke",
  "command": "read_file",
  "args": { "path": "/tmp/foo.txt" }
}
```

**Rust → JS (invoke response):**
```json
{
  "__mozui": true,
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "type": "response",
  "ok": true,
  "payload": "file contents here"
}
```

**Rust → JS (error response):**
```json
{
  "__mozui": true,
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "type": "response",
  "ok": false,
  "error": { "code": "NOT_FOUND", "message": "File not found" }
}
```

**Rust → JS (event push):**
```json
{
  "__mozui": true,
  "type": "event",
  "name": "file_changed",
  "payload": { "path": "/tmp/foo.txt" }
}
```

The `__mozui: true` sentinel allows the JS bridge to filter its own messages from any other `postMessage` traffic.

### Internal Message Flow

```
JS invoke()
  → window.__mozui_send(JSON)        [bridge JS]
    → wry IPC handler                [wry callback, main thread]
      → push to Rc<RefCell<VecDeque<IpcEnvelope>>>
        → WebView::render() drains queue
          → validate origin
          → check payload size
          → look up command in registry
          → deserialize args via serde
          → call command handler with (args, &mut WebView, cx)
            → on completion: evaluate_script("window.__mozui_resolve(id, result)")
```

### The `Rc<RefCell<>>` Question

`Rc<RefCell<VecDeque>>` is correct for this use case. wry's IPC handler requires `F: 'static`, not `F: Send`. Since the webview and its callbacks run entirely on the main thread, `Rc` is safe and avoids `Mutex` overhead. The `Rc` is cloned into the wry closure; `WebView` holds the other end.

For the response path (`evaluate_script`), an `Rc<wry::WebView>` is already stored on `WebView` — this is what we call `evaluate_script` on during render.

### Command Registry

Commands are registered on `WebViewBuilder` and stored in `WebView`. The registry maps command names to boxed handler functions.

```rust
type CommandHandler = Box<
    dyn Fn(
        serde_json::Value,          // raw args — handler deserializes its own type
        &mut WebView,
        &mut App,
    ) -> CommandResult
>;

pub struct CommandRegistry {
    handlers: HashMap<String, CommandHandler>,
}

pub enum CommandResult {
    /// Immediate synchronous result
    Immediate(Result<serde_json::Value, IpcError>),
    /// Async result — handler will call cx.emit(IpcResponse { id, ... }) later
    Deferred,
}
```

For the initial implementation, synchronous `Immediate` results cover most use cases. Async/deferred is phase 2.

### Registering Commands

```rust
WebViewBuilder::new()
    .command("read_file", |args: ReadFileArgs, _webview, _cx| {
        let contents = std::fs::read_to_string(&args.path)?;
        Ok(serde_json::to_value(contents)?)
    })
    .command("get_app_version", |_: (), _webview, _cx| {
        Ok(serde_json::json!("1.0.0"))
    })
```

The `command` method is generic over the args type:

```rust
pub fn command<A, F>(mut self, name: impl Into<String>, handler: F) -> Self
where
    A: for<'de> serde::Deserialize<'de>,
    F: Fn(A, &mut WebView, &mut App) -> Result<serde_json::Value, IpcError> + 'static,
```

Deserialization failure (wrong arg shape from JS) returns an `IpcError` with code `INVALID_ARGS` — never panics.

### IpcError

```rust
#[derive(Debug, serde::Serialize)]
pub struct IpcError {
    pub code: &'static str,
    pub message: String,
}

impl IpcError {
    pub const UNKNOWN_COMMAND: &'static str = "UNKNOWN_COMMAND";  // also used for allowlist-denied
    pub const INVALID_ARGS: &'static str = "INVALID_ARGS";
    pub const PAYLOAD_TOO_LARGE: &'static str = "PAYLOAD_TOO_LARGE";
    pub const INTERNAL: &'static str = "INTERNAL";       // message is always "internal error"
    pub const RATE_LIMITED: &'static str = "RATE_LIMITED";
    // No UNAUTHORIZED — allowlist-denied and unregistered commands both return UNKNOWN_COMMAND
    // to prevent timing oracle attacks that enumerate the command surface.
}
```

### Origin Validation

The wry IPC handler receives an `http::Request<String>`. The `Origin` header is checked against `Capabilities::ipc_origins` before the message even enters the queue. Messages from disallowed origins are silently dropped (no error response — don't reveal the bridge exists to foreign origins).

---

## TypeScript Bridge SDK

The bridge is a small JS module injected via `with_initialization_script` before any page code runs. It is not a separate npm package (though one could be published for DX tooling). It sets up `window.__mozui_*` internals and exports a clean `mozui` global.

### Injected Bridge (`__mozui_bridge.js`, ~3KB minified)

```typescript
// Internal state — closure-scoped, not reachable from page code
const _pending = new Map<string, { resolve: Function; reject: Function }>();
const _listeners = new Map<string, Set<Function>>();

// Internal send function — also closure-scoped
function _send(json: string): void {
  // Platform-correct IPC entry point (macOS/Linux vs Windows WebView2)
  if ((window as any).ipc) {
    (window as any).ipc.postMessage(json);
  } else if ((window as any).chrome?.webview) {
    (window as any).chrome.webview.postMessage(json);
  }
}

// Called by Rust via evaluate_script("window.__mozui_dispatch(JSON)")
// Defined as non-writable and non-configurable so page code cannot overwrite it.
Object.defineProperty(window, '__mozui_dispatch', {
  value: (rawJson: string) => {
    let msg: any;
    try { msg = JSON.parse(rawJson); } catch { return; }
    if (!msg?.__mozui) return;

    if (msg.type === 'response') {
      const pending = _pending.get(msg.id);
      if (!pending) return;
      _pending.delete(msg.id);
      if (msg.ok) {
        pending.resolve(msg.payload);
      } else {
        pending.reject(Object.assign(
          new Error(msg.error?.message ?? 'IPC error'),
          { code: msg.error?.code ?? 'INTERNAL' }
        ));
      }
    } else if (msg.type === 'event') {
      _listeners.get(msg.name)?.forEach(h => h(msg.payload));
    }
  },
  writable: false,
  configurable: false,
  enumerable: false,
});

// Public API — frozen so page code cannot replace or extend it
const _mozui = {
  invoke<T = unknown>(command: string, args?: unknown): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = (crypto as any).randomUUID?.() ??
        Math.random().toString(36).slice(2) + Date.now().toString(36);
      _pending.set(id, { resolve, reject });
      _send(JSON.stringify({ __mozui: true, id, type: 'invoke', command, args: args ?? null }));
      setTimeout(() => {
        if (_pending.has(id)) {
          _pending.delete(id);
          reject(Object.assign(new Error(`mozui invoke timed out: ${command}`), { code: 'TIMEOUT' }));
        }
      }, 30_000);
    });
  },

  listen<T = unknown>(event: string, handler: (payload: T) => void): () => void {
    if (!_listeners.has(event)) _listeners.set(event, new Set());
    _listeners.get(event)!.add(handler);
    return () => _listeners.get(event)?.delete(handler);
  },

  emit(event: string, payload?: unknown): void {
    _send(JSON.stringify({ __mozui: true, type: 'emit', name: event, payload: payload ?? null }));
  },
};

Object.freeze(_mozui);
Object.defineProperty(window, 'mozui', {
  value: _mozui,
  writable: false,
  configurable: false,
  enumerable: true,
});
```

The Rust side calls `window.__mozui_dispatch(json)` via `evaluate_script` to deliver responses and push events.

### TypeScript Type Generation

For typed `invoke()` calls without manual casting, we generate a TypeScript declaration file from Rust command/event types.

**Approach: `specta` crate**

[`specta`](https://github.com/oscartbeaumont/specta) generates TypeScript types from Rust types that derive `specta::Type`. Add it as an optional dev-dependency with a `ts-gen` feature flag:

```toml
[features]
ts-gen = ["dep:specta", "dep:specta-typescript"]

[dependencies]
specta = { version = "2", optional = true }

[dev-dependencies]
specta-typescript = { version = "0.0.7", optional = true }
```

Command arg types and return types derive `specta::Type`:

```rust
#[derive(serde::Deserialize, specta::Type)]
pub struct ReadFileArgs {
    pub path: String,
    pub encoding: Option<String>,
}
```

A build script or test (run with `cargo test --features ts-gen`) emits `mozui-bindings.d.ts`:

```typescript
// mozui-bindings.d.ts — generated, do not edit

export interface ReadFileArgs {
  path: string;
  encoding?: string | null;
}

export interface AppVersion {
  major: number;
  minor: number;
  patch: number;
}

// Typed invoke wrappers
declare global {
  interface Window {
    mozui: {
      invoke(command: "read_file", args: ReadFileArgs): Promise<string>;
      invoke(command: "get_app_version", args?: never): Promise<AppVersion>;
      invoke(command: string, args?: unknown): Promise<unknown>;
      listen<T>(event: string, handler: (payload: T) => void): () => void;
      emit(event: string, payload?: unknown): void;
    };
  }
}
```

This gives full autocomplete and type safety in the renderer without any runtime overhead.

**Alternative: `typeshare`**

If `specta` is too heavy or has licensing concerns, `typeshare` from 1Password is a simpler alternative. Same pattern, different derive macro.

---

## Rust → JS Event Push

Rust can push events to the renderer at any time via `WebView::emit_to_js`. This is how Rust notifies the UI of changes without being polled.

```rust
impl WebView {
    pub fn emit_to_js(&self, event: &str, payload: impl serde::Serialize) -> anyhow::Result<()> {
        let msg = serde_json::json!({
            "__mozui": true,
            "type": "event",
            "name": event,
            "payload": payload,
        });
        Ok(self.webview.evaluate_script(&format!(
            "window.__mozui_dispatch({})",
            serde_json::to_string(&msg)?
        ))?)
    }
}
```

The JS bridge's `listen()` receives this. This enables patterns like:

```rust
// Rust: push a file change notification
webview.update(cx, |wv, _cx| {
    wv.emit_to_js("file_changed", &FileChangedPayload { path: path.clone() }).ok();
});
```

```typescript
// TypeScript: react to it
mozui.listen<{ path: string }>('file_changed', ({ path }) => {
  console.log('File changed:', path);
});
```

---

## Custom Protocol / Asset Serving

Custom protocols replace the need for a dev server in production. In development, traffic can be proxied to `http://localhost:PORT`.

### `mozui://` Protocol

Register `mozui` as the custom protocol for serving bundled app assets:

```rust
pub struct AssetServer {
    root: PathBuf,
    /// MIME type overrides: extension → content-type
    mime_overrides: HashMap<String, String>,
    /// Inject CSP header into HTML responses
    csp: Option<String>,
}

impl AssetServer {
    pub fn new(root: impl Into<PathBuf>) -> Self { ... }

    fn serve(&self, request: &http::Request<Vec<u8>>) -> http::Response<Cow<'static, [u8]>> {
        let path = request.uri().path();

        // Security: path traversal prevention
        let relative = path.trim_start_matches('/');
        let resolved = self.root.join(relative).canonicalize();

        let resolved = match resolved {
            Ok(p) if p.starts_with(&self.root) => p,
            _ => return self.not_found(),
        };

        let bytes = match std::fs::read(&resolved) {
            Ok(b) => b,
            Err(_) => return self.not_found(),
        };

        let mime = self.detect_mime(&resolved);
        let mut builder = http::Response::builder()
            .header("Content-Type", mime)
            .header("Cache-Control", "no-cache")
            .header("X-Content-Type-Options", "nosniff");

        // Inject CSP header on HTML responses
        if mime.starts_with("text/html") {
            if let Some(csp) = &self.csp {
                builder = builder.header("Content-Security-Policy", csp);
            }
        }

        builder.body(Cow::Owned(bytes)).unwrap()
    }
}
```

Path traversal is prevented by canonicalizing and checking the result is still under `self.root`. Requests that escape the root return 404 with no information.

### Dev Mode: Proxy to Vite / Other Dev Servers

```rust
WebViewBuilder::new()
    .url(if cfg!(debug_assertions) {
        "http://localhost:5173"
    } else {
        "mozui://localhost/index.html"
    })
    // In dev mode, allow localhost origin for IPC
    .capabilities(Capabilities {
        ipc_origins: if cfg!(debug_assertions) {
            OriginAllowlist::Custom(vec!["http://localhost:5173".into()])
        } else {
            OriginAllowlist::MozuiOnly
        },
        ..Default::default()
    })
```

This is the closest parallel to Electron's dev workflow — point at your existing web dev server in debug, serve bundled assets in release.

---

## Cross-Platform

Cross-platform is a first-class design constraint, not a retrofit. The three targets are macOS (WKWebView), Linux (WebKitGTK2), and Windows (WebView2). Each has meaningful behavioral differences that propagate through the IPC protocol, asset serving, and CSP.

### Custom Protocol URL Differences

This is the most impactful platform difference and must be handled at the foundation level:

| Platform | Custom scheme URL format | Reason |
|---|---|---|
| macOS | `mozui://localhost/path` | WKWebView native custom scheme |
| Linux | `mozui://localhost/path` | WebKitGTK native custom scheme |
| Windows | `https://mozui.localhost/path` | WebView2 maps custom schemes to `https://{name}.localhost/` |

The Windows difference is not cosmetic. It affects:
- **CSP**: `default-src 'self' mozui:` is wrong on Windows — must be `default-src 'self' https://mozui.localhost`
- **Origin validation**: the IPC `Origin` header on Windows will be `https://mozui.localhost`, not `mozui://localhost`
- **Asset URLs** hardcoded in HTML/CSS (e.g. `<img src="mozui://localhost/logo.png">`) will 404 on Windows

**Solution: platform-aware origin abstraction.** Add constants and a helper method:

```rust
/// The origin that the webview considers "self" — varies by platform.
#[cfg(target_os = "windows")]
pub const MOZUI_ORIGIN: &str = "https://mozui.localhost";
#[cfg(not(target_os = "windows"))]
pub const MOZUI_ORIGIN: &str = "mozui://localhost";

/// The URL scheme used for the asset protocol — varies by platform.
#[cfg(target_os = "windows")]
pub const MOZUI_SCHEME: &str = "https";
#[cfg(not(target_os = "windows"))]
pub const MOZUI_SCHEME: &str = "mozui";

impl WebView {
    /// Returns the asset URL for a given path, correct for the current platform.
    /// Use this in Rust code that constructs URLs for `load_url()` or `emit_to_js()`.
    /// Example: `WebView::asset_url("/index.html")` → `"mozui://localhost/index.html"` (macOS)
    ///                                              → `"https://mozui.localhost/index.html"` (Windows)
    pub fn asset_url(path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{MOZUI_ORIGIN}/{path}")
    }
}
```

The bridge JS also exposes this:

```javascript
// Injected alongside the bridge
window.__mozui_origin = "{MOZUI_ORIGIN}";  // filled in at build time by builder.rs
window.__mozui_scheme = "{MOZUI_SCHEME}";
```

**CSP must be platform-conditional.** The `DEFAULT_CSP_TEMPLATE` in `builder.rs` must use `MOZUI_ORIGIN` in `default-src` and `connect-src`, not a hardcoded scheme:

```rust
// builder.rs
#[cfg(target_os = "windows")]
const CSP_SELF_SRC: &str = "https://mozui.localhost";
#[cfg(not(target_os = "windows"))]
const CSP_SELF_SRC: &str = "mozui:";

const DEFAULT_CSP_TEMPLATE: &str = concat!(
    "default-src 'self' ", CSP_SELF_SRC, "; ",
    // ...
);
```

**`OriginAllowlist::MozuiOnly` must validate the platform-correct origin:**

```rust
impl OriginAllowlist {
    pub fn permits(&self, origin: Option<&str>) -> bool {
        match self {
            OriginAllowlist::MozuiOnly => {
                matches!(origin, Some(o) if o.starts_with(MOZUI_ORIGIN))
            }
            // ...
        }
    }
}
```

### IPC Backend Differences

| Platform | JS entry point | wry provides |
|---|---|---|
| macOS / Linux | `window.ipc.postMessage(msg)` | `window.ipc` global |
| Windows | `window.chrome.webview.postMessage(msg)` | Mirrors Chrome's WebView2 API |

The bridge JS already handles both branches in `_send()`. This is correct and sufficient — no Rust-side change needed.

### WebKitGTK on Linux

Linux requires WebKitGTK2 as a system dependency. This is a runtime dep, not a compile-time one. Users who build mozui apps for Linux must have `libwebkit2gtk-4.1-dev` (or equivalent) installed. Document this prominently.

GTK's message loop integrates with wry's event loop. The `with_ipc_handler` closure fires on the GTK main thread — same guarantees as macOS. The `Rc<RefCell<>>` pattern is correct on Linux.

### Feature Flag: `webview` Platform Gating

The `mozui-webview` crate should itself be gated — it does not make sense on pure headless targets. Add a workspace-level note that this crate is only enabled for macOS, Linux, and Windows targets, not WASM or headless test targets.

### Platform Test Matrix

Every CI run must test on all three platforms. The minimum viable test on each:
1. Build succeeds
2. `WebViewBuilder::new().html("<h1>ok</h1>").build(window, cx)` does not panic
3. A round-trip IPC invoke (send from JS, receive in Rust, respond) completes within 1s

The CI matrix should be defined in `.github/workflows/webview.yml` with `matrix: [macos-latest, ubuntu-latest, windows-latest]`.

---

## Navigation Security

### URL Allowlist

Navigation is blocked by default to external URLs. Allowed URL patterns use glob syntax:

```rust
WebViewBuilder::new()
    .allow_navigation("mozui://**")           // always implicitly allowed
    .allow_navigation("https://api.myapp.com/**")  // allow specific API domain
    // external navigations not in the list are blocked
```

The navigation handler:

```rust
fn navigation_handler(url: &str, allowlist: &[GlobPattern]) -> bool {
    // mozui:// is always allowed
    if url.starts_with("mozui://") {
        return true;
    }
    // In dev, localhost is allowed
    #[cfg(debug_assertions)]
    if url.starts_with("http://localhost") {
        return true;
    }
    // Check allowlist
    allowlist.iter().any(|pattern| pattern.matches(url))
}
```

### Navigation Events

Allowed navigations emit `NavigationEvent` into the mozui entity system (queue-drain pattern, same as IPC):

```rust
pub struct NavigationEvent {
    pub url: String,
}

pub struct NavigationBlockedEvent {
    pub url: String,
}
```

Blocked navigations emit `NavigationBlockedEvent` — useful for logging or showing UI feedback.

---

## WebViewBuilder

Full builder with all features integrated:

```rust
pub struct WebViewBuilder {
    // Content
    url: Option<String>,
    html: Option<String>,

    // Security
    capabilities: Capabilities,
    csp: Option<String>,
    navigation_allowlist: Vec<String>,

    // IPC
    command_registry: CommandRegistry,

    // Assets
    asset_server: Option<AssetServer>,

    // Appearance
    transparent: bool,
    user_agent: Option<String>,

    // Dev
    devtools: bool,  // only effective with `inspector` feature
    initialization_scripts: Vec<String>,
}

impl WebViewBuilder {
    pub fn new() -> Self { ... }

    // Content
    pub fn url(mut self, url: impl Into<String>) -> Self
    pub fn html(mut self, html: impl Into<String>) -> Self

    // Security
    pub fn capabilities(mut self, caps: Capabilities) -> Self
    pub fn csp(mut self, policy: impl Into<String>) -> Self
    pub fn allow_navigation(mut self, pattern: impl Into<String>) -> Self

    // IPC commands
    pub fn command<A, F>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        A: for<'de> serde::Deserialize<'de>,
        F: Fn(A, &mut WebView, &mut App) -> Result<serde_json::Value, IpcError> + 'static,

    // Asset server
    pub fn assets(mut self, root: impl Into<PathBuf>) -> Self
    pub fn assets_with(mut self, server: AssetServer) -> Self

    // Appearance
    pub fn transparent(mut self, t: bool) -> Self
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self

    // Dev
    #[cfg(feature = "inspector")]
    pub fn devtools(mut self, enabled: bool) -> Self

    pub fn initialization_script(mut self, script: impl Into<String>) -> Self

    // Build
    pub fn build(self, window: &mut Window, cx: &mut App) -> Entity<WebView>
}
```

### Build Flow

`build()` steps, in order:

1. Create `Rc<RefCell<VecDeque<IpcEnvelope>>>` as the IPC queue
2. Create `Rc<RefCell<VecDeque<NavigationEvent>>>` as the nav event queue
3. Build `wry::WebViewBuilder::new()`
4. Register `with_custom_protocol("mozui", ...)` if `asset_server` is set
5. Inject bridge JS via `with_initialization_script` (CSP nonce, bridge script, user scripts in order)
6. Wire `with_ipc_handler`: validate origin, check payload size, push valid envelopes to queue
7. Wire `with_navigation_handler`: check allowlist, push allowed navigations to nav queue
8. Apply transparency, user agent, devtools
9. Call `.build(&window)` using `Window`'s `HasWindowHandle` impl
10. Wrap in `WebView { ..., command_registry, ipc_queue, nav_queue }` and return as `Entity`

---

## Updated WebView Struct

```rust
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

    // Security
    capabilities: Capabilities,
}
```

### Render Drain

```rust
impl Render for WebView {
    fn render(&mut self, window: &mut mozui::Window, cx: &mut mozui::Context<Self>) -> impl IntoElement {
        // Drain IPC queue
        let envelopes: Vec<IpcEnvelope> = self.ipc_queue.borrow_mut().drain(..).collect();
        for envelope in envelopes {
            self.handle_ipc(envelope, cx);
        }

        // Drain navigation queues
        for url in self.nav_queue.borrow_mut().drain(..) {
            cx.emit(NavigationEvent { url });
        }
        for url in self.nav_blocked_queue.borrow_mut().drain(..) {
            cx.emit(NavigationBlockedEvent { url });
        }

        // ... existing layout/element code ...
    }
}
```

`handle_ipc` runs the registry lookup, deserialization, handler invocation, and response delivery via `evaluate_script`.

---

## Additional WebView Methods

```rust
impl WebView {
    // Navigation
    pub fn forward(&mut self) -> anyhow::Result<()>
    pub fn reload(&mut self) -> anyhow::Result<()>
    pub fn url(&self) -> anyhow::Result<String>
    pub fn load_url(&mut self, url: &str)  // already exists

    // JS interop
    pub fn emit_to_js(&self, event: &str, payload: impl serde::Serialize) -> anyhow::Result<()>

    // Requires ScriptCapability
    pub fn evaluate_script(&self, script: &str) -> anyhow::Result<()> {
        if !self.capabilities.script_execution {
            return Err(anyhow::anyhow!("script_execution capability not enabled"));
        }
        Ok(self.webview.evaluate_script(script)?)
    }
}
```

Note: `emit_to_js` uses `evaluate_script` internally but is not gated by `script_execution` — it only calls `window.__mozui_dispatch()` with a validated JSON payload, not arbitrary code. `evaluate_script` (raw) is what requires the capability.

---

## Event Emitters

```rust
impl EventEmitter<DismissEvent> for WebView {}
impl EventEmitter<IpcMessage> for WebView {}       // raw passthrough for consumers who want untyped
impl EventEmitter<NavigationEvent> for WebView {}
impl EventEmitter<NavigationBlockedEvent> for WebView {}
```

`IpcMessage` is a low-level escape hatch. Most consumers will use the command registry and typed `invoke()` rather than subscribing to raw `IpcMessage`.

---

## New Dependencies

```toml
[dependencies]
anyhow.workspace = true
mozui.workspace = true
wry = { version = "0.53.3", package = "lb-wry" }
serde = { workspace = true, features = ["derive"] }
serde_json.workspace = true
http.workspace = true

[features]
inspector = ["wry/devtools"]
ts-gen = ["dep:specta", "dep:specta-typescript"]

[dev-dependencies]
specta = { version = "2", optional = true, features = ["derive"] }
specta-typescript = { version = "0.0.7", optional = true }
```

---

## File Structure

The crate is still small enough to stay in `lib.rs`. Extract to modules if any single concern exceeds ~200 lines:

```
crates/mozui-webview/src/
  lib.rs               — WebView, WebViewElement, re-exports
  builder.rs           — WebViewBuilder (when it gets large)
  ipc.rs               — CommandRegistry, IpcEnvelope, IpcError, CommandResult
  security.rs          — Capabilities, OriginAllowlist, navigation_handler
  assets.rs            — AssetServer
  bridge/
    bridge.js          — Embedded JS bridge source (minified at build time)
    bridge.min.js      — Output of build script
```

The bridge JS is embedded at compile time via `include_str!("bridge/bridge.min.js")`.

---

## Implementation Phases

### Phase 1 — IPC Foundation
- `IpcEnvelope`, `IpcError`, `CommandRegistry`
- `WebViewBuilder` with `.command()`, `.url()`, `.html()`
- Origin validation, payload size limit
- Bridge JS injected via `initialization_script`
- Queue-drain in `render()`
- `emit_to_js` for Rust → JS push
- Typed event emitters

### Phase 2 — Security Layer
- `Capabilities` struct on builder
- CSP injection (default policy + override)
- Navigation allowlist with glob matching
- `NavigationBlockedEvent`
- `evaluate_script` gated behind capability

### Phase 3 — Asset Server
- `AssetServer` with path-traversal prevention
- `mozui://` custom protocol registration
- MIME detection
- CSP headers on HTML responses
- Dev proxy pattern documented

### Phase 4 — TypeScript Types
- `specta::Type` derives on built-in arg/return types
- `ts-gen` feature flag
- Type export test/binary
- Generated `.d.ts` with typed `invoke()` overloads
- Bridge JS types (`.d.ts` for `window.mozui`)

### Phase 5 — Async Commands
- `AsyncCommandConfig` with `max_concurrent`, `timeout`, `per_command_limit`
- `async_command()` builder method with `Entity<WebView>` + `AsyncApp` handler signature
- `pending_async: HashMap<String, Task<()>>` on `WebView`
- Rate limiting: immediate `RATE_LIMITED` response when at capacity
- `IpcEnvelope::Cancel { id }` variant and drain-loop handling
- Task dropped from `pending_async` on cancel or WebView drop

### Phase 6 — `@mozui/bridge` npm Package

**Why a package beyond the injected bridge:**

The injected bridge (`with_initialization_script`) works for apps where the renderer is a standalone HTML/JS bundle. But for monorepo setups — where the renderer is a TypeScript project with its own `package.json`, imports, and bundler — consumers want to `import { invoke, listen } from "@mozui/bridge"` rather than rely on `window.mozui` being populated at runtime.

The package provides:
- Full TypeScript types for `invoke`, `listen`, `emit` with proper generic signatures
- An `isBridge()` guard (`typeof window !== 'undefined' && !!window.mozui`) that works in SSR/test environments where the bridge isn't present
- Re-exports of the generated types from `specta` (or manually curated) for common arg/return shapes
- A mock bridge for unit testing renderer components without a running Rust backend

**Package structure:**

```
packages/mozui-bridge/
  package.json
  src/
    index.ts          — exports invoke, listen, emit, isBridge
    types.ts          — MozuiError, IpcOptions, UnlistenFn
    mock.ts           — MockBridge for testing
  tsconfig.json
  build.config.ts     — bun build entry
```

**`src/index.ts`:**

```typescript
export type UnlistenFn = () => void;

export interface MozuiError extends Error {
  code: string;
}

// Type guard: returns true if running inside a mozui webview
export function isBridge(): boolean {
  return typeof window !== 'undefined'
    && typeof window.__mozui_dispatch === 'function';
}

// Forward to the injected bridge. The bridge is injected before any JS runs,
// so window.mozui is always present when this module is loaded inside the webview.
export function invoke<T = unknown>(command: string, args?: unknown): Promise<T> {
  if (!isBridge()) {
    return Promise.reject(new Error('[mozui] invoke() called outside a mozui webview'));
  }
  return window.mozui.invoke<T>(command, args);
}

export function listen<T = unknown>(
  event: string,
  handler: (payload: T) => void
): UnlistenFn {
  if (!isBridge()) return () => {};
  return window.mozui.listen<T>(event, handler);
}

export function emit(event: string, payload?: unknown): void {
  if (!isBridge()) return;
  window.mozui.emit(event, payload);
}
```

**`src/mock.ts`** (for testing renderer code without Rust):

```typescript
export function installMockBridge(handlers: Record<string, (args: unknown) => unknown> = {}) {
  const listeners = new Map<string, Set<Function>>();

  window.mozui = {
    invoke: async (command: string, args?: unknown) => {
      const handler = handlers[command];
      if (!handler) throw Object.assign(new Error(`No mock for: ${command}`), { code: 'UNKNOWN_COMMAND' });
      return handler(args);
    },
    listen: (event: string, handler: Function) => {
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event)!.add(handler);
      return () => listeners.get(event)?.delete(handler);
    },
    emit: () => {},
  };

  // Returns a helper to push mock events into listeners
  return {
    push: (event: string, payload: unknown) => {
      listeners.get(event)?.forEach(h => h(payload));
    }
  };
}
```

**Publishing strategy:** Publish to npm as `@mozui/bridge`. Version is kept in sync with `mozui-webview` crate version via a `package.json` script that reads from `Cargo.toml`. The package is built with `bun build` (see Bun section below).

**Deliverables:**
- `@mozui/bridge` package with full TypeScript types
- Mock bridge for testing
- `isBridge()` guard for SSR-safe usage
- Version sync tooling between `Cargo.toml` and `package.json`
- CI check: bridge JS and npm package types must be consistent (compile-time test)

---

## Resolved Design Decisions

### 1. Async Commands: Security-First Design

**What Tauri does:** Tauri marks async commands with `#[tauri::command] async fn`. These run on a `tauri::async_runtime` (Tokio) thread pool. Invocation tracking lives in a `HashMap<InvocationId, oneshot::Sender>`. There is no built-in bound on concurrent invocations — a fast JS caller can enqueue unbounded work. Tauri's security focus is on *what* commands can do (capability/permission system), not on *how many* can run concurrently.

**Our approach — security first, DX second:**

The async path has two distinct threat vectors Tauri doesn't address:

1. **Command flooding:** JS can call `invoke("slow_cmd")` in a tight loop, enqueuing hundreds of pending tasks before any complete. Without a bound, this exhausts memory and degrades the app.
2. **Runaway tasks:** A poorly written command handler may never complete, leaking the pending entry forever.

**Architecture:**

```rust
pub struct AsyncCommandConfig {
    /// Maximum number of in-flight invocations across all commands. Default: 32.
    /// Invocations beyond this limit receive RATE_LIMITED immediately.
    pub max_concurrent: usize,
    /// Per-invocation Rust-side timeout. Default: 30s.
    /// JS has its own 30s timeout — this is the Rust backstop that frees the task slot.
    pub timeout: std::time::Duration,
    /// Per-command concurrency limit. Default: None (uses max_concurrent).
    pub per_command_limit: Option<HashMap<&'static str, usize>>,
}
```

Async handlers are registered separately from sync ones, using `async_command()` on the builder:

```rust
.async_command("fetch_data", |args: FetchArgs, webview: Entity<WebView>, async_cx: AsyncApp| async move {
    let result = do_async_work(args).await;
    async_cx.update(|cx| {
        webview.update(cx, |wv, _cx| {
            wv.send_response(&args_id, Ok(serde_json::to_value(result)?));
        });
    }).ok();
})
```

The handler receives `Entity<WebView>` (not `&mut WebView`) and `AsyncApp` — the mozui async context — rather than the synchronous `&mut WebView`. This is the same pattern mozui uses for all async entity work. The entity update delivers the response back through the queue-drain path on the next frame.

**Pending task tracking on `WebView`:**

```rust
pub struct WebView {
    // ... existing fields ...
    pending_async: HashMap<String, mozui::Task<()>>,  // id → task handle
    async_config: AsyncCommandConfig,
}
```

Before spawning, check `pending_async.len() < async_config.max_concurrent`. If at capacity: respond with `IpcError { code: "RATE_LIMITED", ... }` immediately. The task handle is stored in `pending_async` so it can be cancelled if the WebView drops, or if the JS side times out and we receive a cancellation message.

**Cancellation protocol (Phase 5 extension):**

Add an `IpcEnvelope::Cancel { id }` variant. When the JS-side timeout fires and the request is dropped from `_pending`, the bridge sends a cancel envelope. The Rust drain loop drops the task from `pending_async`, which cancels the future via mozui's task cancellation.

This means a misbehaving JS caller gets rate-limited and their timed-out requests are cleaned up on both sides — no resource leak.

---

### 2. Event Emitter Architecture: Long-Term Stability

**Decision: separate `EventEmitter<T>` impls per event type. This is the stable choice.**

**Why not a unified `WebViewEvent` enum:**

An enum is a closed sum type. Every time a new variant is added, all existing `match` arms in downstream code need a wildcard or a new arm. In practice:
- Library updates adding a new `WebViewEvent::HotKeyPressed` variant silently compiles with `_ => {}` arms — subscribers miss events without any warning
- Or it breaks compilation at every match site if there's no wildcard — a noisy, high-friction upgrade path
- The coupling is also wrong: a subscriber watching for `NavigationEvent` has no reason to know about `IpcMessage`

**Why separate impls are stable:**

Adding `impl EventEmitter<NewEventType> for WebView {}` is purely additive. No existing subscriber code is touched. Subscribers who want the new event opt in with a new `cx.subscribe()` call.

**The DX cost:** consumers who want multiple event types write multiple subscriptions. This is acceptable — in practice they live in a parent view's `new()` call and are cleanly grouped.

**For debugging/observability:** Add a `WebViewEventLog` entity (behind a `debug-log` feature) that subscribes to all event types and maintains an ordered timeline. This is purely opt-in and doesn't pollute the primary API:

```rust
#[cfg(feature = "debug-log")]
pub struct WebViewEventLog {
    entries: Vec<(std::time::Instant, WebViewLogEntry)>,
}
```

This gives the "unified view of everything" for debugging without embedding that concept in the production API surface.

---

### 3. IPC Message Format: Security and DX Alignment

**Decision: structured JSON throughout. `serde_json::Value` for the untyped escape hatch, never raw `String`.**

Security argument: raw `String` passthrough means consumers inevitably write ad-hoc string parsing — regex, `split()`, `starts_with()`. This is the substrate for injection vulnerabilities. A `serde_json::Value` is already a validated tree; it cannot contain structurally invalid JSON. Consumers who need "untyped" access still get a structured value they index rather than parse.

DX argument: the typed command path (primary) is `serde_json::Value` under the hood anyway. The untyped `IpcMessage` escape hatch returning `Value` is consistent with that. Changing it to `String` later would be a breaking change; `Value` is the right type to start with.

**The `IpcMessage` raw event:**

```rust
pub struct IpcMessage {
    pub command: String,
    pub args: serde_json::Value,   // NOT String — structured, never raw text
}
```

Consumers who subscribe to `IpcMessage` get validated structure. They can do `args["field"].as_str()` without any parsing. Serde's `from_value::<T>()` works trivially on top if they want typed extraction.

**No `String` escape hatch anywhere in the public API.** If a consumer genuinely needs the raw bytes of a large binary payload, they should base64-encode it in JS and decode in the Rust handler — not pass raw binary through IPC.

---

### 4. npm Package — Moved to Phase 6

See Implementation Phases below.

---

### 5. Cross-Platform: First-Class Priority

Cross-platform is not an afterthought — it is a first-class constraint that shapes the design. See the **Cross-Platform** section below for full details.

---

### 6. Hot Reload in Dev

Confirmed: when pointing at a Vite/Bun dev server, HMR works via the dev server's WebSocket. No mozui-specific work needed. The only constraint is that `Capabilities::ipc_origins` must include the dev server origin — handled by the dev mode capabilities pattern documented in the Custom Protocol section. This is intentional: the same security system that protects production also explicitly opts in to localhost during development, making the relaxation visible and auditable in code.

---

## Bun Integration

Bun is the recommended JS runtime for mozui-webview's toolchain. It is never a runtime dependency of the Rust app — it is a build-time and development-time accelerator.

### Why Bun Over Node/npm

- **Speed**: `bun build` is 10-100x faster than webpack/rollup/vite for bundling. In a `cargo build` that shells out to bundle the renderer, this matters.
- **Native TypeScript**: no `tsc` step, no `ts-loader`, no separate transpile pass. Bun executes `.ts` files directly.
- **Single binary**: one tool for package management, bundling, testing, and running scripts. Reduces toolchain surface area.
- **Bun.serve()**: a high-performance HTTP dev server in one line — cleaner than Vite for simple cases.
- **`bun --watch`**: re-runs scripts on file change, simpler than nodemon.
- **`bun build --compile`**: produces a single-file executable that bundles a JS app with the Bun runtime — useful for shipping a standalone renderer tooling binary.

### Bridge JS Minification

`build.rs` detects whether `bun` is in `PATH` and uses it for bridge minification if available, falling back to the `minify-js` crate:

```rust
// build.rs
fn minify_bridge(source: &[u8]) -> Vec<u8> {
    // Try bun first — better minification, handles modern JS features
    if let Ok(bun) = std::process::Command::new("bun")
        .args(["build", "--minify", "--no-splitting", "/dev/stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        use std::io::Write;
        if let Ok(output) = bun.stdin.as_ref()
            .map(|mut s| { s.write_all(source).ok(); })
            .and_then(|_| Some(bun.wait_with_output()))
            .and_then(|r| r.ok())
        {
            if output.status.success() && !output.stdout.is_empty() {
                return output.stdout;
            }
        }
    }

    // Fallback: pure-Rust minifier (always available, no external dep)
    let mut out = Vec::new();
    minify_js::minify(
        minify_js::Session::new(),
        minify_js::TopLevelMode::Module,
        source,
        &mut out,
    )
    .expect("bridge.js minification failed");
    out
}
```

### Renderer Bundling via `bun build`

For apps that want to embed the renderer in the binary (ship a single executable with no external assets), add a `bun build` step to `build.rs`:

```rust
// In build.rs, after the bridge minification step:
fn bundle_renderer(renderer_dir: &Path, out_dir: &Path) {
    let status = std::process::Command::new("bun")
        .args([
            "build",
            "src/main.ts",
            "--outdir", out_dir.to_str().unwrap(),
            "--target", "browser",
            "--minify",
            "--sourcemap=none",
        ])
        .current_dir(renderer_dir)
        .status()
        .expect("bun not found — install bun to build embedded renderer");

    assert!(status.success(), "bun build failed");
    println!("cargo:rerun-if-changed={}", renderer_dir.display());
}
```

The output is then embedded with `rust-embed` (already a workspace dep):

```rust
#[derive(rust_embed::Embed)]
#[folder = "$OUT_DIR/renderer/"]
struct RendererAssets;
```

And served from `AssetServer` via an `EmbeddedAssetServer` variant that reads from `RendererAssets` instead of the filesystem. This produces a **single Rust binary with the entire web UI embedded** — the Tauri model, with Bun doing the bundling.

### Dev Server with Bun

Replace Vite with Bun's dev server for projects that don't need Vite's plugin ecosystem:

```typescript
// renderer/dev.ts — run with: bun --watch dev.ts
Bun.serve({
  port: 5173,
  fetch(req) {
    // Serve static files from dist/
    const url = new URL(req.url);
    const file = Bun.file(`./src${url.pathname === '/' ? '/index.html' : url.pathname}`);
    return new Response(file);
  }
});
console.log("Dev server: http://localhost:5173");
```

```bash
bun --hot dev.ts   # hot-reload the server on file changes
```

### Type Generation with Bun

Running the `gen-ts` binary and piping output to the renderer project:

```bash
# In package.json scripts:
"gen-types": "cargo run -p mozui-webview --bin gen-ts --features ts-gen | bun run prettier --parser typescript > src/mozui-bindings.d.ts"
```

Or via a Bun script that handles the cargo invocation:

```typescript
// scripts/gen-types.ts
import { $ } from "bun";
await $`cargo run -p mozui-webview --bin gen-ts --features ts-gen`;
// Bun's $ shell integration handles the pipe and file write
```

### `@mozui/bridge` Package Build

The npm package (Phase 6) is built with `bun build`:

```json
{
  "name": "@mozui/bridge",
  "scripts": {
    "build": "bun build src/index.ts --outdir dist --target browser --format esm --minify && bun build src/index.ts --outdir dist --target browser --format cjs --minify",
    "test": "bun test",
    "types": "bun x tsc --emitDeclarationOnly --declarationDir dist/types"
  }
}
```

This produces both ESM and CJS builds plus `.d.ts` declarations — the full package consumers expect.

### Bun Test for Mock Bridge

The mock bridge (`src/mock.ts`) enables fast renderer unit tests that run entirely in Bun, no webview required:

```typescript
// tests/invoke.test.ts
import { test, expect, beforeEach } from "bun:test";
import { installMockBridge } from "@mozui/bridge/mock";
import { invoke } from "@mozui/bridge";

beforeEach(() => {
  installMockBridge({
    "get_version": () => "1.0.0",
    "read_file": ({ path }: { path: string }) => ({ contents: "hello", size_bytes: 5 }),
  });
});

test("invoke get_version", async () => {
  const version = await invoke<string>("get_version");
  expect(version).toBe("1.0.0");
});

test("invoke with unknown command rejects", async () => {
  await expect(invoke("does_not_exist")).rejects.toMatchObject({ code: "UNKNOWN_COMMAND" });
});
```

Run with `bun test` — no cargo, no webview, no OS dependencies. These run in CI in milliseconds.

---

## Implementation

### Step 1: Module Structure

Split `lib.rs` now rather than later. The new types add enough mass that a flat file becomes unwieldy fast. Final layout:

```
crates/mozui-webview/src/
  lib.rs          — public re-exports, WebView struct, WebViewElement, Render impl
  builder.rs      — WebViewBuilder
  ipc.rs          — IpcEnvelope, IpcError, CommandRegistry, CommandResult
  security.rs     — Capabilities, OriginAllowlist, CommandAllowlist, validation fns
  assets.rs       — AssetServer
  bridge/
    bridge.js     — human-readable bridge source
    bridge.min.js — output of build.rs minification (gitignored or committed)
build.rs          — minify bridge.js into bridge.min.js at compile time
```

`lib.rs` declares the modules and re-exports the public surface:

```rust
mod builder;
mod ipc;
mod security;
mod assets;

pub use builder::WebViewBuilder;
pub use ipc::{CommandResult, IpcError, IpcMessage};
pub use security::{Capabilities, CommandAllowlist, OriginAllowlist};
pub use assets::AssetServer;
pub use events::{NavigationEvent, NavigationBlockedEvent, JsEmitEvent};
```

---

### Step 2: `ipc.rs` — Core Types

The key design decision here is **type erasure for command handlers**. Each handler is generic over its args type at registration time, but stored as a `Box<dyn Fn(...)>` that captures the deserialization step inside the closure. This means `CommandRegistry` itself is not generic.

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// The wire format arriving from JS. Uses serde's internally-tagged enum
/// so we can dispatch on `type` without a wrapper.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum IpcEnvelope {
    Invoke {
        id: String,
        command: String,
        #[serde(default)]
        args: serde_json::Value,
    },
    /// JS calling mozui.emit() — JS-originated event, not a command invocation.
    Emit {
        name: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
}

/// Validates that the incoming JSON is a mozui message before parsing the envelope.
/// Checks for `__mozui: true` sentinel to reject non-bridge messages early.
pub(crate) fn parse_envelope(raw: &str) -> Option<IpcEnvelope> {
    // Quick sentinel check without full parse
    if !raw.contains("\"__mozui\"") {
        return None;
    }
    // Parse into a generic Value first, check sentinel
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    if v.get("__mozui") != Some(&serde_json::Value::Bool(true)) {
        return None;
    }
    // Now deserialize into the typed envelope
    serde_json::from_value(v).ok()
}

#[derive(Debug, Serialize)]
pub struct IpcError {
    pub code: &'static str,
    pub message: String,
}

impl IpcError {
    pub const UNKNOWN_COMMAND: &'static str = "UNKNOWN_COMMAND";
    pub const INVALID_ARGS: &'static str = "INVALID_ARGS";
    pub const PAYLOAD_TOO_LARGE: &'static str = "PAYLOAD_TOO_LARGE";
    pub const INTERNAL: &'static str = "INTERNAL";
    pub const RATE_LIMITED: &'static str = "RATE_LIMITED";
    // Note: no UNAUTHORIZED constant — allowlist-denied commands return UNKNOWN_COMMAND
    // to prevent timing oracle attacks that reveal which commands exist but are restricted.

    /// Used for both unregistered commands AND allowlist-denied commands.
    /// Returning the same code prevents callers from enumerating the registered
    /// command surface by probing command names and observing which error they get.
    pub fn unknown_command(_name: &str) -> Self {
        // The name parameter is intentionally ignored in the response —
        // confirming the name back leaks that we parsed it.
        Self { code: Self::UNKNOWN_COMMAND, message: "unknown command".to_string() }
    }

    pub fn invalid_args(err: impl std::fmt::Display) -> Self {
        Self { code: Self::INVALID_ARGS, message: err.to_string() }
    }

    /// Creates an internal error. The real cause is NOT included in the message —
    /// it must be logged server-side via tracing::error! before calling this.
    /// Sending Rust error strings to JS is an information disclosure vulnerability.
    pub fn internal() -> Self {
        Self { code: Self::INTERNAL, message: "internal error".to_string() }
    }

    pub fn rate_limited() -> Self {
        Self { code: Self::RATE_LIMITED, message: "too many concurrent commands".to_string() }
    }
}

impl From<anyhow::Error> for IpcError {
    fn from(e: anyhow::Error) -> Self {
        // Log the real error here, return sanitized response to renderer
        tracing::error!("IPC command error: {e:?}");
        Self::internal()
    }
}

/// The outcome of a command handler invocation.
pub enum CommandResult {
    /// Handler produced a result synchronously. Rust will call __mozui_dispatch immediately.
    Immediate(Result<serde_json::Value, IpcError>),
    /// Handler will deliver the response asynchronously (Phase 5).
    /// The id is passed so the handler can resolve it later.
    Deferred,
}

/// A raw IPC message exposed as a mozui event for consumers who want untyped access.
pub struct IpcMessage {
    pub command: String,
    pub args: serde_json::Value,
}

// The concrete handler type. Stored as a trait object.
// Takes raw JSON so the registry is not generic — deserialization is captured
// inside the Box::new closure during registration.
pub(crate) type CommandFn = Box<
    dyn Fn(
        serde_json::Value,
        &mut crate::WebView,
        &mut mozui::Context<crate::WebView>,
    ) -> CommandResult + 'static,
>;

#[derive(Default)]
pub struct CommandRegistry {
    pub(crate) handlers: HashMap<String, CommandFn>,
}

impl CommandRegistry {
    /// Register a typed command handler.
    ///
    /// The generic `A` is the args type. Deserialization from `serde_json::Value`
    /// is captured inside the closure — if JS sends the wrong shape, the handler
    /// receives an `INVALID_ARGS` error without ever executing.
    pub fn register<A, F>(&mut self, name: impl Into<String>, handler: F)
    where
        A: for<'de> serde::Deserialize<'de> + 'static,
        F: Fn(A, &mut crate::WebView, &mut mozui::Context<crate::WebView>)
               -> Result<serde_json::Value, IpcError>
           + 'static,
    {
        let name = name.into();
        let boxed: CommandFn = Box::new(move |raw_args, webview, cx| {
            match serde_json::from_value::<A>(raw_args) {
                Ok(args) => CommandResult::Immediate(handler(args, webview, cx)),
                Err(e) => CommandResult::Immediate(Err(IpcError::invalid_args(e))),
            }
        });
        self.handlers.insert(name, boxed);
    }
}
```

**Why the two-step parse in `parse_envelope`:** wry's IPC handler fires for every `window.ipc.postMessage()` call, including any third-party JS that might accidentally call it. The sentinel check short-circuits without a full parse for non-bridge messages. If the sentinel is present but the envelope is malformed, `None` is returned and the message is silently dropped.

---

### Step 3: `security.rs` — Validation Functions

```rust
use http::Request;

#[derive(Clone, Debug)]
pub struct Capabilities {
    pub commands: CommandAllowlist,
    /// Allow raw `evaluate_script` calls from Rust code. Default: false.
    /// `emit_to_js` is NOT gated by this — it calls __mozui_dispatch with
    /// validated JSON only, not arbitrary code.
    pub script_execution: bool,
    /// Allow navigations to external (non-mozui://) URLs not in the allowlist.
    pub external_navigation: bool,
    /// Maximum IPC payload size in bytes. Default: 1MB.
    pub max_payload_bytes: usize,
    /// Which origins may send IPC messages. Default: mozui:// only.
    pub ipc_origins: OriginAllowlist,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            commands: CommandAllowlist::All,
            script_execution: false,
            external_navigation: false,
            max_payload_bytes: 1024 * 1024,
            ipc_origins: OriginAllowlist::MozuiOnly,
        }
    }
}

#[derive(Clone, Debug)]
pub enum CommandAllowlist {
    /// All registered commands are callable. Use in development.
    All,
    /// Only these command names are callable. All others return UNKNOWN_COMMAND
    /// (same code as unregistered commands — prevents timing oracle enumeration).
    Only(Vec<&'static str>),
}

impl CommandAllowlist {
    pub fn permits(&self, command: &str) -> bool {
        match self {
            CommandAllowlist::All => true,
            CommandAllowlist::Only(list) => list.contains(&command),
        }
    }
}

#[derive(Clone, Debug)]
pub enum OriginAllowlist {
    /// Only `mozui://` scheme origins. Production default.
    MozuiOnly,
    /// A fixed set of allowed origin strings (exact match against the Origin header).
    /// Example: `["http://localhost:5173"]` for Vite dev server.
    Custom(Vec<String>),
    /// Any origin. Use only for fully trusted embedded content.
    Any,
}

impl OriginAllowlist {
    /// `origin` is the raw value of the HTTP `Origin` header, if present.
    pub fn permits(&self, origin: Option<&str>) -> bool {
        match self {
            OriginAllowlist::Any => true,
            OriginAllowlist::MozuiOnly => {
                matches!(origin, Some(o) if o.starts_with("mozui://"))
            }
            OriginAllowlist::Custom(allowed) => {
                match origin {
                    Some(o) => allowed.iter().any(|a| a == o),
                    // No Origin header → reject unless explicitly allowing it
                    None => false,
                }
            }
        }
    }
}

/// Full validation gate for an incoming IPC request.
/// Returns an IpcError variant string on failure, or Ok(()) to proceed.
pub(crate) fn validate_ipc_request(
    request: &Request<String>,
    caps: &Capabilities,
) -> Result<(), &'static str> {
    let origin = request
        .headers()
        .get(http::header::ORIGIN)
        .and_then(|v| v.to_str().ok());

    if !caps.ipc_origins.permits(origin) {
        // Do not respond — silently drop. Returning an error would reveal
        // the bridge exists to a foreign origin doing recon.
        return Err("origin denied — drop silently");
    }

    if request.body().len() > caps.max_payload_bytes {
        return Err("payload too large");
    }

    Ok(())
}

/// Returns true if navigation to `url` should proceed.
/// Called from wry's navigation handler — must be fast and non-allocating where possible.
pub(crate) fn navigation_allowed(url: &str, allowlist: &[String], dev_mode: bool) -> bool {
    // Explicit deny: dangerous URI schemes must be blocked before any allowlist check.
    // These could execute code or load arbitrary data regardless of other restrictions.
    if url.starts_with("javascript:")
        || url.starts_with("data:")
        || url.starts_with("vbscript:")
        || url.starts_with("file:")
    {
        return false;
    }

    // Internal scheme: always allow
    if url.starts_with("mozui://")
        || url.starts_with("https://mozui.localhost")  // Windows WebView2 form
        || url == "about:blank"
    {
        return true;
    }

    // Dev mode: localhost allowed. This is a data decision (explicit Capabilities field),
    // not a compiler flag — the same binary can run in dev or prod mode at runtime.
    if dev_mode && (url.starts_with("http://localhost") || url.starts_with("http://127.0.0.1")) {
        return true;
    }

    // Check user-provided glob patterns
    allowlist.iter().any(|pattern| glob_matches(pattern, url))
}

/// Minimal glob matching used for navigation allowlists.
/// `*`  — matches any character sequence that doesn't cross a `/`
/// `**` — matches any character sequence including `/`
/// All other characters are matched literally.
pub(crate) fn glob_matches(pattern: &str, value: &str) -> bool {
    // Convert pattern to a simple NFA walk
    fn inner(pat: &[u8], val: &[u8]) -> bool {
        match (pat, val) {
            ([], []) => true,
            ([], _) => false,
            ([b'*', b'*', rest @ ..], _) => {
                // ** matches zero or more of anything
                if rest.is_empty() { return true; }
                (0..=val.len()).any(|i| inner(rest, &val[i..]))
            }
            ([b'*', rest @ ..], _) => {
                // * matches zero or more non-slash characters
                let limit = val.iter().position(|&c| c == b'/').unwrap_or(val.len());
                (0..=limit).any(|i| inner(rest, &val[i..]))
            }
            ([p, pr @ ..], [v, vr @ ..]) if p == v => inner(pr, vr),
            _ => false,
        }
    }
    inner(pattern.as_bytes(), value.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_exact() {
        assert!(glob_matches("https://api.example.com/v1/users", "https://api.example.com/v1/users"));
        assert!(!glob_matches("https://api.example.com/v1/users", "https://api.example.com/v1/posts"));
    }

    #[test]
    fn glob_single_star() {
        assert!(glob_matches("https://api.example.com/v1/*", "https://api.example.com/v1/users"));
        assert!(!glob_matches("https://api.example.com/v1/*", "https://api.example.com/v1/users/123"));
    }

    #[test]
    fn glob_double_star() {
        assert!(glob_matches("https://api.example.com/**", "https://api.example.com/v1/users/123"));
        assert!(!glob_matches("https://api.example.com/**", "https://other.com/anything"));
    }
}
```

---

### Step 4: Bridge JS and `build.rs`

**`crates/mozui-webview/src/bridge/bridge.js`** (human-readable source):

```javascript
// mozui renderer bridge — injected before any page code runs at the platform level.
// This script is NOT subject to CSP. It runs with the same trust as the binary.
//
// Security properties maintained here:
//   1. _pending and _listeners are closure-scoped — unreachable from page code
//   2. window.__mozui_dispatch is non-writable and non-configurable
//   3. window.mozui is frozen — methods cannot be replaced or deleted
//   4. Double-initialization is guarded against
(function () {
  'use strict';

  // Guard against double-injection (e.g. if the page somehow reloads the script)
  if (Object.getOwnPropertyDescriptor(window, '__mozui_dispatch')) return;

  const _pending = new Map();   // id -> { resolve, reject }
  const _listeners = new Map(); // event name -> Set<handler>

  // Internal send — platform-correct IPC entry point.
  // macOS/Linux: window.ipc   |   Windows WebView2: window.chrome.webview
  function _send(json) {
    if (window.ipc) {
      window.ipc.postMessage(json);
    } else if (window.chrome && window.chrome.webview) {
      window.chrome.webview.postMessage(json);
    }
    // Silently no-op if neither is present (non-webview environment, e.g. tests)
  }

  // Dispatch function called by Rust via evaluate_script("window.__mozui_dispatch(JSON)").
  // Defined as non-writable and non-configurable — page code cannot overwrite or delete it.
  Object.defineProperty(window, '__mozui_dispatch', {
    value: function (rawJson) {
      var msg;
      try { msg = JSON.parse(rawJson); } catch (e) { return; }
      if (!msg || !msg.__mozui) return;

      if (msg.type === 'response') {
        var pending = _pending.get(msg.id);
        if (!pending) return;
        _pending.delete(msg.id);
        if (msg.ok) {
          pending.resolve(msg.payload);
        } else {
          var err = new Error(msg.error ? msg.error.message : 'IPC error');
          err.code = msg.error ? msg.error.code : 'INTERNAL';
          pending.reject(err);
        }
      } else if (msg.type === 'event') {
        var set = _listeners.get(msg.name);
        if (set) set.forEach(function (h) { h(msg.payload); });
      }
    },
    writable: false,
    configurable: false,
    enumerable: false,
  });

  // Public API object — built before freezing so methods share closure scope
  var mozuiApi = {
    /**
     * Invoke a Rust command and await the typed result.
     * Rejects with { code, message } on error or after 30s timeout.
     */
    invoke: function (command, args) {
      return new Promise(function (resolve, reject) {
        var id = (typeof crypto !== 'undefined' && crypto.randomUUID)
          ? crypto.randomUUID()
          : Math.random().toString(36).slice(2) + Date.now().toString(36);

        _pending.set(id, { resolve: resolve, reject: reject });

        _send(JSON.stringify({
          __mozui: true, id: id, type: 'invoke',
          command: command,
          args: args !== undefined ? args : null
        }));

        // Timeout: prevents _pending from leaking if Rust never responds.
        // The bridge also sends a cancel envelope so Rust can free the task slot.
        setTimeout(function () {
          if (_pending.has(id)) {
            _pending.delete(id);
            // Notify Rust to cancel the in-flight task
            _send(JSON.stringify({ __mozui: true, type: 'cancel', id: id }));
            var err = new Error('mozui invoke timed out: ' + command);
            err.code = 'TIMEOUT';
            reject(err);
          }
        }, 30000);
      });
    },

    /**
     * Listen for an event pushed from Rust.
     * Returns an unlisten function — call it to remove the handler.
     */
    listen: function (event, handler) {
      if (!_listeners.has(event)) _listeners.set(event, new Set());
      _listeners.get(event).add(handler);
      return function () {
        var set = _listeners.get(event);
        if (set) set.delete(handler);
      };
    },

    /**
     * Emit a JS-originated event to Rust.
     * Rust-side: fires JsEmitEvent into the mozui entity system.
     * Only event names in Capabilities::js_emit_allowlist are processed;
     * all others are silently dropped on the Rust side.
     */
    emit: function (event, payload) {
      _send(JSON.stringify({
        __mozui: true, type: 'emit',
        name: event,
        payload: payload !== undefined ? payload : null
      }));
    }
  };

  // Freeze the API object and define it as non-writable/non-configurable on window.
  // Any attempt to overwrite window.mozui from page code will silently fail in
  // non-strict mode, or throw a TypeError in strict mode.
  Object.freeze(mozuiApi);
  Object.defineProperty(window, 'mozui', {
    value: mozuiApi,
    writable: false,
    configurable: false,
    enumerable: true,
  });
})();
```

**`crates/mozui-webview/build.rs`**:

The build script minifies `bridge.js` into `bridge.min.js` using the `minify-js` crate. This keeps the embedded JS small without pulling Node.js into the build chain.

```rust
// build.rs
fn main() {
    let bridge_src = std::path::PathBuf::from("src/bridge/bridge.js");
    let bridge_out = std::path::PathBuf::from("src/bridge/bridge.min.js");

    println!("cargo:rerun-if-changed=src/bridge/bridge.js");

    let source = std::fs::read(&bridge_src)
        .expect("bridge.js not found — run from crates/mozui-webview/");

    // minify-js: zero-dependency JS minifier in pure Rust
    let mut output = Vec::new();
    minify_js::minify(
        minify_js::Session::new(),
        minify_js::TopLevelMode::Module,
        &source,
        &mut output,
    )
    .expect("bridge.js minification failed");

    // Only write if content changed, to avoid spurious rebuilds
    let existing = std::fs::read(&bridge_out).unwrap_or_default();
    if existing != output {
        std::fs::write(&bridge_out, &output).expect("failed to write bridge.min.js");
    }
}
```

Add to `Cargo.toml`:
```toml
[build-dependencies]
minify-js = "0.6"
```

In `lib.rs`, embed at compile time:
```rust
const BRIDGE_JS: &str = include_str!("bridge/bridge.min.js");
```

**Windows IPC note:** On Windows, wry uses `window.chrome.webview.postMessage` instead of `window.ipc.postMessage`. The bridge handles both branches above. This is the only platform-specific behavior in the bridge — everything else is uniform.

---

### Step 5: `assets.rs` — Asset Server

```rust
use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
};
use http::{Request, Response};

pub struct AssetServer {
    /// Absolute, canonicalized root directory. All served paths must be under this.
    root: PathBuf,
    /// CSP header value to inject into HTML responses. None = no header.
    pub csp: Option<String>,
    /// Per-extension MIME overrides. Key: lowercase extension without dot. Value: content-type.
    pub mime_overrides: HashMap<String, String>,
}

impl AssetServer {
    /// Create an asset server rooted at `root`.
    /// Panics at build time if `root` does not exist or cannot be canonicalized —
    /// a misconfigured asset root is a programmer error, not a runtime error.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into().canonicalize()
            .expect("AssetServer root must be an existing directory");
        Self {
            root,
            csp: None,
            mime_overrides: HashMap::new(),
        }
    }

    pub fn with_csp(mut self, csp: impl Into<String>) -> Self {
        self.csp = Some(csp.into());
        self
    }

    pub fn with_mime(mut self, ext: impl Into<String>, content_type: impl Into<String>) -> Self {
        self.mime_overrides.insert(ext.into(), content_type.into());
        self
    }

    /// Serve a wry custom protocol request.
    /// The `request` URI path is resolved relative to `self.root`.
    ///
    /// Security invariant: the resolved path is canonicalized and checked to be
    /// under `self.root` before any bytes are read. Any path that escapes the root
    /// (e.g. `/../../../etc/passwd`) returns a 404 with no body.
    pub fn serve(&self, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
        let path = request.uri().path();

        // Strip leading slash, normalize
        let relative = path.trim_start_matches('/');

        // Prevent empty path from resolving to root directory
        if relative.is_empty() {
            return self.not_found();
        }

        // Resolve and canonicalize. If the file doesn't exist, canonicalize fails —
        // that's fine, we return 404. If it exists but escapes root, we return 404.
        let resolved = match self.root.join(relative).canonicalize() {
            Ok(p) if p.starts_with(&self.root) => p,
            _ => return self.not_found(),
        };

        // Must be a file, not a directory
        if !resolved.is_file() {
            return self.not_found();
        }

        let bytes = match std::fs::read(&resolved) {
            Ok(b) => b,
            Err(_) => return self.not_found(),
        };

        let content_type = self.mime_for(&resolved);

        let mut builder = Response::builder()
            .status(200)
            .header(http::header::CONTENT_TYPE, &content_type)
            // Prevent browsers from MIME-sniffing away from the declared type
            .header("X-Content-Type-Options", "nosniff")
            // No caching in dev; production apps can override with a custom AssetServer
            .header(http::header::CACHE_CONTROL, "no-cache, no-store");

        // Inject CSP via header for HTML responses.
        // The meta-tag injection in the initialization script covers the initial
        // page load, but protocol headers are the authoritative source for
        // subsequent navigations to served HTML files.
        if content_type.starts_with("text/html") {
            if let Some(csp) = &self.csp {
                builder = builder.header("Content-Security-Policy", csp);
            }
        }

        builder.body(Cow::Owned(bytes)).unwrap_or_else(|_| self.internal_error())
    }

    fn not_found(&self) -> Response<Cow<'static, [u8]>> {
        Response::builder()
            .status(404)
            .header(http::header::CONTENT_TYPE, "text/plain")
            .body(Cow::Borrowed(b"Not Found" as &[u8]))
            .unwrap()
    }

    fn internal_error(&self) -> Response<Cow<'static, [u8]>> {
        Response::builder()
            .status(500)
            .header(http::header::CONTENT_TYPE, "text/plain")
            .body(Cow::Borrowed(b"Internal Server Error" as &[u8]))
            .unwrap()
    }

    fn mime_for(&self, path: &Path) -> String {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        if let Some(override_type) = self.mime_overrides.get(&ext) {
            return override_type.clone();
        }

        // Built-in MIME table. Covers compiled/bundled web app output only.
        // Note: .ts is intentionally absent — TypeScript source files should
        // not be present in the asset root (which should contain compiled output).
        // Serving .ts would expose application source code unnecessarily.
        match ext.as_str() {
            "html" | "htm" => "text/html; charset=utf-8",
            "js" | "mjs"   => "text/javascript; charset=utf-8",
            "css"           => "text/css; charset=utf-8",
            "json"          => "application/json; charset=utf-8",
            "wasm"          => "application/wasm",
            "svg"           => "image/svg+xml",
            "png"           => "image/png",
            "jpg" | "jpeg"  => "image/jpeg",
            "gif"           => "image/gif",
            "webp"          => "image/webp",
            "ico"           => "image/x-icon",
            "woff"          => "font/woff",
            "woff2"         => "font/woff2",
            "ttf"           => "font/ttf",
            "otf"           => "font/otf",
            "txt"           => "text/plain; charset=utf-8",
            "xml"           => "application/xml",
            _               => "application/octet-stream",
        }
        .to_string()
    }
}
```

---

### Step 6: Updated `WebView` Struct and `handle_ipc`

The most important borrow-checker detail is **`std::mem::take` on `command_registry`**. During `handle_ipc`, we need to:
1. Look up the handler in `self.command_registry`
2. Call it with `(args, self, cx)` — which requires `&mut self`

You cannot hold a reference into `self.command_registry` and simultaneously pass `&mut self` to the handler. The solution is to temporarily move the registry out of `self`, call the handler, then move it back. `std::mem::take` works because `CommandRegistry` derives `Default` (an empty `HashMap`).

```rust
use std::{
    cell::RefCell,
    collections::VecDeque,
    ops::Deref,
    rc::Rc,
};

use mozui::{
    App, Bounds, ContentMask, Context, DismissEvent, Element, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, GlobalElementId, Hitbox, IntoElement, LayoutId, MouseDownEvent,
    ParentElement as _, Pixels, Render, Size, Style, Styled as _, Window, canvas, div,
};
use wry::{Rect, dpi::{self, LogicalSize}};

use crate::{
    ipc::{CommandRegistry, CommandResult, IpcEnvelope, IpcError, IpcMessage, parse_envelope},
    security::{Capabilities, CommandAllowlist, navigation_allowed, validate_ipc_request},
};

pub struct WebView {
    focus_handle: FocusHandle,
    webview: Rc<wry::WebView>,
    visible: bool,
    bounds: Bounds<Pixels>,

    // IPC
    /// Shared with the wry IPC closure. The closure pushes raw envelopes;
    /// render() drains and processes them.
    ipc_queue: Rc<RefCell<VecDeque<IpcEnvelope>>>,
    command_registry: CommandRegistry,

    // Navigation
    /// URLs that passed the allowlist filter, queued for NavigationEvent emission.
    nav_queue: Rc<RefCell<VecDeque<String>>>,
    /// URLs that were blocked, queued for NavigationBlockedEvent emission.
    nav_blocked_queue: Rc<RefCell<VecDeque<String>>>,

    // Security
    capabilities: Capabilities,
}

impl WebView {
    /// Escapes U+2028 (LINE SEPARATOR) and U+2029 (PARAGRAPH SEPARATOR) in a JSON string.
    ///
    /// serde_json does not escape these characters. Both are valid in JSON but are treated
    /// as line terminators by JavaScript engines, which can break the `evaluate_script` call
    /// that wraps the JSON. This function must be applied to every string passed to
    /// `evaluate_script` that originated from serde serialization.
    fn sanitize_for_evaluate(json: String) -> String {
        // These are rare enough that checking first avoids allocation in the common case
        if !json.contains('\u{2028}') && !json.contains('\u{2029}') {
            return json;
        }
        json.replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029")
    }

    /// Send a response to a pending JS `invoke()` call.
    pub fn send_response(&self, id: &str, result: Result<serde_json::Value, IpcError>) {
        let msg = match result {
            Ok(payload) => serde_json::json!({
                "__mozui": true,
                "id": id,
                "type": "response",
                "ok": true,
                "payload": payload,
            }),
            Err(err) => serde_json::json!({
                "__mozui": true,
                "id": id,
                "type": "response",
                "ok": false,
                "error": err,
            }),
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            let json = Self::sanitize_for_evaluate(json);
            let _ = self.webview.evaluate_script(
                &format!("window.__mozui_dispatch({json})")
            );
        }
    }

    /// Push an event to the renderer. JS listeners registered via `mozui.listen()`
    /// will receive it synchronously in the bridge's dispatch function.
    pub fn emit_to_js(&self, event: &str, payload: impl serde::Serialize) -> anyhow::Result<()> {
        let msg = serde_json::json!({
            "__mozui": true,
            "type": "event",
            "name": event,
            "payload": payload,
        });
        let json = Self::sanitize_for_evaluate(serde_json::to_string(&msg)?);
        Ok(self.webview.evaluate_script(&format!("window.__mozui_dispatch({json})"))?)
    }

    /// Raw script execution. Requires `capabilities.script_execution = true`.
    /// For pushing data to JS prefer `emit_to_js`, which is not gated.
    pub fn evaluate_script(&self, script: &str) -> anyhow::Result<()> {
        if !self.capabilities.script_execution {
            anyhow::bail!("script_execution capability is not enabled on this WebView");
        }
        Ok(self.webview.evaluate_script(script)?)
    }

    pub fn forward(&mut self) -> anyhow::Result<()> {
        Ok(self.webview.evaluate_script("history.forward();")?)
    }

    pub fn reload(&mut self) -> anyhow::Result<()> {
        Ok(self.webview.evaluate_script("location.reload();")?)
    }

    pub fn url(&self) -> anyhow::Result<String> {
        Ok(self.webview.url()?)
    }

    /// Process one IPC envelope. Called from `render()` after draining the queue.
    fn handle_ipc(&mut self, envelope: IpcEnvelope, cx: &mut Context<WebView>) {
        match envelope {
            IpcEnvelope::Invoke { id, command, args } => {
                // Capability check: is this command in the allowlist?
                // IMPORTANT: return UNKNOWN_COMMAND for both "not registered" and
                // "registered but not allowed" — returning UNAUTHORIZED would let callers
                // enumerate the command surface by observing which error code they get.
                if !self.capabilities.commands.permits(&command) {
                    self.send_response(&id, Err(IpcError::unknown_command(&command)));
                    return;
                }

                // Emit raw IpcMessage for untyped subscribers (escape hatch)
                cx.emit(IpcMessage { command: command.clone(), args: args.clone() });

                // Temporarily take the registry to avoid a double-borrow:
                // handler(args, &mut self, cx) requires &mut self, but calling
                // registry.handlers.get() would hold a borrow on self.command_registry.
                // std::mem::take leaves an empty Default registry in its place.
                let registry = std::mem::take(&mut self.command_registry);

                let result = if let Some(handler) = registry.handlers.get(&command) {
                    handler(args, self, cx)
                } else {
                    // Command passed the allowlist check but isn't in the registry.
                    // This can happen if CommandAllowlist::Only names a command that
                    // was never registered — treat as unknown, same code as above.
                    CommandResult::Immediate(Err(IpcError::unknown_command(&command)))
                };

                // Restore before doing anything else (including send_response,
                // which may re-enter handle_ipc indirectly via cx in a future phase)
                self.command_registry = registry;

                match result {
                    CommandResult::Immediate(r) => self.send_response(&id, r),
                    CommandResult::Deferred => {
                        // Phase 5: handler has the id and will call send_response later
                        // via a spawned task and entity update
                    }
                }
            }

            IpcEnvelope::Emit { name, payload } => {
                // JS-originated event (mozui.emit()). Check against the allowlist before
                // dispatching into the mozui entity system. Unknown names are silently
                // dropped — no error response, to avoid revealing the event surface.
                let allowed = match &self.capabilities.js_emit_allowlist {
                    None => true, // dev default: allow all
                    Some(list) => list.contains(&name.as_str()),
                };

                if allowed {
                    cx.emit(JsEmitEvent { name, payload });
                }
                // else: silently drop — do not respond, do not log at warn level
                // (high-frequency probing would spam logs; use a counter metric instead)
            }

            IpcEnvelope::Cancel { id } => {
                // Phase 5: JS-side timeout fired — cancel the in-flight async task
                // and free the pending slot. No-op for sync commands (already completed).
                self.pending_async.remove(&id);
            }
        }
    }
}

impl Drop for WebView {
    fn drop(&mut self) { self.hide(); }
}

impl Focusable for WebView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for WebView {}
impl EventEmitter<IpcMessage> for WebView {}
impl EventEmitter<NavigationEvent> for WebView {}
impl EventEmitter<NavigationBlockedEvent> for WebView {}
impl EventEmitter<JsEmitEvent> for WebView {}

impl Render for WebView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // --- Drain IPC queue ---
        // Collect first so we release the borrow on ipc_queue before calling
        // handle_ipc, which needs &mut self.
        let envelopes: Vec<IpcEnvelope> = self.ipc_queue.borrow_mut().drain(..).collect();
        for envelope in envelopes {
            self.handle_ipc(envelope, cx);
        }

        // --- Drain navigation queues ---
        let nav_urls: Vec<String> = self.nav_queue.borrow_mut().drain(..).collect();
        for url in nav_urls {
            cx.emit(NavigationEvent { url });
        }
        let blocked_urls: Vec<String> = self.nav_blocked_queue.borrow_mut().drain(..).collect();
        for url in blocked_urls {
            cx.emit(NavigationBlockedEvent { url });
        }

        // --- Layout (unchanged from existing implementation) ---
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
            .child(WebViewElement::new(self.webview.clone(), view))
    }
}
```

**Why `collect()` before `handle_ipc`:** If we held `self.ipc_queue.borrow_mut()` across the loop and `handle_ipc` tried to access any field of `self`, the borrow checker would stop us because `borrow_mut()` returns a `RefMut<VecDeque<...>>` that holds a live borrow on `self.ipc_queue`. Collecting into a `Vec` first releases that borrow so `handle_ipc` can take `&mut self` cleanly.

---

### Step 7: `builder.rs` — Full `build()` Implementation

This is the most complex step. The builder wires up all the wry callbacks, clones `Rc` handles into closures, and constructs the `WebView` entity.

```rust
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::VecDeque,
    path::PathBuf,
    rc::Rc,
};

use mozui::{App, AppContext as _, Entity, Window};
use wry::WebViewBuilder as WryBuilder;

use crate::{
    WebView,
    assets::AssetServer,
    ipc::{CommandRegistry, IpcEnvelope, parse_envelope},
    security::{Capabilities, navigation_allowed, validate_ipc_request},
};

const BRIDGE_JS: &str = include_str!("bridge/bridge.min.js");

/// Default CSP policy string. No nonce here — the nonce is generated per-session
/// and prepended to the script-src directive at build time in `build()`.
const DEFAULT_CSP_TEMPLATE: &str =
    "default-src 'self' mozui:; \
     script-src 'self' 'nonce-{NONCE}'; \
     style-src 'self' 'unsafe-inline'; \
     img-src 'self' mozui: data: blob:; \
     connect-src 'self' mozui:; \
     object-src 'none'; \
     base-uri 'self'; \
     frame-src 'none';";

pub struct WebViewBuilder {
    url: Option<String>,
    html: Option<String>,
    capabilities: Capabilities,
    csp: Option<String>,           // None = use default; Some("") = disable entirely
    navigation_allowlist: Vec<String>,
    command_registry: CommandRegistry,
    asset_server: Option<AssetServer>,
    transparent: bool,
    user_agent: Option<String>,
    #[cfg(feature = "inspector")]
    devtools: bool,
    initialization_scripts: Vec<String>,
}

impl Default for WebViewBuilder {
    fn default() -> Self { Self::new() }
}

impl WebViewBuilder {
    pub fn new() -> Self {
        Self {
            url: None,
            html: None,
            capabilities: Capabilities::default(),
            csp: None,
            navigation_allowlist: Vec::new(),
            command_registry: CommandRegistry::default(),
            asset_server: None,
            transparent: false,
            user_agent: None,
            #[cfg(feature = "inspector")]
            devtools: false,
            initialization_scripts: Vec::new(),
        }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into()); self
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into()); self
    }

    pub fn capabilities(mut self, caps: Capabilities) -> Self {
        self.capabilities = caps; self
    }

    /// Override the CSP. Pass an empty string to disable CSP injection entirely.
    pub fn csp(mut self, policy: impl Into<String>) -> Self {
        self.csp = Some(policy.into()); self
    }

    /// Allow navigation to URLs matching this glob pattern.
    /// `mozui://` and (in debug builds) `localhost` are always allowed.
    pub fn allow_navigation(mut self, pattern: impl Into<String>) -> Self {
        self.navigation_allowlist.push(pattern.into()); self
    }

    pub fn command<A, F>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        A: for<'de> serde::Deserialize<'de> + 'static,
        F: Fn(A, &mut crate::WebView, &mut mozui::Context<crate::WebView>)
               -> Result<serde_json::Value, crate::IpcError>
           + 'static,
    {
        self.command_registry.register(name, handler);
        self
    }

    pub fn assets(mut self, root: impl Into<PathBuf>) -> Self {
        self.asset_server = Some(AssetServer::new(root)); self
    }

    pub fn assets_with(mut self, server: AssetServer) -> Self {
        self.asset_server = Some(server); self
    }

    pub fn transparent(mut self, t: bool) -> Self {
        self.transparent = t; self
    }

    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into()); self
    }

    #[cfg(feature = "inspector")]
    pub fn devtools(mut self, enabled: bool) -> Self {
        self.devtools = enabled; self
    }

    /// Append a JS script that runs after the bridge but before page code.
    ///
    /// # Security
    ///
    /// Scripts added here run at the platform level, bypassing CSP entirely.
    /// They execute with the same trust as the bridge itself.
    ///
    /// **Never interpolate user-controlled or runtime data into these scripts.**
    /// String-formatting dynamic values into JS source is a code injection vulnerability.
    /// To pass data to the renderer at startup, use `emit_to_js` after the webview loads,
    /// or serve it from the asset server as a JSON file the renderer fetches.
    pub fn initialization_script(mut self, script: impl Into<String>) -> Self {
        self.initialization_scripts.push(script.into()); self
    }

    /// Construct the `Entity<WebView>` and register it with the app.
    ///
    /// This is where all wry callbacks are wired. Each callback captures a clone
    /// of an `Rc` — the `Rc` ensures the queue stays alive as long as either
    /// the WebView entity or the wry WebView is alive.
    pub fn build(self, window: &mut Window, cx: &mut App) -> Entity<WebView> {
        // --- Shared queues (Rc because wry callbacks are 'static but not Send) ---
        let ipc_queue: Rc<RefCell<VecDeque<IpcEnvelope>>> =
            Rc::new(RefCell::new(VecDeque::new()));
        let nav_queue: Rc<RefCell<VecDeque<String>>> =
            Rc::new(RefCell::new(VecDeque::new()));
        let nav_blocked_queue: Rc<RefCell<VecDeque<String>>> =
            Rc::new(RefCell::new(VecDeque::new()));

        // Clones for capture into closures
        let ipc_queue_for_handler = Rc::clone(&ipc_queue);
        let nav_queue_for_handler = Rc::clone(&nav_queue);
        let nav_blocked_queue_for_handler = Rc::clone(&nav_blocked_queue);

        // --- Generate CSP nonce (random per session, prevents script injection) ---
        let nonce = uuid::Uuid::new_v4().simple().to_string();

        // --- Build CSP string ---
        let csp_header = match &self.csp {
            Some(s) if s.is_empty() => None, // explicitly disabled
            Some(s) => Some(s.clone()),
            None => Some(DEFAULT_CSP_TEMPLATE.replace("{NONCE}", &nonce)),
        };

        // --- Build initialization script ---
        // Order: CSP meta tag → bridge → user scripts
        let mut init_script = String::new();

        // CSP as meta tag (belt-and-suspenders with the header)
        if let Some(csp) = &csp_header {
            // Escape any quotes in the CSP string before embedding in JS
            let escaped_csp = csp.replace('\'', "\\'").replace('"', "\\\"");
            init_script.push_str(&format!(
                r#"(function(){{
                    var m = document.createElement('meta');
                    m.httpEquiv = 'Content-Security-Policy';
                    m.content = "{escaped_csp}";
                    document.head.insertBefore(m, document.head.firstChild);
                }})();"#
            ));
        }

        // Bridge JS
        init_script.push_str(BRIDGE_JS);
        init_script.push('\n');

        // User-provided initialization scripts
        for script in &self.initialization_scripts {
            init_script.push_str(script);
            init_script.push('\n');
        }

        // --- Capabilities snapshot for closures ---
        // Clone into the callback closures so they don't borrow `self`
        let caps_for_ipc = self.capabilities.clone();
        let allowlist_for_nav = self.navigation_allowlist.clone();
        let dev_mode_for_nav = self.capabilities.dev_mode;

        // --- Construct wry WebViewBuilder ---
        let mut wry_builder = WryBuilder::new()
            .with_initialization_script(&init_script)
            .with_transparent(self.transparent);

        // URL or HTML content
        if let Some(url) = &self.url {
            wry_builder = wry_builder.with_url(url);
        } else if let Some(html) = &self.html {
            wry_builder = wry_builder.with_html(html);
        }

        // User agent
        if let Some(ua) = &self.user_agent {
            wry_builder = wry_builder.with_user_agent(ua);
        }

        // DevTools (only meaningful with `inspector` feature)
        #[cfg(feature = "inspector")]
        if self.devtools {
            wry_builder = wry_builder.with_devtools(true);
        }

        // --- Custom protocol: mozui:// asset server ---
        if let Some(asset_server) = self.asset_server {
            let csp_for_protocol = csp_header.clone();
            let server_with_csp = if let Some(csp) = csp_for_protocol {
                asset_server.with_csp(csp)
            } else {
                asset_server
            };

            // wry requires the handler be 'static, and AssetServer is owned, so
            // we can move it directly into the closure.
            wry_builder = wry_builder.with_custom_protocol(
                "mozui".to_string(),
                move |_webview_id, request| server_with_csp.serve(&request),
            );
        }

        // --- IPC handler ---
        // Receives all window.ipc.postMessage() calls from JS.
        // Validates origin and size, parses the envelope, pushes to queue.
        // Does NOT call into mozui from here — that happens in render().
        wry_builder = wry_builder.with_ipc_handler(move |request: wry::http::Request<String>| {
            // Security: validate before touching the payload
            if validate_ipc_request(&request, &caps_for_ipc).is_err() {
                // Drop silently — don't leak error info to foreign origins
                return;
            }

            // Parse the envelope; drop malformed or non-bridge messages
            if let Some(envelope) = parse_envelope(request.body()) {
                ipc_queue_for_handler.borrow_mut().push_back(envelope);
            }
        });

        // --- Navigation handler ---
        // Returns true to allow, false to block.
        // Also enqueues the URL for NavigationEvent emission regardless.
        let nav_queue_clone = Rc::clone(&nav_queue_for_handler);
        let nav_blocked_clone = Rc::clone(&nav_blocked_queue_for_handler);

        wry_builder = wry_builder.with_navigation_handler(move |url: String| {
            let allowed = navigation_allowed(&url, &allowlist_for_nav, dev_mode_for_nav);
            if allowed {
                nav_queue_clone.borrow_mut().push_back(url);
            } else {
                nav_blocked_clone.borrow_mut().push_back(url);
            }
            allowed
        });

        // --- Build wry WebView ---
        // Window implements HasWindowHandle (see crates/mozui/src/platform/macos/window.rs)
        // wry uses this to attach to the native window surface.
        let wry_webview = wry_builder
            .build(window)
            .expect("failed to create wry WebView");

        let wry_rc = Rc::new(wry_webview);

        // --- Create mozui entity ---
        cx.new(|cx| WebView {
            focus_handle: cx.focus_handle(),
            webview: wry_rc,
            visible: true,
            bounds: mozui::Bounds::default(),
            ipc_queue,
            command_registry: self.command_registry,
            nav_queue,
            nav_blocked_queue,
            capabilities: self.capabilities,
        })
    }
}
```

**`wry_builder.build(window)` signature:** wry's `WebViewBuilder::build` takes `impl HasWindowHandle + HasDisplayHandle`. mozui's `Window` type implements `raw_window_handle::HasWindowHandle` and `HasDisplayHandle` — both traits are already pulled in via `raw-window-handle = "0.6"` in the workspace. No extra wrapping needed.

---

### Step 8: `Cargo.toml` Changes

```toml
[package]
name = "mozui-webview"
# ... existing fields ...

[features]
inspector = ["wry/devtools"]
ts-gen   = ["dep:specta", "dep:specta-typescript"]

[dependencies]
anyhow.workspace    = true
mozui.workspace     = true
wry                 = { version = "0.53.3", package = "lb-wry" }
serde               = { workspace = true, features = ["derive"] }
serde_json.workspace = true
http.workspace      = true
uuid                = { workspace = true, features = ["v4"] }
specta              = { version = "2", optional = true, features = ["derive"] }

[dev-dependencies]
specta-typescript   = { version = "0.0.7", optional = true }

[build-dependencies]
minify-js = "0.6"
```

`uuid` is already a workspace dep with `v4` available — confirm the workspace `uuid` entry has the `v4` feature enabled (it does: `features = ["v4", "v5", "v7", "serde"]`). No workspace change needed.

`serde` and `serde_json` are both workspace deps; `http` is also a workspace dep (version `"1.1"`). All three are used by `mozui` core already. Adding them to `mozui-webview` is just declaring explicit usage.

---

### Step 9: TypeScript Type Generation Binary

Add a binary target to `Cargo.toml`:

```toml
[[bin]]
name    = "gen-ts"
path    = "src/bin/gen_ts.rs"
required-features = ["ts-gen"]
```

**`src/bin/gen_ts.rs`**:

```rust
//! Run with: cargo run -p mozui-webview --bin gen-ts --features ts-gen
//! Emits `mozui-bindings.d.ts` to stdout (redirect to your project's types dir).

fn main() {
    use specta::export::ts;
    use specta_typescript::Typescript;

    // All types that appear in command args or return values must be registered here.
    // In a real project, you'd pull these in from the app's command definitions.
    // This binary is meant to be customized per app, not run as-is from the library.

    let type_map = specta::TypeCollection::default();
    // type_map.register::<MyArgs>();
    // type_map.register::<MyReturn>();

    let config = Typescript::default()
        .header("// mozui-bindings.d.ts — generated by gen-ts, do not edit\n")
        .bigint(specta_typescript::BigIntExportBehavior::Number);

    // Emit to stdout
    match specta_typescript::export(&type_map, &config) {
        Ok(ts_output) => print!("{ts_output}"),
        Err(e) => {
            eprintln!("type export failed: {e}");
            std::process::exit(1);
        }
    }
}
```

Apps using `mozui-webview` as a dependency define their own version of this binary. The library itself ships the infrastructure (feature flag, dependencies, `specta::Type` derive macros available via re-export). The actual type registration is the app's responsibility.

**Recommended app-level workflow:**

```
cargo run -p my-app --bin gen-ts --features mozui-webview/ts-gen > src/renderer/src/mozui-bindings.d.ts
```

This drops the `.d.ts` file into the Vite project directory, where TypeScript picks it up automatically.

---

### Step 10: End-to-End Usage Example

This is the full picture from both sides — what app code looks like after the implementation is complete.

**Rust:**

```rust
use mozui_webview::{Capabilities, OriginAllowlist, WebViewBuilder};
use serde::{Deserialize, Serialize};

#[cfg(feature = "mozui-webview/ts-gen")]
use specta::Type;

#[derive(Deserialize)]
#[cfg_attr(feature = "mozui-webview/ts-gen", derive(Type))]
struct ReadFileArgs {
    path: String,
}

#[derive(Serialize)]
#[cfg_attr(feature = "mozui-webview/ts-gen", derive(Type))]
struct ReadFileResponse {
    contents: String,
    size_bytes: usize,
}

fn create_webview(window: &mut Window, cx: &mut App) -> Entity<WebView> {
    WebViewBuilder::new()
        .url(if cfg!(debug_assertions) {
            "http://localhost:5173"
        } else {
            "mozui://localhost/index.html"
        })
        .assets("dist")  // serves mozui://localhost/* from ./dist/
        .capabilities(Capabilities {
            ipc_origins: if cfg!(debug_assertions) {
                OriginAllowlist::Custom(vec!["http://localhost:5173".into()])
            } else {
                OriginAllowlist::MozuiOnly
            },
            ..Default::default()
        })
        .command("read_file", |args: ReadFileArgs, _webview, _cx| {
            let contents = std::fs::read_to_string(&args.path)
                .map_err(|e| IpcError::internal(e))?;
            let size_bytes = contents.len();
            Ok(serde_json::to_value(ReadFileResponse { contents, size_bytes })?)
        })
        .command("get_version", |_: (), _webview, _cx| {
            Ok(serde_json::json!(env!("CARGO_PKG_VERSION")))
        })
        .build(window, cx)
}

// Subscribing to navigation events in a parent view:
cx.subscribe(&webview, |this, _webview, event: &NavigationEvent, cx| {
    this.current_url = event.url.clone();
    cx.notify();
});
```

**TypeScript (renderer):**

```typescript
// types/mozui-bindings.d.ts (generated)
declare global {
  interface Window {
    mozui: {
      invoke(command: "read_file", args: { path: string }): Promise<{ contents: string; size_bytes: number }>;
      invoke(command: "get_version"): Promise<string>;
      invoke(command: string, args?: unknown): Promise<unknown>;
      listen<T>(event: string, handler: (payload: T) => void): () => void;
      emit(event: string, payload?: unknown): void;
    };
  }
}

// App code — fully typed, no casting needed
async function loadConfig(): Promise<void> {
  try {
    const result = await mozui.invoke("read_file", { path: "/etc/myapp/config.json" });
    console.log(`Loaded ${result.size_bytes} bytes`);
  } catch (err: any) {
    // err.code is one of the IpcError constants from Rust
    if (err.code === "INVALID_ARGS") {
      console.error("Bad args:", err.message);
    }
  }
}

// Listen for Rust-pushed events
const unlisten = mozui.listen<{ path: string }>("config_changed", (payload) => {
  console.log("Config changed:", payload.path);
  loadConfig();
});

// Clean up when component unmounts
onDestroy(unlisten);
```

**Pushing an event from Rust to the renderer** (e.g., from a file watcher):

```rust
let webview = webview.clone(); // Entity<WebView>
let path = changed_path.clone();

cx.spawn(async move |mut cx| {
    cx.update(|cx| {
        webview.update(cx, |wv, _cx| {
            wv.emit_to_js("config_changed", &serde_json::json!({ "path": path })).ok();
        });
    }).ok();
})
.detach();
```
