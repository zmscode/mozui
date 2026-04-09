use mozui_devtools::InspectorState;
use mozui_elements::{div, divider, label, Element};
use mozui_style::Color;

/// Build the element inspector overlay (highlight + detail panel).
pub fn inspector_overlay(inspector: &InspectorState) -> Box<dyn Element> {
    let highlight_color_bg = Color::rgba(66, 135, 245, 0.25);
    let highlight_color_border = Color::rgba(66, 135, 245, 0.8);
    let panel_bg = Color::new(0.0, 0.0, 0.0, 0.85);
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    let dim = Color::new(0.6, 0.6, 0.6, 1.0);
    let label_bg = Color::new(0.1, 0.1, 0.1, 0.9);

    // Determine which entry to show in the detail panel (selected takes priority).
    let active_idx = inspector.selected.or(inspector.hovered);
    let active_entry = active_idx.and_then(|i| inspector.tree.get(i));

    // --- Highlight area (left, flex_grow) ---
    let highlight_area = if let Some(entry) = inspector.hovered.and_then(|i| inspector.tree.get(i))
    {
        let tooltip_text = format!("{} {:.0}\u{00d7}{:.0}", entry.type_name, entry.width, entry.height);

        let tooltip = div()
            .bg(label_bg)
            .rounded(4.0)
            .px(6.0)
            .py(2.0)
            .child(label(&tooltip_text).font_size(11.0).color(white));

        let highlight_box = div()
            .w(entry.width)
            .h(entry.height)
            .bg(highlight_color_bg)
            .border(1.0, highlight_color_border)
            .rounded(2.0);

        // Position the highlight using padding on a wrapper to offset from top-left.
        div()
            .flex_col()
            .flex_grow(1.0)
            .h_full()
            .pt(entry.y.max(0.0))
            .pl(entry.x.max(0.0))
            .child(tooltip)
            .child(highlight_box)
    } else {
        div().flex_grow(1.0).h_full()
    };

    // --- Detail panel (right side, 280px) ---
    let detail_panel = if let Some(entry) = active_entry {
        let mut panel = div()
            .flex_col()
            .w(280.0)
            .h_full()
            .bg(panel_bg)
            .p(12.0)
            .gap(8.0);

        // Type name heading
        panel = panel.child(label(entry.type_name).font_size(14.0).color(white).bold());

        // Layout section
        panel = panel.child(label("Layout").font_size(12.0).color(dim));

        let layout_items = [
            format!("x: {:.1}", entry.x),
            format!("y: {:.1}", entry.y),
            format!("width: {:.1}", entry.width),
            format!("height: {:.1}", entry.height),
        ];
        for item in &layout_items {
            panel = panel.child(
                div()
                    .pl(8.0)
                    .child(label(item).font_size(11.0).color(white)),
            );
        }

        // Divider
        panel = panel.child(divider());

        // Properties section
        panel = panel.child(label("Properties").font_size(12.0).color(dim));

        if entry.properties.is_empty() {
            panel = panel.child(
                div()
                    .pl(8.0)
                    .child(label("(none)").font_size(11.0).color(dim)),
            );
        } else {
            for (key, value) in &entry.properties {
                let prop_text = format!("{}: {}", key, value);
                panel = panel.child(
                    div()
                        .pl(8.0)
                        .child(label(&prop_text).font_size(11.0).color(white)),
                );
            }
        }

        panel
    } else {
        div()
            .flex_col()
            .w(280.0)
            .h_full()
            .bg(panel_bg)
            .p(12.0)
            .items_center()
            .justify_start()
            .child(
                div()
                    .pt(40.0)
                    .child(label("Hover over an element").font_size(12.0).color(dim)),
            )
    };

    // Root: full-window flex row with highlight area + detail panel.
    Box::new(
        div()
            .w_full()
            .h_full()
            .flex_row()
            .child(highlight_area)
            .child(detail_panel),
    )
}
