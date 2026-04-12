use std::path::PathBuf;

use mozui::prelude::*;
use mozui::{
    App as MozuiApp, Application, WindowBounds, WindowOptions, current_platform, px, size,
};
use mozui_components::Root;

use mozui_builder::builder::BuilderView;
use mozui_builder::document::DocumentTree;
use mozui_builder::renderer::DocumentRenderer;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: mozui-builder <command> [args]");
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  edit [file.mozui.json]    Open the builder (creates new if no file)");
        eprintln!("  open <file.mozui.json>    Render a layout file (viewer only)");
        eprintln!("  new <file.mozui.json>     Create a new empty layout file");
        std::process::exit(1);
    }

    let command = args[1].as_str();

    match command {
        "edit" => {
            let path = args.get(2).map(PathBuf::from);
            launch_builder(path);
        }
        "open" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Usage: mozui-builder open <file.mozui.json>");
                std::process::exit(1);
            });
            open_viewer(PathBuf::from(path));
        }
        "new" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Usage: mozui-builder new <file.mozui.json>");
                std::process::exit(1);
            });
            create_new_file(PathBuf::from(path));
        }
        _ => {
            let path = PathBuf::from(command);
            if path.exists() {
                // Default: open in builder
                launch_builder(Some(path));
            } else {
                eprintln!("Unknown command: {command}");
                eprintln!("Run without arguments for usage info.");
                std::process::exit(1);
            }
        }
    }
}

fn launch_builder(path: Option<PathBuf>) {
    let (tree, file_path) = match &path {
        Some(p) if p.exists() => {
            let tree = DocumentTree::load_from_path(p).unwrap_or_else(|e| {
                eprintln!("Failed to load {}: {e:#}", p.display());
                std::process::exit(1);
            });
            (tree, Some(p.clone()))
        }
        Some(p) => {
            // File doesn't exist yet — create a new tree, will save on first save
            (DocumentTree::new(), Some(p.clone()))
        }
        None => (DocumentTree::new(), None),
    };

    run_app(size(px(1200.0), px(800.0)), move |window, cx| {
        let builder = cx.new(|cx| BuilderView::new(tree, file_path, window, cx));
        cx.new(|cx| Root::new(builder, window, cx))
    });
}

fn open_viewer(path: PathBuf) {
    let tree = DocumentTree::load_from_path(&path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {e:#}", path.display());
        std::process::exit(1);
    });

    run_app(size(px(900.0), px(700.0)), move |window, cx| {
        let viewer = cx.new(|_cx| DocumentRenderer::new(tree));
        cx.new(|cx| Root::new(viewer, window, cx))
    });
}

fn create_new_file(path: PathBuf) {
    let tree = DocumentTree::new();
    tree.save_to_path(&path).unwrap_or_else(|e| {
        eprintln!("Failed to create {}: {e:#}", path.display());
        std::process::exit(1);
    });
    println!("Created {}", path.display());
}

fn run_app<V: 'static + Render>(
    window_size: mozui::Size<mozui::Pixels>,
    build_view: impl FnOnce(&mut mozui::Window, &mut MozuiApp) -> mozui::Entity<V> + 'static,
) {
    let platform = current_platform(false);
    Application::with_platform(platform).run(move |cx: &mut MozuiApp| {
        mozui_components::init(cx);
        mozui_components::theme::Theme::change(mozui_components::theme::ThemeMode::Dark, None, cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(window_size, cx)),
                focus: true,
                show: true,
                ..Default::default()
            },
            build_view,
        )
        .expect("failed to open window");
    });
}
