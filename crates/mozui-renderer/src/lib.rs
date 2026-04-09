mod atlas;
mod draw;
mod glyph_pipeline;
mod gpu;
mod rect_pipeline;

pub use draw::{Border, DrawCommand, DrawList};
pub use gpu::GpuContext;
pub use rect_pipeline::{FILL_LINEAR_GRADIENT, FILL_RADIAL_GRADIENT, FILL_SOLID, RectInstance};

use atlas::{GlyphKey, IconKey, TextureAtlas};
use glyph_pipeline::{GlyphInstance, GlyphPipeline};
use mozui_platform::PlatformWindow;
use mozui_style::{Color, Size};
use mozui_text::{FontSystem, TextStyle};

pub struct Renderer {
    gpu: GpuContext,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    rect_pipeline: rect_pipeline::RectPipeline,
    glyph_pipeline: GlyphPipeline,
    atlas: TextureAtlas,
    font_system: FontSystem,
    atlas_dirty: bool,
}

impl Renderer {
    pub fn new(window: &dyn PlatformWindow) -> Self {
        let (gpu, surface) = GpuContext::new_with_surface(window);

        let size = window.content_size();
        let scale = window.scale_factor();
        let physical_width = (size.width * scale) as u32;
        let physical_height = (size.height * scale) as u32;

        let surface_config = surface
            .get_default_config(&gpu.adapter, physical_width.max(1), physical_height.max(1))
            .expect("Surface not supported by adapter");

        surface.configure(&gpu.device, &surface_config);

        let rect_pipeline = rect_pipeline::RectPipeline::new(&gpu.device, surface_config.format);
        let glyph_pipeline = GlyphPipeline::new(&gpu.device, surface_config.format);
        let atlas = TextureAtlas::new(&gpu.device, 1024);
        let font_system = FontSystem::new();

        Self {
            gpu,
            surface,
            surface_config,
            rect_pipeline,
            glyph_pipeline,
            atlas,
            font_system,
            atlas_dirty: true,
        }
    }

    pub fn font_system(&self) -> &FontSystem {
        &self.font_system
    }

