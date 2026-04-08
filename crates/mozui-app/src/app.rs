use crate::context::Context;
use mozui_elements::{Element, InteractionMap};
use mozui_events::PlatformEvent;
use mozui_layout::LayoutEngine;
use mozui_platform::{WindowOptions, create_platform};
use mozui_renderer::{DrawList, Renderer};
use mozui_style::Theme;
use taffy::prelude::*;

pub struct App {
    theme: Theme,
    window_options: WindowOptions,
}

impl App {
    pub fn new() -> Self {
        Self {
            theme: Theme::dark(),
            window_options: WindowOptions::default(),
        }
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn window(mut self, options: WindowOptions) -> Self {
        self.window_options = options;
        self
    }

    pub fn run<F>(self, root: F) -> !
    where
        F: Fn(&mut Context) -> Box<dyn Element> + 'static,
    {
        let mut platform = create_platform();
        let window = platform.open_window(self.window_options);

        let mut renderer = Renderer::new(window.as_ref());
        let mut cx = Context::new(self.theme.clone());
        let mut layout_engine = LayoutEngine::new();
        let bg_color = self.theme.background;

        let mut needs_render = true;
        let mut interactions = InteractionMap::new();

        // Build initial element tree
        cx.reset_hooks();
        let mut element_tree: Box<dyn Element> = root(&mut cx);

        platform.run(Box::new(move |event| {
            match event {
                PlatformEvent::RedrawRequested => {
                    if needs_render {
                        let size = window.content_size();
                        let scale = window.scale_factor();

                        // Phase 1: Build Taffy layout tree
                        layout_engine.clear();
                        let root_node =
                            element_tree.layout(&mut layout_engine, renderer.font_system());

                        // Phase 2: Compute layout
                        layout_engine.compute_layout(
                            root_node,
                            taffy::prelude::Size {
                                width: AvailableSpace::Definite(size.width),
                                height: AvailableSpace::Definite(size.height),
                            },
                        );

                        // Phase 3: Collect absolute positions
                        let layouts = layout_engine.collect_layouts(root_node);

                        // Phase 4: Paint with computed layouts + collect interactions
                        let mut draw_list = DrawList::new();
                        interactions.clear();
                        let mut index = 0;
                        element_tree.paint(&layouts, &mut index, &mut draw_list, &mut interactions);

                        renderer.render(bg_color, &draw_list, size, scale);
                        needs_render = false;
                    }
                }
                PlatformEvent::MouseUp {
                    button: mozui_events::MouseButton::Left,
                    position,
                    ..
                } => {
                    if interactions.dispatch_click(position, &mut cx) {
                        // Signal was mutated — rebuild tree
                        if cx.is_dirty() {
                            cx.clear_dirty();
                            cx.reset_hooks();
                            element_tree = root(&mut cx);
                            needs_render = true;
                            window.request_redraw();
                        }
                    }
                }
                PlatformEvent::KeyDown { key, modifiers, .. } => {
                    interactions.dispatch_key(key, modifiers, &mut cx);
                    if cx.is_dirty() {
                        cx.clear_dirty();
                        cx.reset_hooks();
                        element_tree = root(&mut cx);
                        needs_render = true;
                        window.request_redraw();
                    }
                }
                PlatformEvent::MouseMove { position, .. } => {
                    if interactions.has_handler_at(position) {
                        mozui_platform::set_cursor_style(mozui_events::CursorStyle::Hand);
                    } else {
                        mozui_platform::set_cursor_style(mozui_events::CursorStyle::Arrow);
                    }
                }
                PlatformEvent::WindowResize { size } => {
                    let scale = window.scale_factor();
                    renderer.resize(size, scale);
                    cx.reset_hooks();
                    element_tree = root(&mut cx);
                    needs_render = true;
                    window.request_redraw();
                }
                PlatformEvent::ScaleFactorChanged { scale } => {
                    let size = window.content_size();
                    renderer.resize(size, scale);
                    needs_render = true;
                    window.request_redraw();
                }
                PlatformEvent::WindowCloseRequested => {
                    std::process::exit(0);
                }
                _ => {
                    tracing::trace!(?event, "platform event");
                }
            }
        }))
    }
}
