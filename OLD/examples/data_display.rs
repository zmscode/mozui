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
    let heading = theme.foreground;
    let muted = theme.muted_foreground;

    // Tab state
    let (tab_idx, set_tab_idx) = cx.use_signal(0usize);
    let selected_tab = *cx.get(tab_idx);

    // Progress state
    let (prog_val, _set_prog_val) = cx.use_signal(65.0f32);
    let pv = *cx.get(prog_val);

    // Pagination state
    let (page_sig, set_page_sig) = cx.use_signal(1usize);
    let current_page = *cx.get(page_sig);

    // Rating state
    let (rating_sig, set_rating_sig) = cx.use_signal(3.5f32);
    let rating_val = *cx.get(rating_sig);

    // List state
    let (list_sel, set_list_sel) = cx.use_signal(0usize);
    let selected_list = *cx.get(list_sel);

    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_col()
            .bg(theme.background)
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
                    .child(divider().color(theme.border))
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
                    .child(divider().color(theme.border))
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
                    .child(divider().color(theme.border))
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
                    .child(divider().color(theme.border))
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
                        div().p(16.0).bg(theme.surface).rounded(8.0).child(
                            label(match selected_tab {
                                0 => "General settings content",
                                1 => "Appearance settings content",
                                2 => "Keybinding settings content",
                                _ => "",
                            })
                            .color(theme.foreground),
                        ),
                    )
                    .child(divider().color(theme.border))
                    // ── Breadcrumb ─────────────────────────────────
                    .child(label("Breadcrumb").font_size(20.0).bold().color(heading))
                    .child(
                        breadcrumb(&theme)
                            .child(
                                breadcrumb_item("Home")
                                    .icon(IconName::House)
                                    .on_click(|_| {}),
                            )
                            .child(breadcrumb_item("Settings").on_click(|_| {}))
                            .child(breadcrumb_item("Profile")),
                    )
                    .child(divider().color(theme.border))
                    // ── Rating ─────────────────────────────────────
                    .child(label("Rating").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(
                                div()
                                    .flex_row()
                                    .gap(16.0)
                                    .items_center()
                                    .child(rating(&theme).value(rating_val).on_change(
                                        move |val, cx_any| {
                                            let cx = cx_any.downcast_mut::<Context>().unwrap();
                                            cx.set(set_rating_sig, val);
                                        },
                                    ))
                                    .child(
                                        label(format!("{:.1}", rating_val))
                                            .font_size(14.0)
                                            .color(theme.foreground),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .gap(16.0)
                                    .items_center()
                                    .child(rating(&theme).value(4.0).small())
                                    .child(rating(&theme).value(2.5).large()),
                            ),
                    )
                    .child(divider().color(theme.border))
                    // ── Pagination ─────────────────────────────────
                    .child(label("Pagination").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(16.0)
                            .child(
                                pagination(&theme)
                                    .current_page(current_page)
                                    .total_pages(10)
                                    .on_click(move |page, cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.set(set_page_sig, page);
                                    }),
                            )
                            .child(
                                pagination(&theme)
                                    .current_page(current_page)
                                    .total_pages(10)
                                    .compact()
                                    .on_click(move |page, cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.set(set_page_sig, page);
                                    }),
                            ),
                    )
                    .child(divider().color(theme.border))
                    // ── Description List ──────────────────────────
                    .child(
                        label("Description List")
                            .font_size(20.0)
                            .bold()
                            .color(heading),
                    )
                    .child(
                        description_list(&theme)
                            .bordered(true)
                            .child(description_item("Name").value("John Doe"))
                            .child(description_item("Email").value("john@example.com"))
                            .child(description_item("Role").value("Administrator"))
                            .child(description_item("Status").value("Active")),
                    )
                    .child(divider().color(theme.border))
                    // ── Group Box ─────────────────────────────────
                    .child(label("Group Box").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_row()
                            .gap(16.0)
                            .child(
                                group_box("Account Settings", &theme)
                                    .child(label("Username: admin").color(theme.foreground))
                                    .child(
                                        label("Email: admin@example.com").color(theme.foreground),
                                    ),
                            )
                            .child(
                                group_box("Preferences", &theme)
                                    .fill()
                                    .child(label("Theme: Dark").color(theme.foreground))
                                    .child(label("Language: English").color(theme.foreground)),
                            ),
                    )
                    .child(divider().color(theme.border))
                    // ── List ──────────────────────────────────────
                    .child(label("List").font_size(20.0).bold().color(heading))
                    .child(
                        div().bg(theme.surface).rounded(8.0).p(4.0).child(
                            list(&theme)
                                .child(
                                    list_item("Inbox")
                                        .icon(IconName::Envelope)
                                        .description("12 unread messages")
                                        .selected(selected_list == 0)
                                        .on_click(move |cx_any| {
                                            let cx = cx_any.downcast_mut::<Context>().unwrap();
                                            cx.set(set_list_sel, 0);
                                        }),
                                )
                                .child(
                                    list_item("Starred")
                                        .icon(IconName::Star)
                                        .description("Important items")
                                        .selected(selected_list == 1)
                                        .on_click(move |cx_any| {
                                            let cx = cx_any.downcast_mut::<Context>().unwrap();
                                            cx.set(set_list_sel, 1);
                                        }),
                                )
                                .child(list_item("").separator())
                                .child(
                                    list_item("Archive")
                                        .icon(IconName::Folder)
                                        .selected(selected_list == 2)
                                        .on_click(move |cx_any| {
                                            let cx = cx_any.downcast_mut::<Context>().unwrap();
                                            cx.set(set_list_sel, 2);
                                        }),
                                )
                                .child(list_item("Trash").icon(IconName::Trash).disabled(true)),
                        ),
                    ),
            ),
    )
}
