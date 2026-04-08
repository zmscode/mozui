use mozui::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Phase 3 & 4: Data Display & Navigation".into(),
            size: Size::new(800.0, 700.0),
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

    // Tab state
    let (tab_idx, set_tab_idx) = cx.use_signal(0usize);
    let selected_tab = *cx.get(tab_idx);

    // Progress state
    let (prog_val, _set_prog_val) = cx.use_signal(65.0f32);
    let pv = *cx.get(prog_val);

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
                    .child(
                        label("Phase 3 & 4: Data Display & Navigation")
                            .font_size(13.0)
                            .color(muted),
                    ),
            )
            // Content
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .flex_col()
                    .p(32.0)
                    .pt(8.0)
                    .gap(24.0)
                    .overflow_y_scroll(scroll)
                    // ── Tags ───────────────────────────────────────
                    .child(label("Tags").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_row()
                            .gap(8.0)
                            .items_center()
                            .child(tag("Default", &theme))
                            .child(tag("Primary", &theme).primary(&theme))
                            .child(tag("Success", &theme).success(&theme))
                            .child(tag("Warning", &theme).warning(&theme))
                            .child(tag("Danger", &theme).danger(&theme))
                            .child(tag("Info", &theme).info(&theme)),
                    )
                    .child(
                        div()
                            .flex_row()
                            .gap(8.0)
                            .items_center()
                            .child(tag("Outline", &theme).primary(&theme).outline())
                            .child(tag("Pill", &theme).success(&theme).pill())
                            .child(tag("Outline Pill", &theme).danger(&theme).outline().pill()),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Progress ───────────────────────────────────
                    .child(label("Progress").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(
                                label(format!("{}%", pv as u32))
                                    .font_size(13.0)
                                    .color(theme.foreground),
                            )
                            .child(progress(&theme).value(pv))
                            .child(progress(&theme).value(30.0).color(theme.success).small())
                            .child(progress(&theme).value(80.0).color(theme.warning))
                            .child(progress(&theme).value(100.0).color(theme.danger).large()),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Links ──────────────────────────────────────
                    .child(label("Links").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_row()
                            .gap(16.0)
                            .items_center()
                            .child(link("Click me", &theme).on_click(|_cx| {}))
                            .child(
                                link("Custom color", &theme)
                                    .color(theme.success)
                                    .on_click(|_cx| {}),
                            )
                            .child(link("Disabled", &theme).disabled(true)),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Badges ─────────────────────────────────────
                    .child(label("Badges").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_row()
                            .gap(24.0)
                            .items_center()
                            .child(
                                badge(&theme)
                                    .dot()
                                    .child(icon(IconName::Bell).color(theme.foreground)),
                            )
                            .child(
                                badge(&theme)
                                    .count(5)
                                    .child(icon(IconName::Envelope).color(theme.foreground)),
                            )
                            .child(
                                badge(&theme)
                                    .count(128)
                                    .max(99)
                                    .child(icon(IconName::Bell).color(theme.foreground)),
                            ),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Tabs ───────────────────────────────────────
                    .child(label("Tabs").font_size(20.0).bold().color(heading))
                    .child(
                        tab_bar(&theme)
                            .child(
                                tab("General", &theme)
                                    .icon(IconName::Gear)
                                    .selected(selected_tab == 0)
                                    .on_click(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.set(set_tab_idx, 0);
                                    }),
                            )
                            .child(
                                tab("Appearance", &theme)
                                    .icon(IconName::Eye)
                                    .selected(selected_tab == 1)
                                    .on_click(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.set(set_tab_idx, 1);
                                    }),
                            )
                            .child(
                                tab("Keybindings", &theme)
                                    .selected(selected_tab == 2)
                                    .on_click(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.set(set_tab_idx, 2);
                                    }),
                            )
                            .child(tab("Disabled", &theme).disabled(true)),
                    )
                    .child(
                        div()
                            .p(16.0)
                            .bg(theme.surface)
                            .rounded(8.0)
                            .child(
                                label(match selected_tab {
                                    0 => "General settings content",
                                    1 => "Appearance settings content",
                                    2 => "Keybinding settings content",
                                    _ => "",
                                })
                                .color(theme.foreground),
                            ),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Breadcrumb ─────────────────────────────────
                    .child(label("Breadcrumb").font_size(20.0).bold().color(heading))
                    .child(
                        breadcrumb(&theme)
                            .child(breadcrumb_item("Home").icon(IconName::House).on_click(|_| {}))
                            .child(breadcrumb_item("Settings").on_click(|_| {}))
                            .child(breadcrumb_item("Profile")),
                    ),
            ),
    )
}
