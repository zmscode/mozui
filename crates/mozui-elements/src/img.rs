use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList, ImageData, ObjectFit};
use mozui_style::{Corners, Rect};
use mozui_text::FontSystem;
use std::sync::Arc;
use std::time::{Duration, Instant};
use taffy::prelude::*;

// ── Decode functions ─────────────────────────────────────────────

/// Decode an image from raw bytes (PNG, JPEG, WebP, GIF first frame).
/// Returns decoded RGBA pixel data.
pub fn decode_image(bytes: &[u8]) -> Option<Arc<ImageData>> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some(Arc::new(ImageData::new(rgba.into_raw(), w, h)))
}

/// Decode an image from a file path.
pub fn decode_image_file(path: &str) -> Option<Arc<ImageData>> {
    let bytes = std::fs::read(path).ok()?;
    decode_image(&bytes)
}

/// Rasterize an SVG string to RGBA pixels at the given dimensions.
/// If width/height are 0, uses the SVG's intrinsic size.
pub fn decode_svg(svg: &str, width: u32, height: u32) -> Option<Arc<ImageData>> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let svg_size = tree.size();
    let w = if width > 0 { width } else { svg_size.width() as u32 };
    let h = if height > 0 { height } else { svg_size.height() as u32 };

    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let scale_x = w as f32 / svg_size.width();
    let scale_y = h as f32 / svg_size.height();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale_x, scale_y),
        &mut pixmap.as_mut(),
    );

    Some(Arc::new(ImageData::new(pixmap.take(), w, h)))
}

/// Rasterize an SVG file to RGBA pixels at the given dimensions.
pub fn decode_svg_file(path: &str, width: u32, height: u32) -> Option<Arc<ImageData>> {
    let svg = std::fs::read_to_string(path).ok()?;
    decode_svg(&svg, width, height)
}

/// Decode all frames of an animated GIF.
/// Returns a list of (frame_data, frame_duration).
pub fn decode_gif_frames(bytes: &[u8]) -> Option<Vec<(Arc<ImageData>, Duration)>> {
    use image::codecs::gif::GifDecoder;
    use image::{AnimationDecoder, ImageDecoder};

    let decoder = GifDecoder::new(std::io::Cursor::new(bytes)).ok()?;
    let (full_w, full_h) = decoder.dimensions();
    let frames = decoder.into_frames().collect_frames().ok()?;

    if frames.is_empty() {
        return None;
    }

    let mut result = Vec::with_capacity(frames.len());
    for frame in frames {
        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay = Duration::from_millis(numer as u64 / denom.max(1) as u64);
        let rgba = frame.into_buffer();
        let (w, h) = rgba.dimensions();
        // GIF frames may be smaller than the full canvas; ensure full size
        if w == full_w && h == full_h {
            result.push((Arc::new(ImageData::new(rgba.into_raw(), w, h)), delay));
        } else {
            // Composite onto full-size canvas (simplified: just use frame as-is)
            result.push((Arc::new(ImageData::new(rgba.into_raw(), w, h)), delay));
        }
    }

    Some(result)
}

// ── Animated image handle ────────────────────────────────────────

/// Handle for an animated image (e.g. GIF). Cycles through frames based on elapsed time.
/// Clone-safe (Rc-based internally via Arc).
#[derive(Clone)]
pub struct AnimatedImage {
    frames: Arc<Vec<(Arc<ImageData>, Duration)>>,
    start: Instant,
    total_duration: Duration,
}

impl AnimatedImage {
    pub fn new(frames: Vec<(Arc<ImageData>, Duration)>) -> Self {
        let total: Duration = frames.iter().map(|(_, d)| *d).sum();
        Self {
            frames: Arc::new(frames),
            start: Instant::now(),
            total_duration: total,
        }
    }

    /// Get the current frame based on elapsed time.
    pub fn current_frame(&self) -> &Arc<ImageData> {
        if self.frames.len() == 1 || self.total_duration.is_zero() {
            return &self.frames[0].0;
        }
        let elapsed = self.start.elapsed();
        let cycle_pos = elapsed.as_millis() % self.total_duration.as_millis();
        let mut acc = 0u128;
        for (data, dur) in self.frames.iter() {
            acc += dur.as_millis();
            if cycle_pos < acc {
                return data;
            }
        }
        &self.frames.last().unwrap().0
    }

    /// Whether this has more than one frame.
    pub fn is_animated(&self) -> bool {
        self.frames.len() > 1
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

// ── Image source enum ────────────────────────────────────────────

/// Source of image data for the Img element.
#[derive(Clone)]
pub enum ImageSource {
    /// Pre-decoded RGBA image data.
    Data(Arc<ImageData>),
    /// Animated image with multiple frames.
    Animated(AnimatedImage),
}

// ── Img element ──────────────────────────────────────────────────

pub struct Img {
    source: ImageSource,
    width: Option<f32>,
    height: Option<f32>,
    corner_radius: f32,
    opacity: f32,
    object_fit: ObjectFit,
}

/// Create an image element from pre-decoded image data.
pub fn img(data: Arc<ImageData>) -> Img {
    Img {
        source: ImageSource::Data(data),
        width: None,
        height: None,
        corner_radius: 0.0,
        opacity: 1.0,
        object_fit: ObjectFit::Cover,
    }
}

/// Create an image element from an animated image handle.
pub fn img_animated(anim: AnimatedImage) -> Img {
    Img {
        source: ImageSource::Animated(anim),
        width: None,
        height: None,
        corner_radius: 0.0,
        opacity: 1.0,
        object_fit: ObjectFit::Cover,
    }
}

impl Img {
    /// Set explicit width in logical pixels.
    pub fn w(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set explicit height in logical pixels.
    pub fn h(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set both width and height.
    pub fn size(self, width: f32, height: f32) -> Self {
        self.w(width).h(height)
    }

    /// Set corner radius for rounded image clipping.
    pub fn rounded(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Set opacity (0.0 = invisible, 1.0 = fully visible).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Set object-fit mode (Cover, Contain, Fill).
    pub fn object_fit(mut self, fit: ObjectFit) -> Self {
        self.object_fit = fit;
        self
    }

    /// Shorthand for ObjectFit::Contain.
    pub fn contain(mut self) -> Self {
        self.object_fit = ObjectFit::Contain;
        self
    }

    /// Shorthand for ObjectFit::Cover.
    pub fn cover(mut self) -> Self {
        self.object_fit = ObjectFit::Cover;
        self
    }

    /// Shorthand for ObjectFit::Fill.
    pub fn fill(mut self) -> Self {
        self.object_fit = ObjectFit::Fill;
        self
    }

    fn current_data(&self) -> &Arc<ImageData> {
        match &self.source {
            ImageSource::Data(d) => d,
            ImageSource::Animated(a) => a.current_frame(),
        }
    }
}

impl Element for Img {
    fn layout(&self, engine: &mut LayoutEngine, _font_system: &FontSystem) -> taffy::NodeId {
        let data = self.current_data();
        let w = self.width.unwrap_or(data.width as f32);
        let h = self.height.unwrap_or(data.height as f32);

        engine.new_leaf(Style {
            size: taffy::Size {
                width: length(w),
                height: length(h),
            },
            ..Default::default()
        })
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        _interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds = Rect::new(layout.x, layout.y, layout.width, layout.height);
        let data = self.current_data();

        draw_list.push(DrawCommand::Image {
            bounds,
            data: data.clone(),
            corner_radii: Corners::uniform(self.corner_radius),
            opacity: self.opacity,
            object_fit: self.object_fit,
        });
    }
}
