use std::collections::HashMap;
use std::time::Instant;

use mozui_webview::IpcError;

/// Tracks a forwarded invoke from a renderer to the main process webview.
pub(crate) struct PendingForward {
    /// Label of the renderer window that originated the invoke.
    pub source_window: String,
    /// The JS-side promise ID (from the renderer's IPC envelope).
    pub original_invoke_id: String,
    /// When this forward was created (for timeout cleanup).
    pub created_at: Instant,
}

/// Routes IPC commands between renderer windows and the main process webview.
///
/// Built-in and plugin commands are handled directly in Rust (fast path).
/// User-defined commands (registered via `Silk.handle()` in main.ts) are
/// forwarded to the main webview and routed back when the response arrives.
pub(crate) struct IpcRouter {
    /// Pending forwards: correlation_id → PendingForward
    pending: HashMap<String, PendingForward>,
    /// Monotonic counter for generating correlation IDs.
    next_id: u64,
    /// Timeout for pending forwards (seconds).
    timeout_secs: u64,
}

impl IpcRouter {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            next_id: 0,
            timeout_secs: 30,
        }
    }

    /// Create a new pending forward and return its correlation ID.
    pub fn create_forward(&mut self, source_window: String, original_invoke_id: String) -> String {
        let correlation_id = format!("silk-fwd-{}", self.next_id);
        self.next_id += 1;

        self.pending.insert(
            correlation_id.clone(),
            PendingForward {
                source_window,
                original_invoke_id,
                created_at: Instant::now(),
            },
        );

        correlation_id
    }

    /// Resolve a pending forward by correlation ID. Returns the forward info
    /// so the caller can route the response back to the source window.
    pub fn resolve(&mut self, correlation_id: &str) -> Option<PendingForward> {
        self.pending.remove(correlation_id)
    }

    /// Clean up forwards that have exceeded the timeout. Returns error responses
    /// that should be sent back to the source windows.
    pub fn sweep_expired(&mut self) -> Vec<(String, String, IpcError)> {
        let timeout = std::time::Duration::from_secs(self.timeout_secs);
        let now = Instant::now();

        let expired: Vec<String> = self
            .pending
            .iter()
            .filter(|(_, fwd)| now.duration_since(fwd.created_at) > timeout)
            .map(|(id, _)| id.clone())
            .collect();

        let mut errors = Vec::new();
        for id in expired {
            if let Some(fwd) = self.pending.remove(&id) {
                errors.push((
                    fwd.source_window,
                    fwd.original_invoke_id,
                    IpcError {
                        code: IpcError::INTERNAL,
                        message: "main process handler timed out".into(),
                    },
                ));
            }
        }
        errors
    }
}
