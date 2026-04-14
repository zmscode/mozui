use serde::Deserialize;
use mozui_webview::IpcError;

#[derive(Deserialize)]
pub(crate) struct OpenDialogArgs {
    pub title: Option<String>,
    pub filters: Option<Vec<DialogFilter>>,
    #[serde(default)]
    pub multiple: bool,
}

#[derive(Deserialize)]
pub(crate) struct SaveDialogArgs {
    pub title: Option<String>,
    #[serde(rename = "defaultPath")]
    pub default_path: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct MessageDialogArgs {
    pub title: Option<String>,
    pub message: String,
    #[serde(rename = "type", default = "default_dialog_type")]
    pub dialog_type: String,
}

#[derive(Deserialize)]
pub(crate) struct DialogFilter {
    pub extensions: Vec<String>,
}

fn default_dialog_type() -> String {
    "info".into()
}

fn err(msg: String) -> IpcError {
    IpcError { code: IpcError::INTERNAL, message: msg }
}

pub(crate) fn open(args: OpenDialogArgs, _wv: &mut mozui_webview::WebView, _cx: &mut mozui::App) -> Result<serde_json::Value, IpcError> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let mut script = String::from("choose file");
        if let Some(title) = &args.title {
            script.push_str(&format!(" with prompt \"{}\"", title.replace('"', "\\\"")));
        }
        if args.multiple {
            script.push_str(" with multiple selections allowed");
        }
        if let Some(filters) = &args.filters {
            let exts: Vec<String> = filters
                .iter()
                .flat_map(|f| f.extensions.iter().map(|e| format!("\"{e}\"")))
                .collect();
            if !exts.is_empty() {
                script.push_str(&format!(" of type {{{}}}", exts.join(", ")));
            }
        }

        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| err(format!("dialog failed: {e}")))?;

        if !output.status.success() {
            return Ok(serde_json::Value::Null);
        }

        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if raw.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        let paths: Vec<String> = raw
            .split(", ")
            .map(|alias| {
                Command::new("osascript")
                    .args(["-e", &format!("POSIX path of \"{alias}\"")])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_else(|_| alias.to_string())
            })
            .collect();

        if args.multiple {
            Ok(serde_json::json!(paths))
        } else {
            Ok(serde_json::json!(paths.first()))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = args;
        Err(err("dialog not supported on this platform".into()))
    }
}

pub(crate) fn save(args: SaveDialogArgs, _wv: &mut mozui_webview::WebView, _cx: &mut mozui::App) -> Result<serde_json::Value, IpcError> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let mut script = String::from("choose file name");
        if let Some(title) = &args.title {
            script.push_str(&format!(" with prompt \"{}\"", title.replace('"', "\\\"")));
        }
        if let Some(path) = &args.default_path {
            script.push_str(&format!(" default name \"{}\"", path.replace('"', "\\\"")));
        }

        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| err(format!("dialog failed: {e}")))?;

        if !output.status.success() {
            return Ok(serde_json::Value::Null);
        }

        let alias = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if alias.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        let posix = Command::new("osascript")
            .args(["-e", &format!("POSIX path of \"{alias}\"")])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or(alias);

        Ok(serde_json::json!(posix))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = args;
        Err(err("dialog not supported on this platform".into()))
    }
}

pub(crate) fn message(args: MessageDialogArgs, _wv: &mut mozui_webview::WebView, _cx: &mut mozui::App) -> Result<serde_json::Value, IpcError> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let icon = match args.dialog_type.as_str() {
            "warning" => "caution",
            "error" => "stop",
            _ => "note",
        };
        let title = args.title.as_deref().unwrap_or("Silk App");
        let script = format!(
            "display dialog \"{}\" with title \"{}\" with icon {} buttons {{\"OK\"}} default button \"OK\"",
            args.message.replace('"', "\\\""),
            title.replace('"', "\\\""),
            icon,
        );

        Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| err(format!("dialog failed: {e}")))?;

        Ok(serde_json::json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = args;
        Err(err("dialog not supported on this platform".into()))
    }
}
