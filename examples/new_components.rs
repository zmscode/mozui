use mozui::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "New Components: Skeleton, Avatar, Color Picker".into(),
            size: Size::new(850.0, 750.0),
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

    // Color picker state
    let (color_sig, set_color_sig) = cx.use_signal(Color::hex("#3b82f6"));
    let current_color = *cx.get(color_sig);

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
                        label("New Components: Skeleton, Avatar, Color Picker")
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
                    // ── Skeleton ──────────────────────────────────
                    .child(label("Skeleton / Loading").font_size(20.0).bold().color(heading))
                    .child(label("Placeholder components for async content loading.").font_size(13.0).color(muted))
                    .child(
                        div()
                            .flex_col()
                            .gap(16.0)
                            // Card skeleton
                            .child(
                                div()
                                    .flex_row()
                                    .gap(16.0)
                                    .items_center()
                                    .child(Skeleton::avatar(&theme, 48.0))
                                    .child(
                                        div()
                                            .flex_col()
                                            .gap(8.0)
                                            .child(skeleton(&theme).w(160.0).h(16.0))
                                            .child(skeleton(&theme).w(100.0).h(12.0)),
                                    ),
                            )
                            // Text lines
                            .child(
                                div()
                                    .flex_col()
                                    .gap(6.0)
                                    .child(Skeleton::text_line(&theme).w(300.0))
                                    .child(Skeleton::text_line(&theme).w(250.0))
                                    .child(Skeleton::text_line(&theme).w(280.0)),
                            )
                            // Shapes
                            .child(
                                div()
                                    .flex_row()
                                    .gap(12.0)
                                    .items_center()
                                    .child(skeleton(&theme).w(100.0).h(32.0))
                                    .child(skeleton(&theme).w(100.0).h(32.0).pill())
                                    .child(skeleton(&theme).h(40.0).circle())
                                    .child(Skeleton::button(&theme)),
                            )
                            // Large content skeleton
                            .child(skeleton(&theme).w(400.0).h(120.0).radius(8.0)),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Avatar ────────────────────────────────────
                    .child(label("Avatar").font_size(20.0).bold().color(heading))
                    .child(label("User avatar with image, initials fallback, and status indicator.").font_size(13.0).color(muted))
                    .child(
                        div()
                            .flex_col()
                            .gap(16.0)
                            // Default icon fallback
                            .child(
                                div()
                                    .flex_row()
                                    .gap(16.0)
                                    .items_center()
                                    .child(label("Default:").font_size(13.0).color(theme.foreground))
                                    .child(avatar(&theme).size(32.0))
                                    .child(avatar(&theme).size(40.0))
                                    .child(avatar(&theme).size(48.0))
                                    .child(avatar(&theme).size(56.0)),
                            )
                            // Initials
                            .child(
                                div()
                                    .flex_row()
                                    .gap(16.0)
                                    .items_center()
                                    .child(label("Initials:").font_size(13.0).color(theme.foreground))
                                    .child(
                                        avatar(&theme)
                                            .initials("JD")
                                            .bg(theme.primary)
                                            .fg(theme.primary_foreground),
                                    )
                                    .child(
                                        avatar(&theme)
                                            .initials("AB")
                                            .bg(theme.success)
                                            .fg(theme.success_foreground),
                                    )
                                    .child(
                                        avatar(&theme)
                                            .initials("ZM")
                                            .bg(theme.danger)
                                            .fg(theme.danger_foreground),
                                    )
                                    .child(
                                        avatar(&theme)
                                            .initials("K")
                                            .bg(theme.warning)
                                            .fg(theme.warning_foreground),
                                    ),
                            )
                            // Status indicators
                            .child(
                                div()
                                    .flex_row()
                                    .gap(16.0)
                                    .items_center()
                                    .child(label("Status:").font_size(13.0).color(theme.foreground))
                                    .child(
                                        avatar(&theme)
                                            .initials("ON")
                                            .bg(theme.primary)
                                            .fg(theme.primary_foreground)
                                            .status(AvatarStatus::Online),
                                    )
                                    .child(
                                        avatar(&theme)
                                            .initials("AW")
                                            .bg(theme.success)
                                            .fg(theme.success_foreground)
                                            .status(AvatarStatus::Away),
                                    )
                                    .child(
                                        avatar(&theme)
                                            .initials("BS")
                                            .bg(theme.warning)
                                            .fg(theme.warning_foreground)
                                            .status(AvatarStatus::Busy),
                                    )
                                    .child(
                                        avatar(&theme)
                                            .initials("OF")
                                            .bg(theme.muted)
                                            .fg(theme.muted_foreground)
                                            .status(AvatarStatus::Offline),
                                    ),
                            )
                            // Sizes
                            .child(
                                div()
                                    .flex_row()
                                    .gap(16.0)
                                    .items_center()
                                    .child(label("Sizes:").font_size(13.0).color(theme.foreground))
                                    .child(avatar(&theme).size(24.0).initials("XS"))
                                    .child(avatar(&theme).size(32.0).initials("SM"))
                                    .child(avatar(&theme).size(40.0).initials("MD"))
                                    .child(avatar(&theme).size(56.0).initials("LG"))
                                    .child(avatar(&theme).size(72.0).initials("XL")),
                            ),
                    )
                    .child(divider().color(Color::hex("#45475a")))
                    // ── Color Picker ─────────────────────────────
                    .child(label("Color Picker").font_size(20.0).bold().color(heading))
                    .child(label("Interactive HSV color picker with hue, saturation, brightness, and alpha.").font_size(13.0).color(muted))
                    .child(
                        div()
                            .flex_row()
                            .gap(24.0)
                            .items_start()
                            .child(
                                color_picker(&theme)
                                    .color(current_color)
                                    .on_change(move |new_color, cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.set(set_color_sig, new_color);
                                    }),
                            )
                            .child(
                                div()
                                    .flex_col()
                                    .gap(12.0)
                                    .child(label("Selected Color:").font_size(13.0).color(theme.foreground))
                                    .child(
                                        div()
                                            .w(120.0)
                                            .h(60.0)
                                            .rounded(8.0)
                                            .bg(current_color),
                                    )
                                    .child(
                                        label(format!(
                                            "R: {:.0}  G: {:.0}  B: {:.0}  A: {:.2}",
                                            current_color.r * 255.0,
                                            current_color.g * 255.0,
                                            current_color.b * 255.0,
                                            current_color.a
                                        ))
                                        .font_size(12.0)
                                        .color(muted),
                                    ),
                            ),
                    ),
            ),
    )
}
