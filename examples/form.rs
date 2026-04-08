use mozui::*;

actions!(app, [Quit]);

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "mozui — Form".into(),
            size: Size::new(600.0, 500.0),
            ..Default::default()
        })
        .keybindings(|kb| {
            kb.bind("escape", Quit);
            kb.bind("cmd-q", Quit);
        })
        .on_action(|action, _cx| {
            if action.as_any().is::<Quit>() {
                std::process::exit(0);
            }
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
            .bg(Color::hex("#1e1e2e"))
            // Custom title bar
            .child(title_bar("Contact Form"))
            // Content area
            .child(
                div()
                    .flex_1()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(20.0)
                    .child(
                        text("Contact Form")
                            .font_size(28.0)
                            .bold()
                            .color(Color::hex("#cdd6f4")),
                    )
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(label_and_input("Name", name, set_name, "Enter your name..."))
                            .child(label_and_input("Email", email, set_email, "you@example.com"))
                            .child(label_and_input("Message", msg, set_msg, "Type a message...")),
                    )
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
                    .child(
                        text("Tab/Shift+Tab to switch | Cmd+C/V/X clipboard | Esc to quit")
                            .font_size(11.0)
                            .color(Color::hex("#6c7086")),
                    ),
            ),
    )
}

/// Custom title bar with drag region and window title.
fn title_bar(title: &str) -> Div {
    div()
        .w_full()
        .h(38.0)
        .flex_row()
        .items_center()
        .justify_center()
        .bg(Color::hex("#181825"))
        .drag_region()
        // macOS traffic lights take ~70px on the left
        .pl(70.0)
        .pr(70.0)
        .child(
            text(title)
                .font_size(13.0)
                .color(Color::hex("#6c7086")),
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
