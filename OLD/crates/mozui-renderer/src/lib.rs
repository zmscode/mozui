mod atlas;
mod draw;
mod glyph_pipeline;
mod gpu;
mod image_pipeline;
mod rect_pipeline;

pub use draw::{Border, DrawCommand, DrawList, ImageData, ObjectFit};
pub use gpu::GpuContext;
pub use rect_pipeline::{FILL_LINEAR_GRADIENT, FILL_RADIAL_GRADIENT, FILL_SOLID, RectInstance};

use atlas::{IconKey, TextureAtlas};
use glyph_pipeline::{GlyphInstance, GlyphPipeline};
use image_pipeline::{ImageInstance, ImagePipeline};
#[cfg(not(target_arch = "wasm32"))]
use mozui_platform::PlatformWindow;
use mozui_style::{Color, Size};
use mozui_text::{FontSystem, TextStyle};
use std::sync::Arc;

pub struct Renderer {
    gpu: GpuContext,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    rect_pipeline: rect_pipeline::RectPipeline,
    glyph_pipeline: GlyphPipeline,
    image_pipeline: ImagePipeline,
    atlas: TextureAtlas,
    font_system: FontSystem,
    atlas_dirty: bool,
}

impl Renderer {
    /// Create a renderer from a platform window (native path).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(window: &dyn PlatformWindow) -> Self {
        let (gpu, surface) = GpuContext::new_with_surface(window);
        let size = window.content_size();
        let scale = window.scale_factor();
        Self::from_gpu(gpu, surface, size, scale)
    }

    /// Create a renderer from a pre-built GPU context and surface.
    /// Used by the WASM path where GPU initialization is async.
    pub fn from_gpu(
        gpu: GpuContext,
        surface: wgpu::Surface<'static>,
        size: Size,
        scale: f32,
    ) -> Self {
        let physical_width = (size.width * scale) as u32;
        let physical_height = (size.height * scale) as u32;

        let surface_config = surface
            .get_default_config(&gpu.adapter, physical_width.max(1), physical_height.max(1))
            .expect("Surface not supported by adapter");

        surface.configure(&gpu.device, &surface_config);

        let rect_pipeline = rect_pipeline::RectPipeline::new(&gpu.device, surface_config.format);
        let glyph_pipeline = GlyphPipeline::new(&gpu.device, surface_config.format);
        let image_pipeline = ImagePipeline::new(&gpu.device, surface_config.format);
        let atlas = TextureAtlas::new(&gpu.device, 2048);
        let font_system = FontSystem::new();

        Self {
            gpu,
            surface,
            surface_config,
            rect_pipeline,
            glyph_pipeline,
            image_pipeline,
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

        // Collect rect, glyph, and image instances, tracking interleaved batches
        // to preserve paint order (critical for overlays like dialogs).
        let mut rects: Vec<RectInstance> = Vec::new();
        let mut glyph_instances: Vec<GlyphInstance> = Vec::new();
        // Image batches: each image command is a separate draw call (different texture)
        let mut image_batches: Vec<(Arc<draw::ImageData>, ImageInstance)> = Vec::new();

        // Batch types: 0=rects, 1=glyphs, 2=image(index into image_batches)
        #[derive(Clone, Copy, PartialEq)]
        enum BatchKind {
            Rects,
            Glyphs,
            Image(usize),
        }
        let mut batches: Vec<(BatchKind, usize, usize)> = Vec::new();
        let mut current_kind: Option<BatchKind> = None;

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
                    shadows,
                } => {
                    // Emit outer shadow instance before the main rect (renders behind)
                    if let Some(shadow) = shadow {
                        if !shadow.inset {
                            let blur_px = shadow.blur * scale;
                            let spread_px = shadow.spread * scale;
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
                                shadow_inset: 0.0,
                            });
                        }
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
                        shadow_inset: 0.0,
                    });

                    // Emit inset shadows after the main rect (renders on top)
                    let all_shadows = shadow.iter().filter(|s| s.inset).chain(shadows.iter().filter(|s| s.inset));
                    for inset_shadow in all_shadows {
                        let blur_px = inset_shadow.blur * scale;
                        let spread_px = inset_shadow.spread * scale;
                        let ix = (bounds.origin.x + inset_shadow.offset_x) * scale + spread_px;
                        let iy = (bounds.origin.y + inset_shadow.offset_y) * scale + spread_px;
                        let iw = bounds.size.width * scale - spread_px * 2.0;
                        let ih = bounds.size.height * scale - spread_px * 2.0;
                        rects.push(RectInstance {
                            bounds: [ix, iy, iw.max(0.0), ih.max(0.0)],
                            color: inset_shadow.color.to_array(),
                            corner_radii: [
                                (corner_radii.top_left * scale - spread_px).max(0.0),
                                (corner_radii.top_right * scale - spread_px).max(0.0),
                                (corner_radii.bottom_right * scale - spread_px).max(0.0),
                                (corner_radii.bottom_left * scale - spread_px).max(0.0),
                            ],
                            border_width: 0.0,
                            border_color: [0.0; 4],
                            fill_mode: FILL_SOLID,
                            gradient_angle: 0.0,
                            gradient_stop1_color: [0.0; 4],
                            gradient_stop0_pos: 0.0,
                            gradient_stop1_pos: 1.0,
                            shadow_blur: blur_px,
                            shadow_inset: 1.0,
                        });
                    }
                    // Emit outer shadows from the shadows vec before the main rect
                    // (already handled: the primary `shadow` field outer shadow is above)
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

                    // cosmic-text's render() passes (0, run.line_y) as offset to
                    // physical(). glyph.y is 0 for the first line, so offset.1
                    // should be the baseline Y. We compute baseline to vertically
                    // center text within the bounds box.
                    let ascent = run.max_ascent;
                    let descent = run.max_descent;
                    let text_height = ascent + descent;
                    let box_height = bounds.size.height;
                    let baseline_y = bounds.origin.y + (box_height - text_height) / 2.0 + ascent;
                    let atlas_size = self.atlas.size as f32;

                    // physical() adds offset AFTER scaling: (glyph.x * scale + offset.0)
                    // so offset must be in physical pixels.
                    let phys_offset = (bounds.origin.x * scale, baseline_y * scale);

                    for glyph in &run.glyphs {
                        if glyph.glyph_id == 0 {
                            continue;
                        }

                        // Get sub-pixel-aware cache key at render scale
                        let physical = glyph.layout_glyph.physical(phys_offset, scale);
                        let cache_key = physical.cache_key;

                        // Rasterize if not cached
                        if self.atlas.get(&cache_key).is_none() {
                            if let Some((bitmap, placement)) =
                                rasterize_glyph_swash(&self.font_system, cache_key)
                            {
                                let w = placement.width;
                                let h = placement.height;
                                let bearing_x = placement.left as f32;
                                let bearing_y = -placement.top as f32;

                                self.atlas.insert(
                                    &self.gpu.queue,
                                    cache_key,
                                    w,
                                    h,
                                    bearing_x,
                                    bearing_y,
                                    &bitmap,
                                );
                                self.atlas_dirty = true;
                            } else {
                                self.atlas
                                    .insert(&self.gpu.queue, cache_key, 0, 0, 0.0, 0.0, &[]);
                            }
                        }

                        if let Some(region) = self.atlas.get(&cache_key) {
                            if region.width == 0 || region.height == 0 {
                                continue;
                            }

                            let gx = physical.x as f32 + region.bearing_x;
                            let gy = physical.y as f32 + region.bearing_y;

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
                DrawCommand::Image {
                    bounds,
                    data,
                    corner_radii,
                    opacity,
                    object_fit,
                } => {
                    let (uv, draw_bounds) = compute_image_uv_and_bounds(
                        bounds,
                        data.width,
                        data.height,
                        *object_fit,
                        scale,
                    );

                    let img_idx = image_batches.len();
                    image_batches.push((
                        data.clone(),
                        ImageInstance {
                            bounds: [
                                draw_bounds[0],
                                draw_bounds[1],
                                draw_bounds[2],
                                draw_bounds[3],
                            ],
                            uv,
                            corner_radii: [
                                corner_radii.top_left * scale,
                                corner_radii.top_right * scale,
                                corner_radii.bottom_right * scale,
                                corner_radii.bottom_left * scale,
                            ],
                            opacity: *opacity,
                            _padding: [0.0; 3],
                        },
                    ));

                    batches.push((BatchKind::Image(img_idx), 0, 1));
                    current_kind = None;
                }
            }

            // Track interleaved batches for correct z-ordering
            let rect_count = rects.len() - rect_start;
            let glyph_count = glyph_instances.len() - glyph_start;
            if rect_count > 0 {
                if current_kind != Some(BatchKind::Rects) {
                    batches.push((BatchKind::Rects, rect_start, 0));
                    current_kind = Some(BatchKind::Rects);
                }
                batches.last_mut().unwrap().2 += rect_count;
            }
            if glyph_count > 0 {
                if current_kind != Some(BatchKind::Glyphs) {
                    batches.push((BatchKind::Glyphs, glyph_start, 0));
                    current_kind = Some(BatchKind::Glyphs);
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

        // Ensure all image textures are uploaded
        for (data, _) in &image_batches {
            self.image_pipeline
                .ensure_uploaded(&self.gpu.device, &self.gpu.queue, data);
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
            for &(kind, start, count) in &batches {
                match kind {
                    BatchKind::Rects => {
                        self.rect_pipeline.draw(
                            &self.gpu.device,
                            &self.gpu.queue,
                            &mut pass,
                            &rects[start..start + count],
                            vw,
                            vh,
                        );
                    }
                    BatchKind::Glyphs => {
                        self.glyph_pipeline.draw(
                            &self.gpu.device,
                            &self.gpu.queue,
                            &mut pass,
                            &glyph_instances[start..start + count],
                            vw,
                            vh,
                        );
                    }
                    BatchKind::Image(idx) => {
                        let (ref data, ref instance) = image_batches[idx];
                        if let Some(bind_group) = self.image_pipeline.get_bind_group(data.id) {
                            self.image_pipeline.draw(
                                &self.gpu.device,
                                &self.gpu.queue,
                                &mut pass,
                                std::slice::from_ref(instance),
                                bind_group,
                                vw,
                                vh,
                            );
                        }
                    }
                }
            }
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

/// Rasterize a glyph using swash, returning (alpha_bitmap, placement).
fn rasterize_glyph_swash(
    font_system: &FontSystem,
    cache_key: cosmic_text::CacheKey,
) -> Option<(Vec<u8>, swash::zeno::Placement)> {
    use swash::scale::{Render, ScaleContext, Source, StrikeWith};
    use swash::zeno::Format;

    let mut fs = font_system.borrow_mut();

    // Look up the font in cosmic-text's database
    let font = fs.get_font(cache_key.font_id, cache_key.font_weight)?;
    let font_ref = font.as_swash();
    let font_size = f32::from_bits(cache_key.font_size_bits);

    let mut context = ScaleContext::new();
    let mut scaler = context.builder(font_ref).size(font_size).hint(true).build();

    let image = Render::new(&[
        Source::ColorOutline(0),
        Source::ColorBitmap(StrikeWith::BestFit),
        Source::Outline,
    ])
    .format(Format::Alpha)
    .offset(swash::zeno::Vector {
        x: cache_key.x_bin.as_float(),
        y: cache_key.y_bin.as_float(),
    })
    .render(&mut scaler, cache_key.glyph_id)?;

    Some((image.data, image.placement))
}

/// Compute UV coordinates and draw bounds for an image based on object-fit mode.
fn compute_image_uv_and_bounds(
    bounds: &mozui_style::Rect,
    img_w: u32,
    img_h: u32,
    fit: draw::ObjectFit,
    scale: f32,
) -> ([f32; 4], [f32; 4]) {
    let bx = bounds.origin.x * scale;
    let by = bounds.origin.y * scale;
    let bw = bounds.size.width * scale;
    let bh = bounds.size.height * scale;

    match fit {
        draw::ObjectFit::Fill => ([0.0, 0.0, 1.0, 1.0], [bx, by, bw, bh]),
        draw::ObjectFit::Contain => {
            let img_aspect = img_w as f32 / img_h as f32;
            let box_aspect = bw / bh;
            let (dw, dh) = if img_aspect > box_aspect {
                (bw, bw / img_aspect)
            } else {
                (bh * img_aspect, bh)
            };
            let dx = bx + (bw - dw) / 2.0;
            let dy = by + (bh - dh) / 2.0;
            ([0.0, 0.0, 1.0, 1.0], [dx, dy, dw, dh])
        }
        draw::ObjectFit::Cover => {
            let img_aspect = img_w as f32 / img_h as f32;
            let box_aspect = bw / bh;
            let uv = if img_aspect > box_aspect {
                let visible_frac = box_aspect / img_aspect;
                let offset = (1.0 - visible_frac) / 2.0;
                [offset, 0.0, offset + visible_frac, 1.0]
            } else {
                let visible_frac = img_aspect / box_aspect;
                let offset = (1.0 - visible_frac) / 2.0;
                [0.0, offset, 1.0, offset + visible_frac]
            };
            (uv, [bx, by, bw, bh])
        }
    }
}

/// Rasterize an SVG string to an alpha bitmap at the given pixel size.
fn rasterize_icon_svg(svg_data: &str, size_px: u32) -> Option<Vec<u8>> {
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
    let rgba = pixmap.data();
    let mut alpha = Vec::with_capacity((size_px * size_px) as usize);
    for pixel in rgba.chunks_exact(4) {
        alpha.push(pixel[3]);
    }
    Some(alpha)
}
