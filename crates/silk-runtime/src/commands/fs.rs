use serde::Deserialize;
use mozui_webview::IpcError;

#[derive(Deserialize)]
pub(crate) struct ReadTextArgs {
    pub path: String,
}

#[derive(Deserialize)]
pub(crate) struct WriteTextArgs {
    pub path: String,
    pub contents: String,
}

#[derive(Deserialize)]
pub(crate) struct PathArgs {
    pub path: String,
}

#[derive(Deserialize)]
pub(crate) struct MkdirArgs {
    pub path: String,
    #[serde(default)]
    pub recursive: bool,
}

fn err(msg: String) -> IpcError {
    IpcError { code: IpcError::INTERNAL, message: msg }
}

pub(crate) fn read_text(args: ReadTextArgs, _wv: &mut mozui_webview::WebView, _cx: &mut mozui::App) -> Result<serde_json::Value, IpcError> {
    let contents = std::fs::read_to_string(&args.path)
        .map_err(|e| err(format!("failed to read {}: {e}", args.path)))?;
    Ok(serde_json::Value::String(contents))
}

pub(crate) fn write_text(args: WriteTextArgs, _wv: &mut mozui_webview::WebView, _cx: &mut mozui::App) -> Result<serde_json::Value, IpcError> {
    std::fs::write(&args.path, &args.contents)
        .map_err(|e| err(format!("failed to write {}: {e}", args.path)))?;
    Ok(serde_json::json!(true))
}

pub(crate) fn exists(args: PathArgs, _wv: &mut mozui_webview::WebView, _cx: &mut mozui::App) -> Result<serde_json::Value, IpcError> {
    Ok(serde_json::json!(std::path::Path::new(&args.path).exists()))
}

pub(crate) fn mkdir(args: MkdirArgs, _wv: &mut mozui_webview::WebView, _cx: &mut mozui::App) -> Result<serde_json::Value, IpcError> {
    if args.recursive {
        std::fs::create_dir_all(&args.path)
    } else {
        std::fs::create_dir(&args.path)
    }
    .map_err(|e| err(format!("failed to mkdir {}: {e}", args.path)))?;
    Ok(serde_json::json!(true))
}

pub(crate) fn remove(args: PathArgs, _wv: &mut mozui_webview::WebView, _cx: &mut mozui::App) -> Result<serde_json::Value, IpcError> {
    let path = std::path::Path::new(&args.path);
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
    .map_err(|e| err(format!("failed to remove {}: {e}", args.path)))?;
    Ok(serde_json::json!(true))
}
