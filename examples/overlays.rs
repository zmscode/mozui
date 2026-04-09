use mozui::*;
use std::time::Duration;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Overlays, Shadows & Gradients".into(),
            size: Size::new(800.0, 700.0),
            titlebar: TitlebarStyle::Transparent,
            ..Default::default()
        })
        .run(app);
}

fn app(cx: &mut Context) -> Box<dyn Element> {
    let theme = cx.theme().clone();
    let scroll = cx.use_scroll();

    // Dialog state — animation handle persists across frames
    let (dialog_open, set_dialog_open) = cx.use_signal(false);
    let is_dialog_open = *cx.get(dialog_open);
    let (dialog_anim_sig, set_dialog_anim_sig) =
        cx.use_signal::<Option<Animated<f32>>>(None);
    let dialog_anim_handle = cx.get(dialog_anim_sig).clone();

    // Menu state
    let (menu_open, set_menu_open) = cx.use_signal(false);
    let is_menu_open = *cx.get(menu_open);

    // Notification state: Vec of (id, type, title, message, animation_handle)
    type NotifEntry = (
        usize,
        NotificationType,
        &'static str,
        &'static str,
        Animated<f32>,
    );
    let (notif_list, set_notif_list) = cx.use_signal::<Vec<NotifEntry>>(Vec::new());
    let notifications = cx.get(notif_list).clone();
    let (notif_counter, set_notif_counter) = cx.use_signal(0usize);

    let anim_flag = cx.animation_flag();

    let heading_color = theme.foreground;
    let muted = theme.muted_foreground;

    let mut root = div()
        .w_full()
        .h_full()
        .flex_col()
        .bg(theme.background)
        .on_key_down(|key, _mods, _cx| {
            if key == Key::Escape {
                std::process::exit(0);
            }
        });

    // Title bar
    root = root.child(
        div()
            .w_full()
            .h(38.0)
            .flex_row()
            .items_center()
            .justify_center()
            .drag_region()
            .child(
                label("Overlays, Shadows & Gradients")
                    .color(muted)
                    .font_size(12.0),
            ),
    );

    // Scrollable content
    let content =
        div()
            .w_full()
            .flex_1()
            .flex_col()
            .overflow_y_scroll(scroll)
            .p(24.0)
            .gap(32.0)
            // ── Shadows ──
            .child(
                div()
                    .flex_col()
                    .gap(12.0)
                    .child(label("Shadows").color(heading_color).font_size(18.0).bold())
                    .child(
                        div()
                            .flex_row()
                            .gap(24.0)
                            .child(shadow_card("Small", theme.shadow_sm, &theme))
                            .child(shadow_card("Medium", theme.shadow_md, &theme))
                            .child(shadow_card("Large", theme.shadow_lg, &theme)),
                    ),
            )
            // ── Gradients ──
            .child(
                div()
                    .flex_col()
                    .gap(12.0)
                    .child(
                        label("Gradients")
                            .color(heading_color)
                            .font_size(18.0)
                            .bold(),
                    )
                    .child(
                        div()
                            .flex_row()
                            .gap(16.0)
                            .flex_wrap()
                            .child(gradient_card(
                                "Linear 0deg",
                                Fill::LinearGradient {
                                    angle: 0.0,
                                    stops: vec![
                                        (0.0, Color::hex("#cba6f7")),
                                        (1.0, Color::hex("#89b4fa")),
                                    ],
                                },
                                &theme,
                            ))
                            .child(gradient_card(
                                "Linear 90deg",
                                Fill::LinearGradient {
                                    angle: std::f32::consts::FRAC_PI_2,
                                    stops: vec![
                                        (0.0, Color::hex("#f38ba8")),
                                        (1.0, Color::hex("#f9e2af")),
                                    ],
                                },
                                &theme,
                            ))
                            .child(gradient_card(
                                "Linear 45deg",
                                Fill::LinearGradient {
                                    angle: std::f32::consts::FRAC_PI_4,
                                    stops: vec![
                                        (0.0, Color::hex("#a6e3a1")),
                                        (1.0, Color::hex("#89dceb")),
                                    ],
                                },
                                &theme,
                            ))
                            .child(gradient_card(
                                "Radial",
                                Fill::RadialGradient {
                                    center: Point::new(0.5, 0.5),
                                    radius: 1.0,
                                    stops: vec![
                                        (0.0, Color::hex("#f5c2e7")),
                                        (1.0, Color::hex("#313244")),
                                    ],
                                },
                                &theme,
                            )),
                    ),
            )
            // ── Tooltips ──
            .child(
                div()
                    .flex_col()
                    .gap(12.0)
                    .child(
                        label("Tooltips")
                            .color(heading_color)
                            .font_size(18.0)
                            .bold(),
                    )
                    .child(
                        div()
                            .flex_row()
                            .gap(16.0)
                            .child(
                                tooltip(&theme, "Copy to clipboard")
                                    .placement(Placement::Top)
                                    .child(button("Hover me (top)", &theme).secondary(&theme)),
                            )
                            .child(
                                tooltip(&theme, "Save document")
                                    .placement(Placement::Bottom)
                                    .shortcut("Cmd+S")
                                    .child(button("Hover me (bottom)", &theme).secondary(&theme)),
                            )
                            .child(
                                tooltip(&theme, "Settings")
                                    .placement(Placement::Right)
                                    .child(button("Hover me (right)", &theme).secondary(&theme)),
                            ),
                    ),
            )
            // ── Dialog trigger ──
            .child(
                div()
                    .flex_col()
                    .gap(12.0)
                    .child(label("Dialog").color(heading_color).font_size(18.0).bold())
                    .child(
                        div().flex_row().gap(12.0).child(
                            button("Open Dialog", &theme)
                                .primary(&theme)
                                .on_click({
                                    let flag = anim_flag.clone();
                                    move |cx| {
                                        let cx = cx.downcast_mut::<Context>().unwrap();
                                        cx.set(set_dialog_open, true);
                                        cx.set(set_dialog_anim_sig, Some(dialog_anim(flag.clone())));
                                    }
                                }),
                        ),
                    ),
            )
            // ── Notifications ──
            .child(
                div()
                    .flex_col()
                    .gap(12.0)
                    .child(
                        label("Notifications")
                            .color(heading_color)
                            .font_size(18.0)
                            .bold(),
                    )
                    .child(
                        div()
                            .flex_row()
                            .gap(8.0)
                            .flex_wrap()
                            .child(button("Default", &theme).secondary(&theme).on_click({
                                let flag = anim_flag.clone();
                                move |cx| {
                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                    let id = *cx.get(notif_counter);
                                    cx.set(set_notif_counter, id + 1);
                                    let anim = notification_anim(flag.clone());
                                    cx.update(set_notif_list, move |list| {
                                        list.push((id, NotificationType::Default, "Notification", "A plain notification with no icon.", anim));
                                    });
                                }
                            }))
                            .child(button("Info", &theme).secondary(&theme).on_click({
                                let flag = anim_flag.clone();
                                move |cx| {
                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                    let id = *cx.get(notif_counter);
                                    cx.set(set_notif_counter, id + 1);
                                    let anim = notification_anim(flag.clone());
                                    cx.update(set_notif_list, move |list| {
                                        list.push((id, NotificationType::Info, "Info", "This is an informational notification.", anim));
                                    });
                                }
                            }))
                            .child(button("Success", &theme).secondary(&theme).on_click({
                                let flag = anim_flag.clone();
                                move |cx| {
                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                    let id = *cx.get(notif_counter);
                                    cx.set(set_notif_counter, id + 1);
                                    let anim = notification_anim(flag.clone());
                                    cx.update(set_notif_list, move |list| {
                                        list.push((id, NotificationType::Success, "File saved", "Your changes have been saved successfully.", anim));
                                    });
                                }
                            }))
                            .child(button("Warning", &theme).secondary(&theme).on_click({
                                let flag = anim_flag.clone();
                                move |cx| {
                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                    let id = *cx.get(notif_counter);
                                    cx.set(set_notif_counter, id + 1);
                                    let anim = notification_anim(flag.clone());
                                    cx.update(set_notif_list, move |list| {
                                        list.push((id, NotificationType::Warning, "Warning", "Disk space is running low.", anim));
                                    });
                                }
                            }))
                            .child(button("Error", &theme).secondary(&theme).on_click({
                                let flag = anim_flag.clone();
                                move |cx| {
                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                    let id = *cx.get(notif_counter);
                                    cx.set(set_notif_counter, id + 1);
                                    let anim = notification_anim(flag.clone());
                                    cx.update(set_notif_list, move |list| {
                                        list.push((id, NotificationType::Error, "Connection failed", "Unable to reach the server. Please try again.", anim));
                                    });
                                }
                            }))
                            .child(button("Clear All", &theme).secondary(&theme).on_click({
                                let notifs = notifications.clone();
                                move |cx| {
                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                    // Trigger exit animation on all
                                    for (_, _, _, _, anim) in notifs.iter() {
                                        anim.set(0.0);
                                    }
                                    cx.set_timeout(Duration::from_millis(250), move |cx| {
                                        let cx = cx.downcast_mut::<Context>().unwrap();
                                        cx.set(set_notif_list, Vec::new());
                                    });
                                }
                            })),
                    ),
            )
            // ── Menu trigger ──
            .child(
                div()
                    .flex_col()
                    .gap(12.0)
                    .child(label("Menu").color(heading_color).font_size(18.0).bold())
                    .child(div().flex_row().gap(12.0).child(if is_menu_open {
                        div()
                            .flex_col()
                            .gap(8.0)
                            .child(button("Close Menu", &theme).secondary(&theme).on_click(
                                move |cx| {
                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                    cx.set(set_menu_open, false);
                                },
                            ))
                            .child(
                                menu(&theme)
                                    .item(
                                        menu_item("Cut").icon(IconName::Scissors).shortcut("Cmd+X"),
                                    )
                                    .item(menu_item("Copy").icon(IconName::Copy).shortcut("Cmd+C"))
                                    .item(
                                        menu_item("Paste")
                                            .icon(IconName::ClipboardText)
                                            .shortcut("Cmd+V"),
                                    )
                                    .item(menu_separator())
                                    .item(menu_item("Select All").shortcut("Cmd+A"))
                                    .item(menu_item("Disabled").disabled(true))
                                    .on_dismiss(move |cx| {
                                        let cx = cx.downcast_mut::<Context>().unwrap();
                                        cx.set(set_menu_open, false);
                                    }),
                            )
                    } else {
                        div().child(button("Show Menu", &theme).secondary(&theme).on_click(
                            move |cx| {
                                let cx = cx.downcast_mut::<Context>().unwrap();
                                cx.set(set_menu_open, true);
                            },
                        ))
                    })),
            );

    root = root.child(content);

    // Notification overlays — animation is baked into the component
    for (i, (id, ntype, title, desc, anim_handle)) in notifications.iter().enumerate() {
        let id = *id;
        let stack_offset = i as f32 * (56.0 + 12.0);
        let dismiss_anim = anim_handle.clone();
        root = root.child(
            notification(&theme, *ntype, *desc)
                .title(*title)
                .top_offset(stack_offset)
                .anim(anim_handle.clone())
                .on_dismiss(move |cx| {
                    let cx = cx.downcast_mut::<Context>().unwrap();
                    dismiss_anim.set(0.0); // trigger exit animation
                    cx.set_timeout(Duration::from_millis(250), move |cx| {
                        let cx = cx.downcast_mut::<Context>().unwrap();
                        cx.update(set_notif_list, move |list| {
                            list.retain(|(nid, _, _, _, _)| *nid != id);
                        });
                    });
                }),
        );
    }

    // Dialog overlay — animation handle persists in signal
    if is_dialog_open {
        let t = theme.clone();
        let dismiss_anim = dialog_anim_handle.clone();
        let mut dlg = dialog(&theme)
            .on_dismiss({
                let anim = dialog_anim_handle.clone();
                move |cx| {
                    let cx = cx.downcast_mut::<Context>().unwrap();
                    if let Some(ref a) = anim {
                        a.set(0.0); // trigger exit animation
                    }
                    cx.set_timeout(Duration::from_millis(200), move |cx| {
                        let cx = cx.downcast_mut::<Context>().unwrap();
                        cx.set(set_dialog_open, false);
                    });
                }
            })
            .child(
                div()
                    .flex_col()
                    .gap(16.0)
                    .p(24.0)
                    .child(
                        label("Confirm Action")
                            .color(t.foreground)
                            .font_size(18.0)
                            .bold(),
                    )
                    .child(
                        label(
                            "Are you sure you want to proceed? This action cannot be undone.",
                        )
                        .color(t.muted_foreground)
                        .font_size(14.0),
                    )
                    .child(
                        div()
                            .flex_row()
                            .gap(8.0)
                            .justify_end()
                            .child(button("Cancel", &t).secondary(&t).on_click({
                                let anim = dismiss_anim.clone();
                                move |cx| {
                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                    if let Some(ref a) = anim {
                                        a.set(0.0);
                                    }
                                    cx.set_timeout(Duration::from_millis(200), move |cx| {
                                        let cx = cx.downcast_mut::<Context>().unwrap();
                                        cx.set(set_dialog_open, false);
                                    });
                                }
                            }))
                            .child(button("Confirm", &t).primary(&t).on_click({
                                let anim = dismiss_anim.clone();
                                move |cx| {
                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                    if let Some(ref a) = anim {
                                        a.set(0.0);
                                    }
                                    cx.set_timeout(Duration::from_millis(200), move |cx| {
                                        let cx = cx.downcast_mut::<Context>().unwrap();
                                        cx.set(set_dialog_open, false);
                                    });
                                }
                            })),
                    ),
            );
        if let Some(anim) = dialog_anim_handle {
            dlg = dlg.anim(anim);
        }
        root = root.child(dlg);
    }

    Box::new(root)
}

fn shadow_card(title: &str, shadow: Shadow, theme: &Theme) -> Div {
    div()
        .w(160.0)
        .h(100.0)
        .flex_col()
        .items_center()
        .justify_center()
        .bg(theme.surface)
        .rounded(theme.radius_md)
        .shadow(shadow)
        .child(label(title).color(theme.foreground).font_size(14.0))
}

fn gradient_card(title: &str, fill: Fill, theme: &Theme) -> Div {
    div()
        .w(160.0)
        .h(100.0)
        .flex_col()
        .items_center()
        .justify_center()
        .bg(fill)
        .rounded(theme.radius_md)
        .child(label(title).color(Color::WHITE).font_size(13.0).bold())
}
