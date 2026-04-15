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

/// Security capabilities for a webview instance.
///
/// Defaults to the most restrictive posture. Every feature is opt-in.
pub struct Capabilities {
    /// Which commands JS is allowed to invoke.
    pub commands: CommandAllowlist,
    /// Allow raw `evaluate_script` calls from Rust. Default: false.
    pub script_execution: bool,
    /// Allow navigations to external (non-mozui://) URLs.
    pub external_navigation: bool,
    /// Maximum IPC payload size in bytes. Default: 1MB.
    pub max_payload_bytes: usize,
    /// Which origins may send IPC messages. Default: mozui:// only.
    pub ipc_origins: OriginAllowlist,
    /// Which event names JS is allowed to emit via `mozui.emit()`.
    /// None = allow all (development); Some([...]) = allowlist.
    pub js_emit_allowlist: Option<Vec<&'static str>>,
    /// Enable development-mode relaxations. Must be set explicitly.
    pub dev_mode: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            #[allow(deprecated)]
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

/// Which commands the JS renderer is allowed to invoke.
pub enum CommandAllowlist {
    /// Allow all registered commands.
    /// DEPRECATED in release builds.
    #[cfg_attr(
        not(debug_assertions),
        deprecated = "CommandAllowlist::All in release builds allows every registered \
                      command from the renderer. Set CommandAllowlist::Only explicitly."
    )]
    All,
    /// Allow only the named commands. All others return UNKNOWN_COMMAND.
    Only(Vec<&'static str>),
}

impl CommandAllowlist {
    /// Check if a command name is allowed.
    pub fn permits(&self, name: &str) -> bool {
        match self {
            #[allow(deprecated)]
            CommandAllowlist::All => true,
            CommandAllowlist::Only(list) => list.iter().any(|&allowed| allowed == name),
        }
    }
}

/// Which origins may send IPC messages to the webview.
#[derive(Clone)]
pub enum OriginAllowlist {
    /// Only the platform-correct mozui:// origin. Production default.
    MozuiOnly,
    /// Exact-match against the HTTP Origin header.
    Custom(Vec<String>),
    /// Any origin. Use only for fully trusted, sandboxed content.
    Any,
}

impl OriginAllowlist {
    /// Check if an origin is permitted.
    pub fn permits(&self, origin: Option<&str>) -> bool {
        match self {
            OriginAllowlist::MozuiOnly => {
                // WKWebView does not send an Origin header for custom-scheme
                // (mozui://) pages, so `None` is accepted alongside the
                // expected mozui origin.
                match origin {
                    None => true,
                    Some(o) => o.starts_with(MOZUI_ORIGIN),
                }
            }
            OriginAllowlist::Custom(list) => {
                matches!(origin, Some(o) if list.iter().any(|allowed| allowed == o))
            }
            OriginAllowlist::Any => true,
        }
    }
}
