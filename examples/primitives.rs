use mozui::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Phase 1 Primitives".into(),
            size: Size::new(700.0, 600.0),
            titlebar: TitlebarStyle::Transparent,
            ..Default::default()
        })
        .run(app);
}

fn app(cx: &mut Context) -> Box<dyn Element> {
    let theme = cx.theme().clone();
    let scroll = cx.use_scroll();
    let heading_color = theme.foreground;
    let text_color = theme.text_secondary;
    let muted_color = theme.muted_foreground;
    let accent = theme.primary;
    let green = theme.success;
    let red = theme.danger;
    let yellow = theme.warning;
    let surface = theme.surface;

    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_col()
            .bg(theme.background)
            .on_key_down(move |key, _mods, _cx_any| {
                if key == Key::Escape {
                    std::process::exit(0);
                }
            })
            // Title bar drag region
            .child(
                div()
                    .w_full()
                    .h(38.0)
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .drag_region()
                    .child(
                        label("Phase 1: Core Primitives")
                            .font_size(13.0)
                            .color(muted_color),
                    ),
            )
            // Content area (scrollable)
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .flex_col()
                    .p(32.0)
                    .pt(8.0)
                    .gap(24.0)
                    .overflow_y_scroll(scroll)
                    // Section title
                    .child(
                        label("Phase 1: Core Primitives")
                            .font_size(28.0)
                            .bold()
                            .color(heading_color),
                    )
                    // ── Icons Section ──────────────────────────────────
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(label("Icons").font_size(18.0).bold().color(heading_color))
                            .child(
                                div()
                                    .flex_row()
                                    .gap(16.0)
                                    .items_center()
                                    .child(icon(IconName::House).color(text_color).small())
                                    .child(icon(IconName::Gear).color(text_color))
                                    .child(icon(IconName::MagnifyingGlass).color(accent))
                                    .child(icon(IconName::Star).color(yellow).large())
                                    .child(icon(IconName::Heart).color(red).size_px(24.0))
                                    .child(icon(IconName::Bell).color(green))
                                    .child(icon(IconName::Envelope).color(text_color))
                                    .child(icon(IconName::User).color(accent))
                                    .child(icon(IconName::Trash).color(red))
                                    .child(icon(IconName::Copy).color(text_color))
                                    .child(icon(IconName::PencilSimple).color(text_color))
                                    .child(icon(IconName::Check).color(green)),
                            )
                            .child(
                                label("12 icons at various sizes (small / medium / large / 24px)")
                                    .font_size(12.0)
                                    .color(muted_color),
                            ),
                    )
                    // ── Divider ────────────────────────────────────────
                    .child(divider().color(theme.border))
                    // ── Labels Section ─────────────────────────────────
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(label("Labels").font_size(18.0).bold().color(heading_color))
                            .child(label("Default label").color(text_color))
                            .child(label("Bold label").bold().color(text_color))
                            .child(label("Italic label").italic().color(text_color))
                            .child(label("Small label").small().color(muted_color))
                            .child(label("Large label").large().color(accent))
                            .child(label("Masked: password123").masked().color(text_color)),
                    )
                    // ── Labeled Divider ────────────────────────────────
                    .child(divider().label("Keyboard Shortcuts").color(theme.border))
                    // ── Kbd Section ────────────────────────────────────
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(label("Kbd").font_size(18.0).bold().color(heading_color))
                            .child(
                                div()
                                    .flex_row()
                                    .gap(8.0)
                                    .items_center()
                                    .child(label("Save").font_size(13.0).color(text_color))
                                    .child(kbd("cmd-s")),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .gap(8.0)
                                    .items_center()
                                    .child(label("Undo").font_size(13.0).color(text_color))
                                    .child(kbd("cmd-z")),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .gap(8.0)
                                    .items_center()
                                    .child(label("Find").font_size(13.0).color(text_color))
                                    .child(kbd("cmd-shift-f")),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .gap(8.0)
                                    .items_center()
                                    .child(label("Quit").font_size(13.0).color(text_color))
                                    .child(kbd("escape")),
                            ),
                    )
                    // ── Divider Variants ───────────────────────────────
                    .child(divider().color(theme.border))
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(
                                label("Divider Variants")
                                    .font_size(18.0)
                                    .bold()
                                    .color(heading_color),
                            )
                            .child(
                                div()
                                    .flex_row()
                                    .gap(16.0)
                                    .items_center()
                                    .h(24.0)
                                    .child(label("Left").font_size(13.0).color(text_color))
                                    .child(divider().vertical().color(theme.border))
                                    .child(label("Center").font_size(13.0).color(text_color))
                                    .child(divider().vertical().color(theme.border))
                                    .child(label("Right").font_size(13.0).color(text_color)),
                            ),
                    )
                    // ── Combined Example ───────────────────────────────
                    .child(divider().label("Combined").color(theme.border))
                    .child(
                        div()
                            .flex_row()
                            .gap(12.0)
                            .p(16.0)
                            .bg(surface)
                            .rounded(8.0)
                            .items_center()
                            .child(icon(IconName::Info).color(accent))
                            .child(
                                label("Press Esc to quit this demo")
                                    .font_size(13.0)
                                    .color(text_color),
                            )
                            .child(kbd("escape")),
                    ),
            ), // close content div
    )
}