    pub fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub fn resize(&mut self, size: Size, scale: f32) {
        let width = (size.width * scale) as u32;
        let height = (size.height * scale) as u32;
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface
                .configure(&self.gpu.device, &self.surface_config);
        }
    }

    pub fn render(
        &mut self,
        clear_color: Color,
        draw_list: &DrawList,
        _logical_size: Size,
        scale: f32,
    ) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface
                    .configure(&self.gpu.device, &self.surface_config);
                return;
            }
            Err(e) => {
                tracing::error!("Surface error: {:?}", e);
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mozui_encoder"),
            });

        // Collect rect and glyph instances, tracking interleaved batches
        // to preserve paint order (critical for overlays like dialogs).
        let mut rects: Vec<RectInstance> = Vec::new();
        let mut glyph_instances: Vec<GlyphInstance> = Vec::new();
        // Each batch: (is_glyphs, start_index, count) into the respective vec
        let mut batches: Vec<(bool, usize, usize)> = Vec::new();
        let mut current_is_glyphs: Option<bool> = None;

        for cmd in draw_list.commands() {
            let rect_start = rects.len();
            let glyph_start = glyph_instances.len();

            match cmd {
                DrawCommand::Rect {
                    bounds,
                    background,
                    corner_radii,
                    border,
                    shadow,
                } => {
                    // Emit shadow instance before the main rect (Gaussian blur via erf)
                    if let Some(shadow) = shadow {
                        let blur_px = shadow.blur * scale;
                        let spread_px = shadow.spread * scale;
                        // Shadow bounds = original rect + offset, dilated by spread
                        let sx = (bounds.origin.x + shadow.offset_x) * scale - spread_px;
                        let sy = (bounds.origin.y + shadow.offset_y) * scale - spread_px;
                        let sw = bounds.size.width * scale + spread_px * 2.0;
                        let sh = bounds.size.height * scale + spread_px * 2.0;
                        rects.push(RectInstance {
                            bounds: [sx, sy, sw, sh],
                            color: shadow.color.to_array(),
                            corner_radii: [
                                corner_radii.top_left * scale,
                                corner_radii.top_right * scale,
                                corner_radii.bottom_right * scale,
                                corner_radii.bottom_left * scale,
                            ],
                            border_width: 0.0,
                            border_color: [0.0; 4],
                            fill_mode: FILL_SOLID,
                            gradient_angle: 0.0,
                            gradient_stop1_color: [0.0; 4],
                            gradient_stop0_pos: 0.0,
                            gradient_stop1_pos: 1.0,
                            shadow_blur: blur_px,
                            _padding: 0.0,
                        });
                    }

                    let (fill_mode, color, gradient_angle, stop1_color, stop0_pos, stop1_pos) =
                        match background {
                            mozui_style::Fill::Solid(c) => {
                                (FILL_SOLID, c.to_array(), 0.0, [0.0; 4], 0.0, 1.0)
                            }
                            mozui_style::Fill::LinearGradient { angle, stops } => {
                                let c0 =
                                    stops.first().map(|(_, c)| c.to_array()).unwrap_or([1.0; 4]);
                                let c1 = stops.get(1).map(|(_, c)| c.to_array()).unwrap_or(c0);
                                let p0 = stops.first().map(|(p, _)| *p).unwrap_or(0.0);
                                let p1 = stops.get(1).map(|(p, _)| *p).unwrap_or(1.0);
                                (FILL_LINEAR_GRADIENT, c0, *angle, c1, p0, p1)
                            }
                            mozui_style::Fill::RadialGradient { stops, .. } => {
                                let c0 =
                                    stops.first().map(|(_, c)| c.to_array()).unwrap_or([1.0; 4]);
                                let c1 = stops.get(1).map(|(_, c)| c.to_array()).unwrap_or(c0);
                                let p0 = stops.first().map(|(p, _)| *p).unwrap_or(0.0);
                                let p1 = stops.get(1).map(|(p, _)| *p).unwrap_or(1.0);
                                (FILL_RADIAL_GRADIENT, c0, 0.0, c1, p0, p1)
                            }
                        };

                    let (border_width, border_color) = border
                        .as_ref()
                        .map(|b| (b.width, b.color.to_array()))
                        .unwrap_or((0.0, [0.0; 4]));

                    rects.push(RectInstance {
                        bounds: [
                            bounds.origin.x * scale,
                            bounds.origin.y * scale,
                            bounds.size.width * scale,
                            bounds.size.height * scale,
                        ],
                        color,
                        corner_radii: [
                            corner_radii.top_left * scale,
                            corner_radii.top_right * scale,
                            corner_radii.bottom_right * scale,
                            corner_radii.bottom_left * scale,
                        ],
                        border_width: border_width * scale,
                        border_color,
                        fill_mode,
                        gradient_angle,
                        gradient_stop1_color: stop1_color,
                        gradient_stop0_pos: stop0_pos,
                        gradient_stop1_pos: stop1_pos,
                        shadow_blur: 0.0,
                        _padding: 0.0,
                    });
                }
                DrawCommand::Icon {
                    name,
                    weight,
                    bounds,
                    color,
                    size_px,
                } => {
                    let physical_size = (*size_px * scale) as u16;
                    let key = IconKey {
                        name: *name,
                        weight: *weight,
                        size_px: physical_size,
                    };

                    // Rasterize if not cached
                    if self.atlas.get_icon(&key).is_none() {
                        if let Some(alpha_bitmap) =
                            rasterize_icon_svg(name.svg(*weight), physical_size as u32)
                        {
                            self.atlas.insert_icon(
                                &self.gpu.queue,
                                key,
                                physical_size as u32,
                                physical_size as u32,
                                &alpha_bitmap,
                            );
                            self.atlas_dirty = true;
                        }
                    }

                    if let Some(region) = self.atlas.get_icon(&key) {
                        if region.width > 0 && region.height > 0 {
                            let atlas_size = self.atlas.size as f32;
                            // Center the icon within the bounds
                            let icon_w = region.width as f32;
                            let icon_h = region.height as f32;
                            let gx = bounds.origin.x * scale
                                + (bounds.size.width * scale - icon_w) / 2.0;
                            let gy = bounds.origin.y * scale
                                + (bounds.size.height * scale - icon_h) / 2.0;

                            glyph_instances.push(GlyphInstance {
                                bounds: [gx, gy, icon_w, icon_h],
                                uv: [
                                    region.x as f32 / atlas_size,
                                    region.y as f32 / atlas_size,
                                    (region.x + region.width) as f32 / atlas_size,
                                    (region.y + region.height) as f32 / atlas_size,
                                ],
                                color: color.to_array(),
                            });
                        }
                    }
                }
                DrawCommand::Text {
                    text,
                    bounds,
                    font_size,
                    color,
                    weight,
                    italic,
                } => {
                    let text_style = TextStyle {
                        font_size: *font_size,
                        weight: if *weight >= 700 {
                            mozui_text::FontWeight::Bold
                        } else {
                            mozui_text::FontWeight::Regular
                        },
                        slant: if *italic {
                            mozui_text::FontSlant::Italic
                        } else {
                            mozui_text::FontSlant::Normal
                        },
                        color: *color,
                        ..Default::default()
                    };

                    let run = mozui_text::shaping::shape_text(text, &text_style, &self.font_system);
                    let font_id = self.font_system.default_font();
                    let metrics = self.font_system.font_metrics(font_id, *font_size);
                    // Center the glyph run vertically within the text element's bounds.
                    // leading = line_height - (ascent - descent); descent is negative
                    let line_height = bounds.size.height;
                    let leading = line_height - (metrics.ascent - metrics.descent);
                    let baseline_y = bounds.origin.y + leading / 2.0 + metrics.ascent;
                    let size_px = (*font_size * scale) as u16;
                    let atlas_size = self.atlas.size as f32;

                    for glyph in &run.glyphs {
                        if glyph.glyph_id == 0 {
                            continue;
                        }

                        let key = GlyphKey {
                            font_id: glyph.font_id,
                            glyph_id: glyph.glyph_id,
                            size_px,
                        };

                        // Rasterize if not cached
                        if self.atlas.get(&key).is_none() {
                            let font = self.font_system.get_font(glyph.font_id);
                            let raster_scale = *font_size * scale;

                            use pathfinder_geometry::transform2d::Transform2F;
                            use pathfinder_geometry::vector::Vector2F;
                            let identity = Transform2F::default();
                            match font.raster_bounds(
                                glyph.glyph_id,
                                raster_scale,
                                identity,
                                font_kit::hinting::HintingOptions::None,
                                font_kit::canvas::RasterizationOptions::GrayscaleAa,
                            ) {
                                Ok(raster_bounds) => {
                                    let w = raster_bounds.size().x() as u32;
                                    let h = raster_bounds.size().y() as u32;

                                    if w > 0 && h > 0 {
                                        let mut canvas = font_kit::canvas::Canvas::new(
                                            pathfinder_geometry::vector::Vector2I::new(
                                                w as i32, h as i32,
                                            ),
                                            font_kit::canvas::Format::A8,
                                        );

                                        // Translate so the glyph renders within the canvas
                                        let raster_transform =
                                            Transform2F::from_translation(Vector2F::new(
                                                -raster_bounds.origin().x() as f32,
                                                -raster_bounds.origin().y() as f32,
                                            ));

                                        let _ = font.rasterize_glyph(
                                            &mut canvas,
                                            glyph.glyph_id,
                                            raster_scale,
                                            raster_transform,
                                            font_kit::hinting::HintingOptions::None,
                                            font_kit::canvas::RasterizationOptions::GrayscaleAa,
                                        );

                                        let bearing_x = raster_bounds.origin().x() as f32;
                                        let bearing_y = raster_bounds.origin().y() as f32;

                                        // font-kit canvas stride may differ from width
                                        let data = if canvas.stride as u32 != w {
                                            let mut packed = Vec::with_capacity((w * h) as usize);
                                            for row in 0..h {
                                                let start = (row as usize) * canvas.stride;
                                                packed.extend_from_slice(
                                                    &canvas.pixels[start..start + w as usize],
                                                );
                                            }
                                            packed
                                        } else {
                                            canvas.pixels.clone()
                                        };

                                        self.atlas.insert(
                                            &self.gpu.queue,
                                            key,
                                            w,
                                            h,
                                            bearing_x,
                                            bearing_y,
                                            &data,
                                        );
                                        self.atlas_dirty = true;
                                    } else {
                                        self.atlas.insert(
                                            &self.gpu.queue,
                                            key,
                                            0,
                                            0,
                                            0.0,
                                            0.0,
                                            &[],
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        ?e,
                                        glyph_id = glyph.glyph_id,
                                        "raster_bounds failed"
                                    );
                                }
                            }
                        }

                        if let Some(region) = self.atlas.get(&key) {
                            if region.width == 0 || region.height == 0 {
                                continue;
                            }

                            let gx = (bounds.origin.x + glyph.x_offset) * scale + region.bearing_x;
                            let gy = baseline_y * scale + region.bearing_y;

                            glyph_instances.push(GlyphInstance {
                                bounds: [gx, gy, region.width as f32, region.height as f32],
                                uv: [
                                    region.x as f32 / atlas_size,
                                    region.y as f32 / atlas_size,
                                    (region.x + region.width) as f32 / atlas_size,
                                    (region.y + region.height) as f32 / atlas_size,
                                ],
                                color: color.to_array(),
                            });
                        }
                    }
                }
            }

            // Track interleaved batches for correct z-ordering
            let rect_count = rects.len() - rect_start;
            let glyph_count = glyph_instances.len() - glyph_start;
            if rect_count > 0 {
                if current_is_glyphs != Some(false) {
                    batches.push((false, rect_start, 0));
                    current_is_glyphs = Some(false);
                }
                batches.last_mut().unwrap().2 += rect_count;
            }
            if glyph_count > 0 {
                if current_is_glyphs != Some(true) {
                    batches.push((true, glyph_start, 0));
                    current_is_glyphs = Some(true);
                }
                batches.last_mut().unwrap().2 += glyph_count;
            }
        }

        // Update atlas bind group if dirty
        if self.atlas_dirty {
            self.glyph_pipeline
                .update_atlas(&self.gpu.device, &self.atlas.view);
            self.atlas_dirty = false;
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mozui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color.r as f64,
                            g: clear_color.g as f64,
                            b: clear_color.b as f64,
                            a: clear_color.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw interleaved batches to preserve paint order
            let vw = self.surface_config.width as f32;
            let vh = self.surface_config.height as f32;
            for &(is_glyphs, start, count) in &batches {
                if is_glyphs {
                    self.glyph_pipeline.draw(
                        &self.gpu.device,
                        &self.gpu.queue,
                        &mut pass,
                        &glyph_instances[start..start + count],
                        vw,
                        vh,
                    );
                } else {
                    self.rect_pipeline.draw(
                        &self.gpu.device,
                        &self.gpu.queue,
                        &mut pass,
                        &rects[start..start + count],
                        vw,
                        vh,
                    );
                }
            }
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

/// Rasterize an SVG string to an alpha bitmap at the given pixel size.
/// Returns a Vec<u8> of R8 alpha values, or None on failure.
fn rasterize_icon_svg(svg_data: &str, size_px: u32) -> Option<Vec<u8>> {
    // Phosphor icons use currentColor — replace with white so we get full alpha
    let svg_white = svg_data.replace("currentColor", "white");

    let tree = resvg::usvg::Tree::from_str(&svg_white, &resvg::usvg::Options::default()).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(size_px, size_px)?;

    let svg_size = tree.size();
    let scale_x = size_px as f32 / svg_size.width();
    let scale_y = size_px as f32 / svg_size.height();

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale_x, scale_y),
        &mut pixmap.as_mut(),
    );

    // Convert RGBA pixmap to R8 alpha channel only
    let rgba = pixmap.data();
    let mut alpha = Vec::with_capacity((size_px * size_px) as usize);
    for pixel in rgba.chunks_exact(4) {
        alpha.push(pixel[3]); // alpha channel
    }

    Some(alpha)
}
