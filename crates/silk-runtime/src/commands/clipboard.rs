use mozui_webview::IpcError;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct WriteClipboardArgs {
    pub text: String,
}

fn err(msg: String) -> IpcError {
    IpcError {
        code: IpcError::INTERNAL,
        message: msg,
    }
}

pub(crate) fn read(
    _args: (),
    _wv: &mut mozui_webview::WebView,
    _cx: &mut mozui::App,
) -> Result<serde_json::Value, IpcError> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("pbpaste")
            .output()
            .map_err(|e| err(format!("clipboard read failed: {e}")))?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(serde_json::Value::String(text))
    }
    #[cfg(not(target_os = "macos"))]
    Err(err("clipboard not supported on this platform".into()))
}

pub(crate) fn write(
    args: WriteClipboardArgs,
    _wv: &mut mozui_webview::WebView,
    _cx: &mut mozui::App,
) -> Result<serde_json::Value, IpcError> {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| err(format!("clipboard write failed: {e}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(args.text.as_bytes())
                .map_err(|e| err(format!("clipboard write failed: {e}")))?;
        }
        child
            .wait()
            .map_err(|e| err(format!("clipboard write failed: {e}")))?;
        Ok(serde_json::json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = args;
        Err(err("clipboard not supported on this platform".into()))
    }
}
