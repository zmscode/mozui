use mozui::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Hello mozui".into(),
            size: Size::new(800.0, 600.0),
            ..Default::default()
        })
        .run(app);
}

fn app(cx: &mut Context) -> Box<dyn Element> {
    let (count, set_count) = cx.use_signal(0i32);
    let current = *cx.get(count);

    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(16.0)
            .bg(Color::hex("#1e1e2e"))
            .on_key_down(move |key, _mods, cx_any| {
                let cx = cx_any.downcast_mut::<Context>().unwrap();
                match key {
                    Key::ArrowUp => cx.update(set_count, |n| *n += 1),
                    Key::ArrowDown => cx.update(set_count, |n| *n -= 1),
                    Key::Escape => std::process::exit(0),
                    _ => {}
                }
            })
            .child(
                text("Hello, mozui!")
                    .font_size(32.0)
                    .bold()
                    .color(Color::hex("#cdd6f4")),
            )
            .child(
                text(format!("Count: {current}"))
                    .font_size(24.0)
                    .color(Color::hex("#a6e3a1")),
            )
            .child(
                text("↑/↓ keys or click buttons")
                    .font_size(12.0)
                    .color(Color::hex("#6c7086")),
            )
            .child(
                div()
                    .flex_row()
                    .gap(12.0)
                    .child(
                        div()
                            .w(120.0)
                            .h(40.0)
                            .bg(Color::hex("#3b82f6"))
                            .rounded(8.0)
                            .items_center()
                            .justify_center()
                            .on_click(move |cx_any| {
                                let cx = cx_any.downcast_mut::<Context>().unwrap();
                                cx.update(set_count, |n| *n += 1);
                            })
                            .child(text("Increment").font_size(14.0).color(Color::WHITE)),
                    )
                    .child(
                        div()
                            .w(120.0)
                            .h(40.0)
                            .bg(Color::hex("#ef4444"))
                            .rounded(8.0)
                            .items_center()
                            .justify_center()
                            .on_click(move |cx_any| {
                                let cx = cx_any.downcast_mut::<Context>().unwrap();
                                cx.set(set_count, 0);
                            })
                            .child(text("Reset").font_size(14.0).color(Color::WHITE)),
                    ),
            ),
    )
}
