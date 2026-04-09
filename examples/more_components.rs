use mozui::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "More Components".into(),
            size: Size::new(900.0, 850.0),
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

    // Tree view state
    let (tree_expanded, set_tree_expanded) = cx.use_signal([true, false, true]);
    let (tree_selected, set_tree_selected) = cx.use_signal(String::new());
    let expanded = *cx.get(tree_expanded);
    let selected_file = cx.get(tree_selected).clone();

    // Number input state
    let (num_val, set_num_val) = cx.use_signal(5.0f64);
    let (float_val, set_float_val) = cx.use_signal(0.5f64);
    let nv = *cx.get(num_val);
    let fv = *cx.get(float_val);

    // Toggle group state
    let (view_mode, set_view_mode) = cx.use_signal("grid".to_string());
    let vm = cx.get(view_mode).clone();

    // Alert dismiss state
    let (show_info, set_show_info) = cx.use_signal(true);
    let (show_warning, set_show_warning) = cx.use_signal(true);
    let info_visible = *cx.get(show_info);
    let warning_visible = *cx.get(show_warning);

    // Command palette visibility + animation
    let (palette_open, set_palette_open) = cx.use_signal(false);
    let palette_visible = *cx.get(palette_open);
    let (palette_anim_sig, set_palette_anim_sig) =
        cx.use_signal::<Option<Animated<f32>>>(None);
    let palette_anim_handle = cx.get(palette_anim_sig).clone();
    let anim_flag = cx.animation_flag();

    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_col()
            .bg(theme.background)
            .on_key_down(move |key, mods, cx| {
                if key == Key::Escape {
                    std::process::exit(0);
                }
                if key == Key::Character('k') && mods.meta {
                    let cx = cx.downcast_mut::<Context>().unwrap();
                    let new_state = !palette_visible;
                    cx.set(set_palette_open, new_state);
                    if new_state {
                        cx.set(set_palette_anim_sig, Some(command_palette_anim(anim_flag.clone())));
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
                    .child(label("More Components").font_size(13.0).color(muted)),
            )
            // Content
            .child(
                div()
                    .w_full()
                    .flex_grow(1.0)
                    .overflow_y_scroll(scroll)
                    .flex_col()
                    .gap(36.0)
                    .p(32.0)

                    // ── Tree View ──────────────────────────────────────
                    .child(
                        div().flex_col().gap(12.0)
                            .child(label("Tree View").font_size(20.0).bold().color(heading))
                            .child(label("Hierarchical data with expand/collapse and icons.").font_size(13.0).color(muted))
                            .child(
                                div().flex_row().gap(24.0).items_start()
                                    .child(
                                        tree_view(&theme)
                                            .root(
                                                tree_node("src")
                                                    .icon(IconName::FolderOpen)
                                                    .expanded(expanded[0])
                                                    .on_toggle({
                                                        move |cx| {
                                                            let cx = cx.downcast_mut::<Context>().unwrap();
                                                            let mut e = expanded;
                                                            e[0] = !e[0];
                                                            cx.set(set_tree_expanded, e);
                                                        }
                                                    })
                                                    .child(
                                                        tree_node("components")
                                                            .icon(IconName::Folder)
                                                            .expanded(expanded[1])
                                                            .on_toggle({
                                                                move |cx| {
                                                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                                                    let mut e = expanded;
                                                                    e[1] = !e[1];
                                                                    cx.set(set_tree_expanded, e);
                                                                }
                                                            })
                                                            .child(
                                                                tree_node("Button.tsx")
                                                                    .icon(IconName::FileCode)
                                                                    .selected(selected_file == "Button.tsx")
                                                                    .on_click({
                                                                        let set = set_tree_selected;
                                                                        move |cx| {
                                                                            let cx = cx.downcast_mut::<Context>().unwrap();
                                                                            cx.set(set, "Button.tsx".to_string());
                                                                        }
                                                                    })
                                                            )
                                                            .child(
                                                                tree_node("Card.tsx")
                                                                    .icon(IconName::FileCode)
                                                                    .selected(selected_file == "Card.tsx")
                                                                    .on_click({
                                                                        let set = set_tree_selected;
                                                                        move |cx| {
                                                                            let cx = cx.downcast_mut::<Context>().unwrap();
                                                                            cx.set(set, "Card.tsx".to_string());
                                                                        }
                                                                    })
                                                            )
                                                    )
                                                    .child(
                                                        tree_node("utils")
                                                            .icon(IconName::Folder)
                                                            .expanded(expanded[2])
                                                            .on_toggle({
                                                                move |cx| {
                                                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                                                    let mut e = expanded;
                                                                    e[2] = !e[2];
                                                                    cx.set(set_tree_expanded, e);
                                                                }
                                                            })
                                                            .child(
                                                                tree_node("helpers.ts")
                                                                    .icon(IconName::File)
                                                                    .selected(selected_file == "helpers.ts")
                                                                    .on_click({
                                                                        let set = set_tree_selected;
                                                                        move |cx| {
                                                                            let cx = cx.downcast_mut::<Context>().unwrap();
                                                                            cx.set(set, "helpers.ts".to_string());
                                                                        }
                                                                    })
                                                            )
                                                    )
                                                    .child(
                                                        tree_node("main.ts")
                                                            .icon(IconName::File)
                                                            .selected(selected_file == "main.ts")
                                                            .on_click({
                                                                let set = set_tree_selected;
                                                                move |cx| {
                                                                    let cx = cx.downcast_mut::<Context>().unwrap();
                                                                    cx.set(set, "main.ts".to_string());
                                                                }
                                                            })
                                                    )
                                            )
                                    )
                                    .child(
                                        div().flex_col().gap(4.0).pt(4.0)
                                            .child(label(&format!("Selected: {}", if selected_file.is_empty() { "none" } else { &selected_file })).font_size(13.0).color(muted))
                                    )
                            )
                    )

                    // ── Number Input ──────────────────────────────────
                    .child(
                        div().flex_col().gap(12.0)
                            .child(label("Number Input").font_size(20.0).bold().color(heading))
                            .child(label("Numeric input with increment/decrement buttons.").font_size(13.0).color(muted))
                            .child(
                                div().flex_row().gap(24.0).items_center()
                                    .child(
                                        div().flex_col().gap(8.0)
                                            .child(label("Integer (1-20)").font_size(12.0).color(muted))
                                            .child(
                                                number_input(&theme)
                                                    .value(nv)
                                                    .min(1.0)
                                                    .max(20.0)
                                                    .step(1.0)
                                                    .on_change(move |val, cx| {
                                                        let cx = cx.downcast_mut::<Context>().unwrap();
                                                        cx.set(set_num_val, val);
                                                    })
                                            )
                                    )
                                    .child(
                                        div().flex_col().gap(8.0)
                                            .child(label("Float (0.0-1.0, step 0.1)").font_size(12.0).color(muted))
                                            .child(
                                                number_input(&theme)
                                                    .value(fv)
                                                    .min(0.0)
                                                    .max(1.0)
                                                    .step(0.1)
                                                    .precision(1)
                                                    .on_change(move |val, cx| {
                                                        let cx = cx.downcast_mut::<Context>().unwrap();
                                                        cx.set(set_float_val, val);
                                                    })
                                            )
                                    )
                            )
                    )

                    // ── Toggle Group ──────────────────────────────────
                    .child(
                        div().flex_col().gap(12.0)
                            .child(label("Toggle Group").font_size(20.0).bold().color(heading))
                            .child(label("Mutually-exclusive button selection (segmented control).").font_size(13.0).color(muted))
                            .child(
                                div().flex_row().gap(16.0).items_center()
                                    .child(
                                        toggle_group(&theme)
                                            .items(vec![
                                                toggle_item("list", "List").icon(IconName::List),
                                                toggle_item("grid", "Grid").icon(IconName::GridFour),
                                                toggle_item("board", "Board").icon(IconName::Kanban),
                                            ])
                                            .selected(&vm)
                                            .on_change(move |value, cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set_view_mode, value.to_string());
                                            })
                                    )
                                    .child(label(&format!("View: {}", vm)).font_size(13.0).color(muted))
                            )
                    )

                    // ── Card ──────────────────────────────────────────
                    .child(
                        div().flex_col().gap(12.0)
                            .child(label("Card").font_size(20.0).bold().color(heading))
                            .child(label("Container with header, body, and footer sections.").font_size(13.0).color(muted))
                            .child(
                                div().flex_row().gap(16.0).items_start()
                                    .child(
                                        card(&theme)
                                            .title("Project Settings")
                                            .description("Configure your project preferences.")
                                            .width(300.0)
                                            .child(
                                                label("Here you can change the project name, description, and other metadata.")
                                                    .font_size(13.0)
                                                    .color(muted)
                                            )
                                            .footer(
                                                button("Cancel", &theme)
                                            )
                                            .footer(
                                                button("Save", &theme)
                                                    .with_variant(ButtonVariant::Primary, &theme)
                                            )
                                    )
                                    .child(
                                        card(&theme)
                                            .title("Statistics")
                                            .width(240.0)
                                            .child(
                                                div().flex_col().gap(8.0)
                                                    .child(label("Total users: 1,234").font_size(13.0).color(heading))
                                                    .child(label("Active today: 89").font_size(13.0).color(heading))
                                                    .child(label("Revenue: $12,450").font_size(13.0).color(heading))
                                            )
                                    )
                            )
                    )

                    // ── Alert / Banner ────────────────────────────────
                    .child(
                        div().flex_col().gap(12.0)
                            .child(label("Alert / Banner").font_size(20.0).bold().color(heading))
                            .child(label("Semantic status messages with optional dismiss.").font_size(13.0).color(muted))
                            .child({
                                let mut col = div().flex_col().gap(8.0);
                                if info_visible {
                                    col = col.child(
                                        alert(AlertVariant::Info, "New version available", &theme)
                                            .description("Version 2.0 has been released with many improvements.")
                                            .dismissible(move |cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set_show_info, false);
                                            })
                                    );
                                }
                                col = col.child(
                                    alert(AlertVariant::Success, "Deployment complete", &theme)
                                        .description("All services are running normally.")
                                );
                                if warning_visible {
                                    col = col.child(
                                        alert(AlertVariant::Warning, "Rate limit approaching", &theme)
                                            .description("You've used 85% of your API quota.")
                                            .dismissible(move |cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set_show_warning, false);
                                            })
                                    );
                                }
                                col = col.child(
                                    alert(AlertVariant::Danger, "Build failed", &theme)
                                        .description("3 tests failed in the CI pipeline.")
                                );
                                col
                            })
                    )

                    // ── Command Palette ────────────────────────────────
                    .child({
                        let mut section = div().flex_col().gap(12.0)
                            .child(label("Command Palette").font_size(20.0).bold().color(heading))
                            .child(label("Press Cmd+K to toggle. Searchable action list.").font_size(13.0).color(muted));
                        if palette_visible {
                            let mut cp = command_palette(&theme)
                                    .items(vec![
                                        command_item("new_file", "New File").icon(IconName::FilePlus).shortcut("Cmd+N"),
                                        command_item("open_file", "Open File").icon(IconName::FolderOpen).shortcut("Cmd+O"),
                                        command_item("save", "Save").icon(IconName::FloppyDisk).shortcut("Cmd+S"),
                                        command_item("find", "Find in Files").icon(IconName::MagnifyingGlass).shortcut("Cmd+Shift+F"),
                                        command_item("terminal", "Toggle Terminal").icon(IconName::Terminal).shortcut("Cmd+`"),
                                        command_item("settings", "Open Settings").icon(IconName::Gear).shortcut("Cmd+,"),
                                        command_item("palette", "Command Palette").icon(IconName::Command).shortcut("Cmd+Shift+P"),
                                    ])
                                    .selected_index(0);
                            if let Some(anim) = palette_anim_handle {
                                cp = cp.anim(anim);
                            }
                            section = section.child(cp);
                        }
                        section
                    })
            ),
    )
}
