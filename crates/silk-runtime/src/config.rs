use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Silk application configuration, loaded from `silk.config.json`.
#[derive(Debug, Deserialize)]
pub struct SilkConfig {
    /// Application name.
    pub name: String,
    /// Application version.
    #[serde(default = "default_version")]
    pub version: String,
    /// Path to the main process JS file (relative to project root).
    pub main: String,
    /// Directory containing renderer assets (relative to project root).
    #[serde(default = "default_app_dir")]
    pub app: String,
    /// Default window options.
    #[serde(default)]
    pub window: WindowConfig,
    /// URL for dev server (e.g. "http://localhost:5173"). When set and running
    /// a debug build, renderer windows load from this URL instead of the asset server.
    #[serde(rename = "devUrl")]
    pub dev_url: Option<String>,
    /// Path to optional Rust plugin crate (relative to project root).
    pub plugins: Option<String>,
}

/// Default window configuration from silk.config.json.
#[derive(Debug, Deserialize)]
pub struct WindowConfig {
    #[serde(default = "default_width")]
    pub width: f64,
    #[serde(default = "default_height")]
    pub height: f64,
    #[serde(rename = "minWidth")]
    pub min_width: Option<f64>,
    #[serde(rename = "minHeight")]
    pub min_height: Option<f64>,
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default = "default_true")]
    pub resizable: bool,
    #[serde(default = "default_true")]
    pub movable: bool,
    #[serde(rename = "titlebarStyle", default = "default_titlebar_style")]
    pub titlebar_style: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            min_width: None,
            min_height: None,
            title: default_title(),
            resizable: true,
            movable: true,
            titlebar_style: default_titlebar_style(),
        }
    }
}

impl SilkConfig {
    /// Load config from `silk.config.json` in the given project root.
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let config_path = project_root.join("silk.config.json");
        let contents = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", config_path.display()))?;
        let config: Self = serde_json::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("failed to parse silk.config.json: {e}"))?;
        Ok(config)
    }

    /// Resolve the main JS file path relative to project root.
    pub fn main_path(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.main)
    }

    /// Resolve the app asset directory relative to project root.
    pub fn app_dir(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.app)
    }
}

fn default_version() -> String {
    "0.1.0".into()
}
fn default_app_dir() -> String {
    "app".into()
}
fn default_width() -> f64 {
    800.0
}
fn default_height() -> f64 {
    600.0
}
fn default_title() -> String {
    "Silk App".into()
}
fn default_true() -> bool {
    true
}
fn default_titlebar_style() -> String {
    "default".into()
}
