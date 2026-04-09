use mozui::*;
use std::time::Duration;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Animations & Transitions".into(),
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

    // ── Animated values ──────────────────────────────────────────
    let transition = Transition::new(theme.transition_normal);
    let slow_transition = Transition::new(theme.transition_slow);

    // Color animation
    let (color_idx, set_color_idx) = cx.use_signal(0usize);
    let ci = *cx.get(color_idx);
    let color_anim = cx.use_animated(
        theme.accent,
        Transition::new(Duration::from_millis(500)),
    );
    let colors = [
        theme.accent,   // magenta
        theme.primary,  // cyan
        theme.success,  // green
        theme.warning,  // orange
        theme.danger,   // red
    ];
    color_anim.set(colors[ci % colors.len()]);

    // Opacity animation
    let (visible, set_visible) = cx.use_signal(true);
    let is_visible = *cx.get(visible);
    let opacity = cx.use_animated(1.0f32, transition.clone());
    opacity.set(if is_visible { 1.0 } else { 0.1 });

    // Width animation
    let (expanded, set_expanded) = cx.use_signal(false);
    let is_expanded = *cx.get(expanded);
    let bar_width = cx.use_animated(100.0f32, slow_transition.clone());
    bar_width.set(if is_expanded { 400.0 } else { 100.0 });

    // Spring animation
    let spring = cx.use_spring(1.0);

    // Accordion state
    let (acc_open, set_acc_open) = cx.use_signal([true, false, false]);
    let acc_state = *cx.get(acc_open);
    let acc_anim_0 = cx.use_animated(1.0f32, transition.clone());
    let acc_anim_1 = cx.use_animated(0.0f32, transition.clone());
    let acc_anim_2 = cx.use_animated(0.0f32, transition.clone());
    acc_anim_0.set(if acc_state[0] { 1.0 } else { 0.0 });
    acc_anim_1.set(if acc_state[1] { 1.0 } else { 0.0 });
    acc_anim_2.set(if acc_state[2] { 1.0 } else { 0.0 });

    // Collapsible state
    let (coll_open, set_coll_open) = cx.use_signal(false);
    let is_coll_open = *cx.get(coll_open);
    let coll_anim = cx.use_animated(0.0f32, transition.clone());
    coll_anim.set(if is_coll_open { 1.0 } else { 0.0 });

    // Read current values
    let current_color = color_anim.get();
    let current_opacity = opacity.get();
    let current_width = bar_width.get();
    let spring_val = spring.get();
    let coll_factor = coll_anim.get();

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
                        label("Animations & Transitions")
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
                    // ── Color Transition ───────────────────────────
                    .child(label("Color Transition").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_row()
                            .gap(12.0)
                            .items_center()
                            .child(
                                div()
                                    .w(80.0)
                                    .h(80.0)
                                    .rounded(12.0)
                                    .bg(current_color),
                            )
                            .child(
                                button("Next Color", &theme)
                                    .primary(&theme)
                                    .on_click(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.update(set_color_idx, |v| *v += 1);
                                    }),
                            ),
                    )
                    .child(divider().color(theme.border))
                    // ── Opacity ────────────────────────────────────
                    .child(label("Opacity Transition").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_row()
                            .gap(12.0)
                            .items_center()
                            .child(
                                div()
                                    .w(80.0)
                                    .h(80.0)
                                    .rounded(12.0)
                                    .bg(theme.primary.with_alpha(current_opacity)),
                            )
                            .child(
                                button(
                                    if is_visible { "Fade Out" } else { "Fade In" },
                                    &theme,
                                )
                                .on_click(move |cx_any| {
                                    let cx = cx_any.downcast_mut::<Context>().unwrap();
                                    cx.update(set_visible, |v| *v = !*v);
                                }),
                            )
                            .child(
                                label(format!("opacity: {:.2}", current_opacity))
                                    .font_size(13.0)
                                    .color(muted),
                            ),
                    )
                    .child(divider().color(theme.border))
                    // ── Width / Size ───────────────────────────────
                    .child(label("Size Transition").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(12.0)
                            .child(
                                div()
                                    .w(current_width)
                                    .h(40.0)
                                    .rounded(8.0)
                                    .bg(theme.success),
                            )
                            .child(
                                button(
                                    if is_expanded { "Shrink" } else { "Expand" },
                                    &theme,
                                )
                                .on_click(move |cx_any| {
                                    let cx = cx_any.downcast_mut::<Context>().unwrap();
                                    cx.update(set_expanded, |v| *v = !*v);
                                }),
                            ),
                    )
                    .child(divider().color(theme.border))
                    // ── Spring ─────────────────────────────────────
                    .child(label("Spring Animation").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_row()
                            .gap(12.0)
                            .items_center()
                            .child(
                                div()
                                    .w(60.0 * spring_val)
                                    .h(60.0 * spring_val)
                                    .rounded(30.0 * spring_val)
                                    .bg(theme.warning),
                            )
                            .child({
                                let s = spring.clone();
                                button("Bounce", &theme)
                                    .on_click(move |_cx_any| {
                                        s.set(1.5);
                                    })
                            })
                            .child({
                                let s = spring.clone();
                                button("Shrink", &theme)
                                    .on_click(move |_cx_any| {
                                        s.set(0.5);
                                    })
                            })
                            .child({
                                let s = spring.clone();
                                button("Reset", &theme)
                                    .on_click(move |_cx_any| {
                                        s.set(1.0);
                                    })
                            })
                            .child(
                                label(format!("spring: {:.2}", spring_val))
                                    .font_size(13.0)
                                    .color(muted),
                            ),
                    )
                    .child(divider().color(theme.border))
                    // ── Collapsible ────────────────────────────────
                    .child(label("Collapsible").font_size(20.0).bold().color(heading))
                    .child(
                        div()
                            .flex_col()
                            .gap(8.0)
                            .child(
                                button(
                                    if is_coll_open { "Hide Content" } else { "Show Content" },
                                    &theme,
                                )
                                .outline(&theme)
                                .icon(if is_coll_open {
                                    IconName::CaretUp
                                } else {
                                    IconName::CaretDown
                                })
                                .on_click(move |cx_any| {
                                    let cx = cx_any.downcast_mut::<Context>().unwrap();
                                    cx.update(set_coll_open, |v| *v = !*v);
                                }),
                            )
                            .child(
                                collapsible(coll_factor).child(
                                    div()
                                        .flex_col()
                                        .gap(8.0)
                                        .p(16.0)
                                        .bg(theme.surface)
                                        .rounded(8.0)
                                        .child(
                                            label("This content expands and collapses smoothly!")
                                                .color(theme.foreground),
                                        )
                                        .child(
                                            label("It uses AnimatedValue with eased transitions.")
                                                .color(theme.muted_foreground)
                                                .font_size(13.0),
                                        )
                                        .child(
                                            label("The height is interpolated from 0 to full.")
                                                .color(theme.muted_foreground)
                                                .font_size(13.0),
                                        ),
                                ),
                            ),
                    )
                    .child(divider().color(theme.border))
                    // ── Accordion ──────────────────────────────────
                    .child(label("Accordion").font_size(20.0).bold().color(heading))
                    .child(
                        accordion(&theme)
                            .child(
                                accordion_item("What is mozui?")
                                    .open(acc_state[0])
                                    .height_factor(acc_anim_0.get())
                                    .on_toggle(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.update(set_acc_open, |v| v[0] = !v[0]);
                                    })
                                    .child(
                                        label("mozui is a GPU-accelerated cross-platform UI framework for Rust, inspired by GPUI.")
                                            .color(theme.muted_foreground),
                                    ),
                            )
                            .child(
                                accordion_item("How do animations work?")
                                    .icon(IconName::Lightning)
                                    .open(acc_state[1])
                                    .height_factor(acc_anim_1.get())
                                    .on_toggle(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.update(set_acc_open, |v| v[1] = !v[1]);
                                    })
                                    .child(
                                        label("Animations use eased transitions with configurable duration. Springs provide physically-based motion.")
                                            .color(theme.muted_foreground),
                                    ),
                            )
                            .child(
                                accordion_item("What about springs?")
                                    .icon(IconName::Atom)
                                    .open(acc_state[2])
                                    .height_factor(acc_anim_2.get())
                                    .on_toggle(move |cx_any| {
                                        let cx = cx_any.downcast_mut::<Context>().unwrap();
                                        cx.update(set_acc_open, |v| v[2] = !v[2]);
                                    })
                                    .child(
                                        label("Springs use a damped harmonic oscillator model with configurable stiffness and damping for natural-feeling motion.")
                                            .color(theme.muted_foreground),
                                    ),
                            ),
                    ),
            ),
    )
}
