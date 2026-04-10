use mozui::*;
use std::time::Duration;

actions!(app, [Quit]);

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "mozui — Timer Example".into(),
            size: Size::new(400.0, 300.0),
            titlebar: TitlebarStyle::Transparent,
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
    let theme = cx.theme().clone();
    let (seconds, set_seconds) = cx.use_signal(0u32);
    let (running, set_running) = cx.use_signal(false);
    let (timer_id, set_timer_id) = cx.use_signal(None::<TimerId>);

    let secs = *cx.get(seconds);
    let is_running = *cx.get(running);
    let _tid = *cx.get(timer_id);

    let mins = secs / 60;
    let display_secs = secs % 60;
    let time_str = format!("{:02}:{:02}", mins, display_secs);

    let button_label = if is_running { "Pause" } else { "Start" };
    let button_color = if is_running {
        theme.danger
    } else {
        theme.success
    };

    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(24.0)
            .bg(theme.background)
            .child(
                text("Stopwatch")
                    .font_size(20.0)
                    .bold()
                    .color(theme.foreground),
            )
            .child(
                text(time_str)
                    .font_size(64.0)
                    .bold()
                    .color(theme.foreground),
            )
            .child(
                div()
                    .flex_row()
                    .gap(12.0)
                    .child(
                        div()
                            .w(100.0)
                            .h(40.0)
                            .bg(button_color)
                            .rounded(8.0)
                            .items_center()
                            .justify_center()
                            .on_click(move |cx_any| {
                                let cx = cx_any.downcast_mut::<Context>().unwrap();
                                let currently_running = *cx.get(running);
                                if currently_running {
                                    // Pause — cancel timer
                                    if let Some(tid) = *cx.get(timer_id) {
                                        cx.cancel_timer(tid);
                                    }
                                    cx.set(set_timer_id, None);
                                    cx.set(set_running, false);
                                } else {
                                    // Start — set interval
                                    let tid =
                                        cx.set_interval(Duration::from_secs(1), move |cx_any| {
                                            let cx = cx_any.downcast_mut::<Context>().unwrap();
                                            cx.update(set_seconds, |n| *n += 1);
                                        });
                                    cx.set(set_timer_id, Some(tid));
                                    cx.set(set_running, true);
                                }
                            })
                            .child(text(button_label).font_size(14.0).color(theme.background)),
                    )
                    .child(
                        div()
                            .w(100.0)
                            .h(40.0)
                            .bg(theme.secondary_hover)
                            .rounded(8.0)
                            .items_center()
                            .justify_center()
                            .on_click(move |cx_any| {
                                let cx = cx_any.downcast_mut::<Context>().unwrap();
                                // Reset
                                if let Some(tid) = *cx.get(timer_id) {
                                    cx.cancel_timer(tid);
                                }
                                cx.set(set_timer_id, None);
                                cx.set(set_running, false);
                                cx.set(set_seconds, 0);
                            })
                            .child(text("Reset").font_size(14.0).color(theme.foreground)),
                    ),
            )
            .child(
                text("Esc to quit")
                    .font_size(11.0)
                    .color(theme.muted_foreground),
            ),
    )
}
