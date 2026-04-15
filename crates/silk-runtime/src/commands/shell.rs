use mozui_webview::IpcError;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct OpenUrlArgs {
    pub url: String,
}

pub(crate) fn open(
    args: OpenUrlArgs,
    _wv: &mut mozui_webview::WebView,
    _cx: &mut mozui::App,
) -> Result<serde_json::Value, IpcError> {
    let err = |e: std::io::Error| IpcError {
        code: IpcError::INTERNAL,
        message: format!("failed to open URL: {e}"),
    };

    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&args.url)
        .spawn()
        .map_err(err)?;

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&args.url)
        .spawn()
        .map_err(err)?;

    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/c", "start", &args.url])
        .spawn()
        .map_err(err)?;

    Ok(serde_json::json!(true))
}
