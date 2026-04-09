use mozui::*;
use std::time::Duration;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Layout Demo".into(),
            size: Size::new(1100.0, 700.0),
            titlebar: TitlebarStyle::Transparent,
            ..Default::default()
        })
        .run(app);
}

fn app(cx: &mut Context) -> Box<dyn Element> {
    let theme = cx.theme().clone();

    // Sidebar state
    let (active_page, set_active_page) = cx.use_signal("dashboard".to_string());
    let current_page = cx.get(active_page).clone();

    // Animated sidebar width factor (0.0 = collapsed, 1.0 = expanded)
    let sidebar_anim = cx.use_animated(1.0f32, Transition::new(Duration::from_millis(200)));
    let sidebar_wf = sidebar_anim.get();

    // Resizable panel sizes
    let (panel_sizes, set_panel_sizes) = cx.use_signal::<Vec<f32>>(Vec::new());
    let saved_sizes = cx.get(panel_sizes).clone();

    // Sheet state
    let (sheet_open, set_sheet_open) = cx.use_signal(false);
    let is_sheet_open = *cx.get(sheet_open);
    let (sheet_anim_sig, set_sheet_anim_sig) = cx.use_signal::<Option<Animated<f32>>>(None);
    let sheet_anim_handle = cx.get(sheet_anim_sig).clone();
    let anim_flag = cx.animation_flag();

    // Build root — sheet must be a direct child of this full-size div
    let mut root = div()
        .w_full()
        .h_full()
        .flex_col()
        .bg(theme.background)
        .on_key_down({
            let sa = sidebar_anim.clone();
            let flag = anim_flag.clone();
            move |key, mods, cx| {
                if key == Key::Escape {
                    std::process::exit(0);
                }
                if key == Key::Character('b') && mods.meta {
                    if sa.get() > 0.5 {
                        sa.set(0.0);
                    } else {
                        sa.set(1.0);
                    }
                }
                if key == Key::Character('.') && mods.meta {
                    let cx = cx.downcast_mut::<Context>().unwrap();
                    if !is_sheet_open {
                        cx.set(set_sheet_open, true);
                        cx.set(set_sheet_anim_sig, Some(sheet_anim(flag.clone())));
                    } else {
                        // Animate close
                        if let Some(ref anim) = cx.get(sheet_anim_sig).clone() {
                            anim.set(0.0);
                        }
                        cx.set_timeout(Duration::from_millis(SHEET_ANIM_MS), move |cx| {
                            let cx = cx.downcast_mut::<Context>().unwrap();
                            cx.set(set_sheet_open, false);
                        });
                    }
                }
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
                    label("Layout Demo")
                        .font_size(13.0)
                        .color(theme.muted_foreground),
                ),
        )
        // Main content: sidebar + resizable panels
        .child(
            div()
                .w_full()
                .flex_grow(1.0)
                .flex_row()
                .child({
                    let sa = sidebar_anim.clone();
                    sidebar(&theme)
                        .side(SidebarSide::Left)
                        .width_factor(sidebar_wf)
                        .on_toggle(move |_cx| {
                            if sa.get() > 0.5 {
                                sa.set(0.0);
                            } else {
                                sa.set(1.0);
                            }
                        })
                        .group(
                            sidebar_group()
                                .label("Navigation")
                                .item(
                                    sidebar_item("dashboard", "Dashboard")
                                        .icon(IconName::House)
                                        .active(current_page == "dashboard")
                                        .on_click({
                                            let set = set_active_page;
                                            move |cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set, "dashboard".to_string());
                                            }
                                        }),
                                )
                                .item(
                                    sidebar_item("projects", "Projects")
                                        .icon(IconName::Folder)
                                        .active(current_page == "projects")
                                        .on_click({
                                            let set = set_active_page;
                                            move |cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set, "projects".to_string());
                                            }
                                        }),
                                )
                                .item(
                                    sidebar_item("tasks", "Tasks")
                                        .icon(IconName::ListChecks)
                                        .active(current_page == "tasks")
                                        .on_click({
                                            let set = set_active_page;
                                            move |cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set, "tasks".to_string());
                                            }
                                        }),
                                )
                                .item(
                                    sidebar_item("messages", "Messages")
                                        .icon(IconName::ChatCircle)
                                        .active(current_page == "messages")
                                        .on_click({
                                            let set = set_active_page;
                                            move |cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set, "messages".to_string());
                                            }
                                        }),
                                ),
                        )
                        .group(
                            sidebar_group()
                                .label("Settings")
                                .item(
                                    sidebar_item("settings", "Settings")
                                        .icon(IconName::Gear)
                                        .active(current_page == "settings")
                                        .on_click({
                                            let set = set_active_page;
                                            move |cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set, "settings".to_string());
                                            }
                                        }),
                                )
                                .item(
                                    sidebar_item("help", "Help")
                                        .icon(IconName::Question)
                                        .disabled(true),
                                ),
                        )
                })
                .child({
                    let mut group = h_resizable(&theme)
                        .panel(
                            resizable_panel().size(500.0).min_size(200.0).child(
                                div()
                                    .w_full()
                                    .h_full()
                                    .flex_col()
                                    .gap(16.0)
                                    .p(24.0)
                                    .child(
                                        label(&format!("Page: {}", current_page))
                                            .font_size(20.0)
                                            .bold()
                                            .color(theme.foreground),
                                    )
                                    .child(
                                        label("Drag the divider to resize panels.")
                                            .font_size(13.0)
                                            .color(theme.muted_foreground),
                                    )
                                    .child(
                                        label("Cmd+B toggles sidebar. Cmd+. opens sheet.")
                                            .font_size(13.0)
                                            .color(theme.muted_foreground),
                                    )
                                    .child(
                                        button("Open Sheet", &theme)
                                            .with_variant(ButtonVariant::Primary, &theme)
                                            .on_click({
                                                let flag = anim_flag.clone();
                                                move |cx| {
                                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                                    cx.set(set_sheet_open, true);
                                                    cx.set(
                                                        set_sheet_anim_sig,
                                                        Some(sheet_anim(flag.clone())),
                                                    );
                                                }
                                            }),
                                    ),
                            ),
                        )
                        .panel(
                            resizable_panel().min_size(150.0).child(
                                div()
                                    .w_full()
                                    .h_full()
                                    .flex_col()
                                    .gap(12.0)
                                    .p(24.0)
                                    .child(
                                        label("Details")
                                            .font_size(16.0)
                                            .bold()
                                            .color(theme.foreground),
                                    )
                                    .child(
                                        label("Secondary panel — resizable.")
                                            .font_size(13.0)
                                            .color(theme.muted_foreground),
                                    )
                                    .child(divider())
                                    .child(
                                        label("Status: Active")
                                            .font_size(13.0)
                                            .color(theme.success),
                                    )
                                    .child(
                                        label("Priority: High").font_size(13.0).color(theme.danger),
                                    )
                                    .child(
                                        label("Assigned: Zac")
                                            .font_size(13.0)
                                            .color(theme.foreground),
                                    ),
                            ),
                        )
                        .on_resize(move |sizes, cx| {
                            let cx = cx.downcast_mut::<Context>().unwrap();
                            cx.set(set_panel_sizes, sizes);
                        });
                    if !saved_sizes.is_empty() {
                        group = group.sizes(saved_sizes);
                    }
                    group
                }),
        );

    // Sheet overlay — direct child of root so absolute positioning works
    if is_sheet_open {
        let close_anim = sheet_anim_handle.clone();
        let mut s = sheet(SheetPlacement::Right, &theme)
            .size(400.0)
            .on_close(move |cx| {
                let cx = cx.downcast_mut::<Context>().unwrap();
                if let Some(ref anim) = close_anim {
                    anim.set(0.0);
                }
                cx.set_timeout(Duration::from_millis(SHEET_ANIM_MS), move |cx| {
                    let cx = cx.downcast_mut::<Context>().unwrap();
                    cx.set(set_sheet_open, false);
                });
            })
            .title(
                div().w_full().flex_row().items_center().p(16.0).child(
                    label("Sheet Panel")
                        .font_size(16.0)
                        .bold()
                        .color(theme.foreground),
                ),
            )
            .child(
                div()
                    .flex_col()
                    .gap(12.0)
                    .child(
                        label("Slide-in sheet from the right edge.")
                            .font_size(13.0)
                            .color(theme.muted_foreground),
                    )
                    .child(
                        label("Click overlay or Cmd+. to close.")
                            .font_size(13.0)
                            .color(theme.muted_foreground),
                    )
                    .child(divider())
                    .child(progress(&theme).value(0.65)),
            );
        if let Some(anim) = sheet_anim_handle {
            s = s.anim(anim);
        }
        root = root.child(s);
    }

    Box::new(root)
}
