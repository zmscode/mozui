use std::collections::HashSet;

use mozui_devtools::SignalLog;
use mozui_elements::{Element, div, label};
use mozui_style::Color;

const PANEL_WIDTH: f32 = 320.0;
const MAX_LOG_ENTRIES: usize = 20;

/// Shorten a fully-qualified type name, including paths inside generics.
/// e.g. "alloc::string::String" -> "String"
///      "core::option::Option<mozui_style::animation::Animated<f32>>" -> "Option<Animated<f32>>"
fn short_type_name(name: &str) -> String {
    let mut result = String::new();
    let mut i = 0;
    let bytes = name.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' || bytes[i] == b'>' || bytes[i] == b',' || bytes[i] == b' ' {
            result.push(bytes[i] as char);
            i += 1;
        } else {
            // Read a path segment until we hit <, >, comma, space, or end
            let start = i;
            while i < bytes.len()
                && bytes[i] != b'<'
                && bytes[i] != b'>'
                && bytes[i] != b','
                && bytes[i] != b' '
            {
                i += 1;
            }
            let segment = &name[start..i];
            // Take just the last :: component
            match segment.rfind("::") {
                Some(pos) => result.push_str(&segment[pos + 2..]),
                None => result.push_str(segment),
            }
        }
    }
    result
}

/// Build the signal debugger panel overlay.
pub fn signal_panel(
    signal_log: &SignalLog,
    signal_summary: &[(usize, &'static str)],
) -> Box<dyn Element> {
    let bg = Color::new(0.1, 0.1, 0.1, 0.95);
    let dim = Color::new(1.0, 1.0, 1.0, 0.5);
    let bright = Color::new(1.0, 1.0, 1.0, 0.9);
    let header_color = Color::new(1.0, 1.0, 1.0, 1.0);
    let mutated_bg = Color::rgba(240, 200, 60, 0.3);

    let current_frame = signal_log.current_frame;

    // Collect slot IDs that mutated this frame for highlight lookup.
    let mutated_this_frame: HashSet<usize> = signal_log
        .mutations_this_frame(current_frame)
        .map(|m| m.slot_id)
        .collect();

    // --- Title ---
    let title = div()
        .flex_row()
        .items_center()
        .child(label("Signal Debugger").font_size(14.0).bold().color(header_color));

    // --- Signal table section ---
    let table_header = div()
        .flex_row()
        .justify_between()
        .child(label("Signals").font_size(13.0).bold().color(bright))
        .child(label(format!("{} total", signal_summary.len())).font_size(11.0).color(dim));

    let col_header = div()
        .flex_row()
        .gap(8.0)
        .child(div().w(36.0).child(label("Slot").font_size(10.0).color(dim)))
        .child(label("Type").font_size(10.0).color(dim));

    let mut signal_table = div().flex_col().gap(1.0);
    for &(slot_id, type_name) in signal_summary {
        let mutated = mutated_this_frame.contains(&slot_id);
        let short = short_type_name(type_name);

        let mut row = div()
            .flex_row()
            .gap(8.0)
            .px(4.0)
            .py(1.0)
            .rounded(3.0)
            .child(div().w(36.0).child(label(format!("{}", slot_id)).font_size(11.0).color(dim)))
            .child(label(short).font_size(11.0).color(bright));

        if mutated {
            row = row.bg(mutated_bg);
        }

        signal_table = signal_table.child(row);
    }

    // --- Mutation log section ---
    let log_header = div()
        .flex_row()
        .justify_between()
        .child(label("Mutation Log").font_size(13.0).bold().color(bright))
        .child(label(format!("{} recorded", signal_log.len())).font_size(11.0).color(dim));

    let mut log_list = div().flex_col().gap(1.0);

    // Collect mutations, reverse for newest-first, take last N.
    let mutations: Vec<_> = signal_log.iter().collect();
    let recent = mutations.iter().rev().take(MAX_LOG_ENTRIES);

    for mutation in recent {
        let age = current_frame.saturating_sub(mutation.frame);
        let alpha = if age == 0 {
            0.95
        } else if age < 5 {
            0.75
        } else if age < 15 {
            0.55
        } else {
            0.35
        };
        let entry_color = Color::new(1.0, 1.0, 1.0, alpha);
        let short = short_type_name(mutation.type_name);

        let mut row = div()
            .flex_row()
            .px(4.0)
            .py(1.0)
            .rounded(3.0)
            .child(
                label(format!("#{} slot[{}] {}", mutation.frame, mutation.slot_id, short))
                    .font_size(11.0)
                    .color(entry_color),
            );

        if age == 0 {
            row = row.bg(mutated_bg);
        }

        log_list = log_list.child(row);
    }

    // --- Separator line helper ---
    let separator = || {
        div()
            .h(1.0)
            .w_full()
            .bg(Color::new(1.0, 1.0, 1.0, 0.12))
    };

    // --- Assemble panel ---
    let panel = div()
        .w(PANEL_WIDTH)
        .h_full()
        .flex_col()
        .gap(8.0)
        .p(12.0)
        .bg(bg)
        .child(title)
        .child(separator())
        .child(table_header)
        .child(col_header)
        .child(signal_table)
        .child(separator())
        .child(log_header)
        .child(log_list);

    // Full-window overlay wrapper — flex to left side.
    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_row()
            .items_start()
            .child(panel),
    )
}
