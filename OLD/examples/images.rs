use mozui::*;
use std::sync::Arc;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mozui=debug")
        .init();

    App::new()
        .theme(Theme::dark())
        .window(WindowOptions {
            title: "Images & SVG".into(),
            size: Size::new(800.0, 700.0),
            titlebar: TitlebarStyle::Transparent,
            ..Default::default()
        })
        .run(app);
}

/// Generate a simple test SVG with a gradient circle.
fn test_svg() -> &'static str {
    r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
        <defs>
            <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" style="stop-color:#cba6f7"/>
                <stop offset="100%" style="stop-color:#89b4fa"/>
            </linearGradient>
        </defs>
        <circle cx="100" cy="100" r="90" fill="url(#g)"/>
        <text x="100" y="108" text-anchor="middle" font-size="24" fill="white" font-family="sans-serif">SVG</text>
    </svg>"#
}

/// Generate a procedural RGBA test image (gradient pattern).
fn generate_test_image(w: u32, h: u32) -> Arc<ImageData> {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let r = (x as f32 / w as f32 * 255.0) as u8;
            let g = (y as f32 / h as f32 * 255.0) as u8;
            let b = 180u8;
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Arc::new(ImageData::new(pixels, w, h))
}

fn app(cx: &mut Context) -> Box<dyn Element> {
    let theme = cx.theme().clone();
    let scroll = cx.use_scroll();

    // Decode SVG once and cache in signal
    let (svg_sig, _) = cx.use_signal::<Option<Arc<ImageData>>>(None);
    let svg_data = cx.get(svg_sig).clone();
    let svg_img = svg_data.unwrap_or_else(|| {
        let decoded = decode_svg(test_svg(), 200, 200).unwrap();
        // Can't set signal during render, so just use the decoded value
        decoded
    });

    // Generate procedural test image
    let (test_sig, _) = cx.use_signal::<Option<Arc<ImageData>>>(None);
    let test_data = cx.get(test_sig).clone();
    let test_img = test_data.unwrap_or_else(|| generate_test_image(256, 256));

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
            .child(label("Images & SVG").color(muted).font_size(12.0)),
    );

    let content = div()
        .w_full()
        .flex_1()
        .flex_col()
        .overflow_y_scroll(scroll)
        .p(24.0)
        .gap(32.0)
        // ── SVG rendering ──
        .child(
            div()
                .flex_col()
                .gap(12.0)
                .child(
                    label("SVG Rendering")
                        .color(heading_color)
                        .font_size(18.0)
                        .bold(),
                )
                .child(
                    label("SVGs are rasterized via resvg at any resolution.")
                        .color(muted)
                        .font_size(13.0),
                )
                .child(
                    div()
                        .flex_row()
                        .gap(16.0)
                        .items_center()
                        .child(img(svg_img.clone()).w(100.0).h(100.0))
                        .child(img(svg_img.clone()).w(150.0).h(150.0).rounded(16.0))
                        .child(
                            img(svg_img.clone())
                                .w(100.0)
                                .h(100.0)
                                .rounded(50.0), // circle
                        ),
                ),
        )
        // ── Procedural image ──
        .child(
            div()
                .flex_col()
                .gap(12.0)
                .child(
                    label("Procedural Image")
                        .color(heading_color)
                        .font_size(18.0)
                        .bold(),
                )
                .child(
                    label("Generated RGBA pixels uploaded as GPU texture.")
                        .color(muted)
                        .font_size(13.0),
                )
                .child(
                    div()
                        .flex_row()
                        .gap(16.0)
                        .items_center()
                        .child(img(test_img.clone()).w(150.0).h(150.0))
                        .child(
                            img(test_img.clone())
                                .w(150.0)
                                .h(150.0)
                                .rounded(theme.radius_lg),
                        )
                        .child(img(test_img.clone()).w(150.0).h(150.0).opacity(0.5)),
                ),
        )
        // ── Object-fit modes ──
        .child(
            div()
                .flex_col()
                .gap(12.0)
                .child(
                    label("Object-Fit Modes")
                        .color(heading_color)
                        .font_size(18.0)
                        .bold(),
                )
                .child(
                    label("Same image rendered with different fit modes in a 200x100 box.")
                        .color(muted)
                        .font_size(13.0),
                )
                .child(
                    div()
                        .flex_row()
                        .gap(16.0)
                        .child(
                            div()
                                .flex_col()
                                .gap(4.0)
                                .child(label("Cover").color(muted).font_size(12.0))
                                .child(
                                    div()
                                        .w(200.0)
                                        .h(100.0)
                                        .bg(theme.surface)
                                        .rounded(theme.radius_md)
                                        .child(
                                            img(test_img.clone())
                                                .w(200.0)
                                                .h(100.0)
                                                .cover()
                                                .rounded(theme.radius_md),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex_col()
                                .gap(4.0)
                                .child(label("Contain").color(muted).font_size(12.0))
                                .child(
                                    div()
                                        .w(200.0)
                                        .h(100.0)
                                        .bg(theme.surface)
                                        .rounded(theme.radius_md)
                                        .child(
                                            img(test_img.clone())
                                                .w(200.0)
                                                .h(100.0)
                                                .contain()
                                                .rounded(theme.radius_md),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex_col()
                                .gap(4.0)
                                .child(label("Fill").color(muted).font_size(12.0))
                                .child(
                                    div()
                                        .w(200.0)
                                        .h(100.0)
                                        .bg(theme.surface)
                                        .rounded(theme.radius_md)
                                        .child(
                                            img(test_img.clone())
                                                .w(200.0)
                                                .h(100.0)
                                                .fill()
                                                .rounded(theme.radius_md),
                                        ),
                                ),
                        ),
                ),
        )
        // ── File loading info ──
        .child(
            div()
                .flex_col()
                .gap(8.0)
                .child(
                    label("Loading Images")
                        .color(heading_color)
                        .font_size(18.0)
                        .bold(),
                )
                .child(
                    label("Use decode_image_file(\"path.png\") or decode_image(&bytes) for PNG/JPEG/WebP.")
                        .color(muted)
                        .font_size(13.0),
                )
                .child(
                    label("Use decode_svg(svg_str, w, h) or decode_svg_file(\"path.svg\", w, h) for SVGs.")
                        .color(muted)
                        .font_size(13.0),
                )
                .child(
                    label("Use decode_gif_frames(&bytes) + img_animated() for animated GIFs.")
                        .color(muted)
                        .font_size(13.0),
                ),
        );

    root = root.child(content);
    Box::new(root)
}
