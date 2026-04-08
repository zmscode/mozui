use mozui::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "mozui — Form Example".into(),
            size: Size::new(600.0, 500.0),
            ..Default::default()
        })
        .run(app);
}

fn app(cx: &mut Context) -> Box<dyn Element> {
    let (name_state, set_name) = cx.use_signal(TextInputState::new());
    let (email_state, set_email) = cx.use_signal(TextInputState::new());
    let (msg_state, set_msg) = cx.use_signal(TextInputState::new());

    let name = cx.get(name_state).clone();
    let email = cx.get(email_state).clone();
    let msg = cx.get(msg_state).clone();

    // Preview text
    let preview = format!(
        "Name: {} | Email: {} | Message: {}",
        if name.value.is_empty() { "—" } else { &name.value },
        if email.value.is_empty() { "—" } else { &email.value },
        if msg.value.is_empty() { "—" } else { &msg.value },
    );

    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(20.0)
            .bg(Color::hex("#1e1e2e"))
            .on_key_down(move |key, _mods, cx_any| {
                let cx = cx_any.downcast_mut::<Context>().unwrap();
                if key == Key::Escape {
                    std::process::exit(0);
                }
                let _ = cx;
            })
            .child(
                text("Contact Form")
                    .font_size(28.0)
                    .bold()
                    .color(Color::hex("#cdd6f4")),
            )
            // Form fields
            .child(
                div()
                    .flex_col()
                    .gap(12.0)
                    .child(label_and_input("Name", name, set_name, "Enter your name..."))
                    .child(label_and_input("Email", email, set_email, "you@example.com"))
                    .child(label_and_input("Message", msg, set_msg, "Type a message...")),
            )
            // Preview
            .child(
                div()
                    .w(340.0)
                    .p(12.0)
                    .bg(Color::hex("#181825"))
                    .rounded(8.0)
                    .child(
                        text(preview)
                            .font_size(12.0)
                            .color(Color::hex("#a6adc8")),
                    ),
            )
            // Hint
            .child(
                text("Tab to switch fields | Esc to quit")
                    .font_size(11.0)
                    .color(Color::hex("#6c7086")),
            ),
    )
}

fn label_and_input(
    label_text: &str,
    state: TextInputState,
    setter: SetSignal<TextInputState>,
    placeholder_text: &str,
) -> Div {
    div()
        .flex_col()
        .gap(4.0)
        .child(
            text(label_text)
                .font_size(13.0)
                .color(Color::hex("#bac2de")),
        )
        .child(
            text_input(state)
                .w(320.0)
                .placeholder(placeholder_text)
                .on_change(move |f, cx_any| {
                    let cx = cx_any.downcast_mut::<Context>().unwrap();
                    cx.update(setter, f);
                }),
        )
}
