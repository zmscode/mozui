use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct TrafficLightPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize)]
pub(crate) struct CreateWindowArgs {
    pub label: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    #[serde(rename = "minWidth")]
    pub min_width: Option<f64>,
    #[serde(rename = "minHeight")]
    pub min_height: Option<f64>,
    pub resizable: Option<bool>,
    /// "default" | "hidden" | "hiddenInset"
    #[serde(rename = "titlebarStyle")]
    pub titlebar_style: Option<String>,
    /// Position of macOS traffic light buttons (only used when titlebarStyle is "hiddenInset")
    #[serde(rename = "trafficLightPosition")]
    pub traffic_light_position: Option<TrafficLightPosition>,
}

#[derive(Deserialize)]
pub(crate) struct WindowLabelArgs {
    pub label: String,
}

#[derive(Deserialize)]
pub(crate) struct SetTitleArgs {
    pub label: String,
    pub title: String,
}

#[derive(Deserialize)]
pub(crate) struct EmitToArgs {
    pub label: String,
    pub event: String,
    pub payload: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub(crate) struct EmitAllArgs {
    pub event: String,
    pub payload: Option<serde_json::Value>,
}
