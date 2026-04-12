# mozui-webview Expansion Plan

## 1. New Dependencies

Add `http` to `mozui-webview/Cargo.toml` (wry re-exports `http` types but we need it for protocol handler signatures). Also add `serde` and `serde_json` for typed IPC messages.

## 2. IPC / Message Passing

**The problem:** wry's IPC handler signature is `Fn(Request<String>) + 'static` — a plain callback with no access to mozui's `Context`. We need to bridge this into the entity system.

**Solution: shared queue + render drain.**

- Create an `Rc<RefCell<VecDeque<String>>>` as a message buffer
- The wry IPC handler pushes incoming messages onto it
- During `render()`, the `WebView` drains the queue and calls `cx.emit(IpcMessage { body })` for each

Since wry callbacks run on the main thread synchronously (called by the native webview during the same run loop), we can't call into mozui's entity system from inside the callback. The render-drain pattern bridges this gap.

**One-frame delay:** The wry IPC fires during the native event loop. mozui re-renders on the next frame. This means there's a ~16ms delay between JS posting a message and Rust processing it. This is fine — it's the same as any event-driven UI.

**Outbound (Rust -> JS):** `post_message(&self, data: &str)` wraps `evaluate_script` to call a `window.__mozui_onMessage(data)` function. Consumers register a JS handler on that global.

**Event type:**

```rust
pub struct IpcMessage {
    pub body: String,
}
impl EventEmitter<IpcMessage> for WebView {}
```

## 3. WebViewBuilder

Wraps wry's `WebViewBuilder` with mozui-idiomatic API:

```rust
pub struct WebViewBuilder {
    url: Option<String>,
    html: Option<String>,
    transparent: bool,
    user_agent: Option<String>,
    custom_protocols: Vec<(String, ProtocolHandler)>,
    navigation_filter: Option<Box<dyn Fn(&str) -> bool>>,
    devtools: bool,
}
```

**Build flow:**

1. `WebViewBuilder::new()` creates default config
2. Chainable methods configure it
3. `build(window: &mut Window, cx: &mut App) -> Entity<WebView>` does:
   - Create the IPC message queue (`Rc<RefCell<VecDeque<String>>>`)
   - Create `wry::WebViewBuilder::new()` and apply all settings
   - Wire up `with_ipc_handler` to push to the shared queue
   - Wire up `with_navigation_handler` with the filter + emit `NavigationEvent`
   - Wire up any `with_custom_protocol` handlers
   - Call `.build(&window)` using mozui's `HasWindowHandle` impl (`Window` implements `HasWindowHandle` at `window.rs:5474`)
   - Wrap in `WebView` and return as `Entity`

## 4. Navigation Events

**Filter (blocking):** `with_navigation_filter(impl Fn(&str) -> bool + 'static)` on the builder, maps to wry's `with_navigation_handler`. Returns `true` to allow, `false` to block.

**Observation (non-blocking):** Same shared-queue pattern as IPC. The navigation handler pushes allowed URLs onto an `Rc<RefCell<VecDeque<String>>>`, and during render, the WebView emits:

```rust
pub struct NavigationEvent {
    pub url: String,
}
impl EventEmitter<NavigationEvent> for WebView {}
```

## 5. Custom Protocol Handler

Expose on the builder:

```rust
pub fn with_custom_protocol<F>(mut self, name: impl Into<String>, handler: F) -> Self
where
    F: Fn(&str, http::Request<Vec<u8>>) -> http::Response<Cow<'static, [u8]>> + 'static
```

Maps directly to wry's `with_custom_protocol`. The `&str` is the `WebViewId`. Could also offer an async variant later.

## 6. Forward Navigation + URL State

Simple additions to the existing `WebView` impl:

```rust
pub fn forward(&mut self) -> anyhow::Result<()> {
    Ok(self.webview.evaluate_script("history.forward();")?)
}

pub fn reload(&mut self) -> anyhow::Result<()> {
    Ok(self.webview.evaluate_script("location.reload();")?)
}

pub fn url(&self) -> anyhow::Result<String> {
    Ok(self.webview.url()?)
}
```

## File Structure

Everything stays in `lib.rs` — the crate is small enough. New types and the builder go alongside the existing code.

## Open Questions

1. **Multiple event emitters:** `WebView` currently implements `EventEmitter<DismissEvent>`. We'd add `EventEmitter<IpcMessage>` and `EventEmitter<NavigationEvent>`. Consumers subscribe to whichever they care about. Alternative: a single unified `WebViewEvent` enum.

2. **IPC message format:** Plain `String` (consumers parse however they like) vs. requiring JSON (`serde_json::Value`). Leaning plain `String` to keep it flexible — consumers can layer JSON on top.

3. **Navigation filter default:** If no filter is set, all navigations are allowed (wry default).

4. **Render-drain pattern:** Cleanest approach given wry's callback constraints. The alternative would be trying to get a raw platform callback into mozui's event loop, but that's invasive. One-frame-delay tradeoff is acceptable for most use cases.
