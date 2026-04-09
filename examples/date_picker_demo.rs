use mozui::time::{Date, Month};
use mozui::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Date Picker".into(),
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

    // Date picker state
    let (selected, set_selected) = cx.use_signal::<Option<Date>>(None);
    let (picker_open, set_picker_open) = cx.use_signal(false);
    let (view_year, set_view_year) = cx.use_signal(2026i32);
    let (view_month, set_view_month) = cx.use_signal(Month::April);

    let sel_val = *cx.get(selected);
    let is_open = *cx.get(picker_open);
    let vy = *cx.get(view_year);
    let vm = *cx.get(view_month);

    // Standalone calendar state
    let (cal_selected, set_cal_selected) = cx.use_signal::<Option<Date>>(None);
    let (cal_year, set_cal_year) = cx.use_signal(2026i32);
    let (cal_month, set_cal_month) = cx.use_signal(Month::April);
    let cal_sel = *cx.get(cal_selected);
    let cy = *cx.get(cal_year);
    let cm = *cx.get(cal_month);

    // Range calendar state
    let (range_start, set_range_start) = cx.use_signal::<Option<Date>>(None);
    let (range_end, set_range_end) = cx.use_signal::<Option<Date>>(None);
    let (range_year, set_range_year) = cx.use_signal(2026i32);
    let (range_month, set_range_month) = cx.use_signal(Month::April);
    let rs = *cx.get(range_start);
    let re = *cx.get(range_end);
    let ry = *cx.get(range_year);
    let rm = *cx.get(range_month);

    let selection = match sel_val {
        Some(d) => DateSelection::Single(Some(d)),
        None => DateSelection::Single(None),
    };

    let cal_selection = match cal_sel {
        Some(d) => DateSelection::Single(Some(d)),
        None => DateSelection::Single(None),
    };

    let range_selection = DateSelection::Range(rs, re);

    // Format selected date for display
    let selected_text = match sel_val {
        Some(d) => format!(
            "Selected: {:04}/{:02}/{:02}",
            d.year(),
            d.month() as u8,
            d.day()
        ),
        None => "No date selected".into(),
    };

    let cal_text = match cal_sel {
        Some(d) => format!(
            "Selected: {:04}/{:02}/{:02}",
            d.year(),
            d.month() as u8,
            d.day()
        ),
        None => "Click a date to select".into(),
    };

    let range_text = match (rs, re) {
        (Some(a), Some(b)) => format!(
            "Range: {:04}/{:02}/{:02} – {:04}/{:02}/{:02}",
            a.year(),
            a.month() as u8,
            a.day(),
            b.year(),
            b.month() as u8,
            b.day()
        ),
        (Some(a), None) => format!(
            "Start: {:04}/{:02}/{:02} (pick end date)",
            a.year(),
            a.month() as u8,
            a.day()
        ),
        _ => "Click to start a range".into(),
    };

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
                        label("Date Picker")
                            .font_size(13.0)
                            .color(muted),
                    ),
            )
            // Content
            .child(
                div()
                    .w_full()
                    .flex_grow(1.0)
                    .overflow_y_scroll(scroll)
                    .flex_col()
                    .gap(32.0)
                    .p(32.0)
                    // ── Section 1: DatePicker (input + dropdown) ──
                    .child(
                        div().flex_col().gap(12.0)
                            .child(label("DatePicker (Input + Dropdown)").font_size(18.0).color(heading).bold())
                            .child(label("Click the input to open the calendar dropdown.").font_size(13.0).color(muted))
                            .child(
                                div().flex_row().gap(16.0).items_center()
                                    .child(
                                        date_picker(&theme)
                                            .selection(selection)
                                            .view(vy, vm)
                                            .open(is_open)
                                            .on_toggle(move |cx: &mut dyn std::any::Any| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set_picker_open, !is_open);
                                            })
                                            .on_select(move |date, cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set_selected, Some(date));
                                                cx.set(set_picker_open, false);
                                            })
                                            .on_nav(move |year, month, cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set_view_year, year);
                                                cx.set(set_view_month, month);
                                            })
                                    )
                                    .child(label(&selected_text).font_size(13.0).color(muted))
                            )
                    )
                    // ── Section 2: Standalone Calendar ──
                    .child(
                        div().flex_col().gap(12.0)
                            .child(label("Standalone Calendar").font_size(18.0).color(heading).bold())
                            .child(label("A calendar grid used directly, without the dropdown trigger.").font_size(13.0).color(muted))
                            .child(
                                div().flex_row().gap(24.0).items_start()
                                    .child(
                                        calendar(&theme)
                                            .selection(cal_selection)
                                            .view(cy, cm)
                                            .on_select(move |date, cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set_cal_selected, Some(date));
                                            })
                                            .on_nav(move |year, month, cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set_cal_year, year);
                                                cx.set(set_cal_month, month);
                                            })
                                    )
                                    .child(
                                        div().flex_col().gap(8.0).pt(12.0)
                                            .child(label(&cal_text).font_size(13.0).color(muted))
                                    )
                            )
                    )
                    // ── Section 3: Range Selection ──
                    .child(
                        div().flex_col().gap(12.0)
                            .child(label("Range Selection").font_size(18.0).color(heading).bold())
                            .child(label("Click two dates to select a range. Days between are highlighted.").font_size(13.0).color(muted))
                            .child(
                                div().flex_row().gap(24.0).items_start()
                                    .child(
                                        calendar(&theme)
                                            .selection(range_selection)
                                            .view(ry, rm)
                                            .on_select(move |date, cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                // If no start or both set: reset to start
                                                if rs.is_none() || re.is_some() {
                                                    cx.set(set_range_start, Some(date));
                                                    cx.set(set_range_end, None);
                                                } else {
                                                    cx.set(set_range_end, Some(date));
                                                }
                                            })
                                            .on_nav(move |year, month, cx| {
                                                let cx = cx.downcast_mut::<Context>().unwrap();
                                                cx.set(set_range_year, year);
                                                cx.set(set_range_month, month);
                                            })
                                    )
                                    .child(
                                        div().flex_col().gap(8.0).pt(12.0)
                                            .child(label(&range_text).font_size(13.0).color(muted))
                                    )
                            )
                    )
                    // ── Section 4: Disabled Dates ──
                    .child(
                        div().flex_col().gap(12.0)
                            .child(label("Disabled Dates").font_size(18.0).color(heading).bold())
                            .child(label("Weekends are disabled. Past dates before April 5 are disabled.").font_size(13.0).color(muted))
                            .child(
                                calendar(&theme)
                                    .view(2026, Month::April)
                                    .disabled(DisabledMatcher::Weekdays(vec![5, 6])) // Sat, Sun
                                    .disabled(DisabledMatcher::Before(
                                        Date::from_calendar_date(2026, Month::April, 5).unwrap()
                                    ))
                                    .on_select(|_date, _cx| {
                                        // No-op for demo
                                    })
                                    .on_nav(|_y, _m, _cx| {
                                        // No-op — fixed view for demo
                                    })
                            )
                    )
            ),
    )
}
