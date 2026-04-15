use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::WebView;
use mozui::App;

/// Parsed incoming IPC message from the JS bridge.
#[derive(Debug, Deserialize)]
pub struct IpcEnvelope {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub command: Option<String>,
    pub name: Option<String>,
    pub args: Option<serde_json::Value>,
    pub payload: Option<serde_json::Value>,
}

/// Error returned to the JS bridge on command failure.
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

    pub fn unknown_command() -> Self {
        Self {
            code: Self::UNKNOWN_COMMAND,
            message: "unknown command".into(),
        }
    }

    pub fn invalid_args(detail: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INVALID_ARGS,
            message: format!("invalid arguments: {detail}"),
        }
    }

    pub fn payload_too_large() -> Self {
        Self {
            code: Self::PAYLOAD_TOO_LARGE,
            message: "payload too large".into(),
        }
    }

    pub fn internal() -> Self {
        Self {
            code: Self::INTERNAL,
            message: "internal error".into(),
        }
    }
}

impl From<serde_json::Error> for IpcError {
    fn from(_: serde_json::Error) -> Self {
        Self::internal()
    }
}

/// Result of a command handler.
pub enum CommandResult {
    Immediate(Result<serde_json::Value, IpcError>),
    /// The handler kicked off an async operation (e.g. a native file dialog) and
    /// returns a future that will resolve with the result.
    Deferred(
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, IpcError>>>>,
    ),
}

/// A boxed command handler function.
pub type CommandHandler = Box<dyn Fn(serde_json::Value, &mut WebView, &mut App) -> CommandResult>;

/// Registry mapping command names to their handlers.
pub struct CommandRegistry {
    handlers: HashMap<String, CommandHandler>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a typed command handler. Args are deserialized from JSON automatically.
    pub fn register<A, F>(&mut self, name: impl Into<String>, handler: F)
    where
        A: for<'de> Deserialize<'de> + 'static,
        F: Fn(A, &mut WebView, &mut App) -> Result<serde_json::Value, IpcError> + 'static,
    {
        let name = name.into();
        self.handlers.insert(
            name,
            Box::new(move |raw_args, webview, cx| {
                let args: A = match serde_json::from_value(raw_args) {
                    Ok(a) => a,
                    Err(e) => return CommandResult::Immediate(Err(IpcError::invalid_args(e))),
                };
                CommandResult::Immediate(handler(args, webview, cx))
            }),
        );
    }

    /// Register a command handler that may return a deferred (async) result.
    ///
    /// Unlike `register`, the handler returns `CommandResult` directly, allowing
    /// it to return either `Immediate` or `Deferred`.
    pub fn register_deferred<A, F>(&mut self, name: impl Into<String>, handler: F)
    where
        A: for<'de> Deserialize<'de> + 'static,
        F: Fn(A, &mut WebView, &mut App) -> CommandResult + 'static,
    {
        let name = name.into();
        self.handlers.insert(
            name,
            Box::new(move |raw_args, webview, cx| {
                let args: A = match serde_json::from_value(raw_args) {
                    Ok(a) => a,
                    Err(e) => return CommandResult::Immediate(Err(IpcError::invalid_args(e))),
                };
                handler(args, webview, cx)
            }),
        );
    }

    /// Look up a handler by command name.
    pub fn get(&self, name: &str) -> Option<&CommandHandler> {
        self.handlers.get(name)
    }

    /// Check if a command is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }
}

/// Configuration for async command execution limits.
pub struct AsyncCommandConfig {
    /// Maximum number of in-flight async invocations across all commands. Default: 32.
    pub max_concurrent: usize,
    /// Per-invocation Rust-side timeout. Default: 30s.
    pub timeout: Duration,
    /// Per-command concurrency limit. None = use max_concurrent.
    pub per_command_limit: Option<HashMap<&'static str, usize>>,
}

impl Default for AsyncCommandConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 32,
            timeout: Duration::from_secs(30),
            per_command_limit: None,
        }
    }
}

/// A boxed async command handler. Takes raw JSON args, deserializes, and returns
/// a boxed future that resolves to the IPC result.
pub type AsyncCommandHandler = Box<
    dyn Fn(
        serde_json::Value,
    ) -> Result<
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, IpcError>>>>,
        IpcError,
    >,
>;

/// Registry for async command handlers, separate from sync handlers.
pub struct AsyncCommandRegistry {
    handlers: HashMap<String, AsyncCommandHandler>,
}

impl AsyncCommandRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a typed async command handler.
    ///
    /// The handler takes deserialized args and returns a future.
    /// The infrastructure handles spawning, response delivery, and rate limiting.
    pub fn register<A, Fut, F>(&mut self, name: impl Into<String>, handler: F)
    where
        A: for<'de> Deserialize<'de> + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, IpcError>> + 'static,
        F: Fn(A) -> Fut + 'static,
    {
        let name = name.into();
        self.handlers.insert(
            name,
            Box::new(move |raw_args| {
                let args: A =
                    serde_json::from_value(raw_args).map_err(|e| IpcError::invalid_args(e))?;
                Ok(Box::pin(handler(args)))
            }),
        );
    }

    /// Look up an async handler by command name.
    pub fn get(&self, name: &str) -> Option<&AsyncCommandHandler> {
        self.handlers.get(name)
    }

    /// Check if an async command is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }
}

/// Raw IPC message event emitted for consumers who want untyped access.
pub struct IpcMessage {
    pub command: String,
    pub args: serde_json::Value,
}

/// Navigation event emitted when the webview navigates to an allowed URL.
pub struct NavigationEvent {
    pub url: String,
}

/// Navigation blocked event emitted when a navigation is denied.
pub struct NavigationBlockedEvent {
    pub url: String,
}

/// Event emitted when JS calls `mozui.emit(name, payload)`.
///
/// Only fires if the event name passes the `js_emit_allowlist` in `Capabilities`.
pub struct JsEmitEvent {
    pub name: String,
    pub payload: Option<serde_json::Value>,
}
