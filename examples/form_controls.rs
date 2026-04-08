use mozui::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Phase 2: Form Controls".into(),
            size: Size::new(750.0, 700.0),
            titlebar: TitlebarStyle::Transparent,
            ..Default::default()
        })
        .run(app);
}

fn app(cx: &mut Context) -> Box<dyn Element> {
    let theme = cx.theme().clone();
    let scroll = cx.use_scroll();
    let heading = Color::hex("#cdd6f4");
    let muted = Color::hex("#6c7086");

    // Checkbox state
    let (check1, set_check1) = cx.use_signal(false);
    let (check2, set_check2) = cx.use_signal(true);
    let checked1 = *cx.get(check1);
    let checked2 = *cx.get(check2);

    // Radio state
    let (radio_idx, set_radio_idx) = cx.use_signal(0usize);
    let selected = *cx.get(radio_idx);

    // Switch state
    let (sw1, set_sw1) = cx.use_signal(false);
    let (sw2, set_sw2) = cx.use_signal(true);
    let sw1_on = *cx.get(sw1);
    let sw2_on = *cx.get(sw2);

    // Slider state
    let (slider_val, set_slider_val) = cx.use_signal(40.0f32);
    let (slider_val2, set_slider_val2) = cx.use_signal(75.0f32);
    let sv1 = *cx.get(slider_val);
    let sv2 = *cx.get(slider_val2);

    // Stepper state
    let (step_idx, set_step_idx) = cx.use_signal(1usize);
    let current_step = *cx.get(step_idx);

    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_col()
            .bg(Color::hex("#1e1e2e"))
            .on_key_down(|key, _mods, _cx| {
                if key == Key::Escape {
                    std::process::exit(0);
                }
            })
            // Title bar
            .child(
                div()
                    .w_full()
                    .h(38.0)
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .drag_region()
                    .child(label("Phase 2: Form Controls").font_size(13.0).color(muted)),
            )
            // Content (scrollable)
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .flex_col()
                    .p(32.0)
                    .pt(8.0)
                    .gap(28.0)
                    .overflow_y_scroll(scroll)
                    // ── Buttons ─────────────────────────────────
                    .child(label("Buttons").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(
                                div()
                                    .flex_row()
                                    .gap(8.0)
                                    .items_center()
                                    .child(button("Default", &theme))
                                    .child(button("Primary", &theme).primary(&theme))
                                    .child(button("Danger", &theme).danger(&theme))
                                    .child(button("Ghost", &theme).ghost(&theme))
                                    .child(button("Outline", &theme).outline(&theme)),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .gap(8.0)
                                    .items_center()
                                    .child(
                                        button("With Icon", &theme)
                                            .primary(&theme)
                                            .icon(IconName::Star),
                                    )
                                    .child(
                                        button("Disabled", &theme).primary(&theme).disabled(true),
                                    )
                                    .child(icon_button(IconName::Gear, &theme))
                                    .child(icon_button(IconName::Trash, &theme))
                                    .child(icon_button(IconName::Copy, &theme)),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .gap(8.0)
                                    .items_center()
                                    .child(button("Small", &theme).primary(&theme).small())
                                    .child(button("Medium", &theme).primary(&theme))
                                    .child(button("Large", &theme).primary(&theme).large()),
                            ),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Checkboxes ──────────────────────────────
                    .child(label("Checkboxes").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(
                                checkbox(&theme)
                                    .label("Accept terms and conditions")
                                    .checked(checked1)
                                    .on_click(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.update(set_check1, |v| *v = !*v);
                                    }),
                            )
                            .child(
                                checkbox(&theme)
                                    .label("Subscribe to newsletter")
                                    .checked(checked2)
                                    .on_click(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.update(set_check2, |v| *v = !*v);
                                    }),
                            )
                            .child(
                                checkbox(&theme)
                                    .label("Disabled checkbox")
                                    .checked(true)
                                    .disabled(true),
                            ),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Radio ───────────────────────────────────
                    .child(label("Radio").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(radio("Option A", &theme).checked(selected == 0).on_click(
                                move |cx_any| {
                                    let cx = cx_any.downcast_mut::<Context>().unwrap();
                                    cx.set(set_radio_idx, 0);
                                },
                            ))
                            .child(radio("Option B", &theme).checked(selected == 1).on_click(
                                move |cx_any| {
                                    let cx = cx_any.downcast_mut::<Context>().unwrap();
                                    cx.set(set_radio_idx, 1);
                                },
                            ))
                            .child(
                                radio("Option C (disabled)", &theme)
                                    .checked(selected == 2)
                                    .disabled(true),
                            ),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Switches ────────────────────────────────
                    .child(label("Switches").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(switch(&theme).label("Dark mode").checked(sw1_on).on_click(
                                move |cx_any| {
                                    let cx = cx_any.downcast_mut::<Context>().unwrap();
                                    cx.update(set_sw1, |v| *v = !*v);
                                },
                            ))
                            .child(
                                switch(&theme)
                                    .label("Notifications")
                                    .checked(sw2_on)
                                    .on_click(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.update(set_sw2, |v| *v = !*v);
                                    }),
                            )
                            .child(
                                switch(&theme)
                                    .label("Disabled switch")
                                    .checked(true)
                                    .disabled(true),
                            ),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Sliders ─────────────────────────────────
                    .child(label("Sliders").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(16.0)
                            .child(
                                div().flex_row().gap(12.0).items_center().child(
                                    label(format!("Volume: {:.0}%", sv1))
                                        .font_size(13.0)
                                        .color(theme.foreground),
                                ),
                            )
                            .child(
                                slider(&theme)
                                    .min(0.0)
                                    .max(100.0)
                                    .step(1.0)
                                    .value(sv1)
                                    .on_change(move |val, cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.set(set_slider_val, val);
                                    }),
                            )
                            .child(
                                div().flex_row().gap(12.0).items_center().child(
                                    label(format!("Brightness: {:.0}%", sv2))
                                        .font_size(13.0)
                                        .color(theme.foreground),
                                ),
                            )
                            .child(
                                slider(&theme)
                                    .min(0.0)
                                    .max(100.0)
                                    .step(5.0)
                                    .value(sv2)
                                    .on_change(move |val, cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.set(set_slider_val2, val);
                                    }),
                            )
                            .child(
                                slider(&theme)
                                    .min(0.0)
                                    .max(100.0)
                                    .value(50.0)
                                    .disabled(true),
                            ),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Stepper ─────────────────────────────────
                    .child(label("Stepper").font_size(20.0).bold().color(heading))
                    .child(
                        stepper(&theme)
                            .current(current_step)
                            .item(StepperItem::new("Account"))
                            .item(StepperItem::new("Profile"))
                            .item(StepperItem::new("Review"))
                            .item(StepperItem::new("Complete"))
                            .on_click(move |idx, cx_any| {
                                let cx = cx_any.downcast_mut::<Context>().unwrap();
                                cx.set(set_step_idx, idx);
                            }),
                    ),
            ),
    )
}
