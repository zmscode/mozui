use std::collections::HashMap;
use std::path::PathBuf;

use mozui::{Entity, Pixels, Point, Size, WindowBackgroundAppearance, px};
use mozui_webview::WebView;

use crate::commands::window::TrafficLightPosition;
use crate::config::WindowConfig;

/// Tracks a renderer window opened by the main process.
pub(crate) struct ManagedWindow {
    pub label: String,
    pub webview: Entity<WebView>,
}

/// Manages all renderer windows. Shared via Rc<RefCell<>> so IPC handlers can access it.
pub(crate) struct WindowManager {
    pub renderer_windows: HashMap<String, ManagedWindow>,
    pub main_webview: Option<Entity<WebView>>,
    pub asset_root: PathBuf,
    pub defaults: WindowConfig,
    pub dev_url: Option<String>,
}

impl WindowManager {
    pub fn new(asset_root: PathBuf, defaults: WindowConfig, dev_url: Option<String>) -> Self {
        Self {
            renderer_windows: HashMap::new(),
            main_webview: None,
            asset_root,
            defaults,
            dev_url,
        }
    }

    pub fn register(&mut self, label: String, webview: Entity<WebView>) {
        self.renderer_windows
            .insert(label.clone(), ManagedWindow { label, webview });
    }

    pub fn remove(&mut self, label: &str) -> Option<ManagedWindow> {
        self.renderer_windows.remove(label)
    }

    pub fn get(&self, label: &str) -> Option<&ManagedWindow> {
        self.renderer_windows.get(label)
    }

    /// Build WindowOptions from JS args merged with defaults.
    pub fn build_window_options(
        &self,
        title: Option<String>,
        width: Option<f64>,
        height: Option<f64>,
        min_width: Option<f64>,
        min_height: Option<f64>,
        resizable: Option<bool>,
        titlebar_style: Option<String>,
        traffic_light_position: Option<TrafficLightPosition>,
        cx: &mozui::App,
    ) -> mozui::WindowOptions {
        let w = width.unwrap_or(self.defaults.width);
        let h = height.unwrap_or(self.defaults.height);
        let t = title.unwrap_or_else(|| self.defaults.title.clone());
        let style = titlebar_style.unwrap_or_else(|| self.defaults.titlebar_style.clone());

        let window_size: Size<Pixels> = mozui::size(px(w as f32), px(h as f32));

        let appears_transparent = matches!(style.as_str(), "hidden" | "hiddenInset");

        let tl_position = traffic_light_position.map(|pos| {
            Point {
                x: px(pos.x as f32),
                y: px(pos.y as f32),
            }
        });

        let window_background = if appears_transparent {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Opaque
        };

        let mut opts = mozui::WindowOptions {
            window_bounds: Some(mozui::WindowBounds::centered(window_size, cx)),
            titlebar: Some(mozui::TitlebarOptions {
                title: if appears_transparent { None } else { Some(t.into()) },
                appears_transparent,
                traffic_light_position: tl_position,
            }),
            window_background,
            focus: true,
            show: true,
            is_resizable: resizable.unwrap_or(self.defaults.resizable),
            is_movable: self.defaults.movable,
            ..Default::default()
        };

        let mw = min_width.or(self.defaults.min_width);
        let mh = min_height.or(self.defaults.min_height);
        if let (Some(mw), Some(mh)) = (mw, mh) {
            opts.window_min_size = Some(mozui::size(px(mw as f32), px(mh as f32)));
        }

        opts
    }
}
