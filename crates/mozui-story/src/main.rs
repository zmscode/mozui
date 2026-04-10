use mozui_assets::Assets;
use mozui_story::{Gallery, init, create_new_window};

fn main() {
    let app = mozui::platform::application().with_assets(Assets);

    // Parse `cargo run -- <story_name>`
    let name = std::env::args().nth(1);

    app.run(move |cx| {
        init(cx);
        cx.activate(true);

        create_new_window(
            "GPUI Component",
            move |window, cx| Gallery::view(name.as_deref(), window, cx),
            cx,
        );
    });
}
