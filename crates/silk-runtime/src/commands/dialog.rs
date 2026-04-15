use mozui::PathPromptOptions;
use mozui_webview::{CommandResult, IpcError};
use serde::Deserialize;
use std::path::Path;

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
    IpcError {
        code: IpcError::INTERNAL,
        message: msg,
    }
}

pub(crate) fn open(
    args: OpenDialogArgs,
    _wv: &mut mozui_webview::WebView,
    cx: &mut mozui::App,
) -> CommandResult {
    let options = PathPromptOptions {
        files: true,
        directories: false,
        multiple: args.multiple,
        prompt: args.title.map(Into::into),
    };

    let rx = cx.prompt_for_paths(options);

    CommandResult::Deferred(Box::pin(async move {
        let result = rx
            .await
            .map_err(|_| err("dialog cancelled".into()))?
            .map_err(|e| err(format!("dialog failed: {e}")))?;

        match result {
            None => Ok(serde_json::Value::Null),
            Some(paths) => {
                let string_paths: Vec<String> = paths
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                if args.multiple {
                    Ok(serde_json::json!(string_paths))
                } else {
                    Ok(serde_json::json!(string_paths.first()))
                }
            }
        }
    }))
}

pub(crate) fn save(
    args: SaveDialogArgs,
    _wv: &mut mozui_webview::WebView,
    cx: &mut mozui::App,
) -> CommandResult {
    let directory = args
        .default_path
        .as_deref()
        .and_then(|p| Path::new(p).parent())
        .unwrap_or(Path::new("."))
        .to_owned();

    let suggested_name = args
        .default_path
        .as_deref()
        .and_then(|p| Path::new(p).file_name())
        .map(|n| n.to_string_lossy().into_owned());

    let rx = cx.prompt_for_new_path(&directory, suggested_name.as_deref());

    CommandResult::Deferred(Box::pin(async move {
        let result = rx
            .await
            .map_err(|_| err("dialog cancelled".into()))?
            .map_err(|e| err(format!("dialog failed: {e}")))?;

        match result {
            None => Ok(serde_json::Value::Null),
            Some(path) => Ok(serde_json::json!(path.to_string_lossy())),
        }
    }))
}

pub(crate) fn message(
    args: MessageDialogArgs,
    _wv: &mut mozui_webview::WebView,
    _cx: &mut mozui::App,
) -> CommandResult {
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

        // Message dialog: spawn osascript on a background thread so it doesn't
        // block the main run loop.
        let (tx, rx) = futures::channel::oneshot::channel::<()>();
        std::thread::spawn(move || {
            let _ = Command::new("osascript").args(["-e", &script]).output();
            let _ = tx.send(());
        });

        CommandResult::Deferred(Box::pin(async move {
            let _ = rx.await;
            Ok(serde_json::json!(true))
        }))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = args;
        CommandResult::Immediate(Err(err("dialog not supported on this platform".into())))
    }
}
