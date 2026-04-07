use mozui_text::FontId;
use rustc_hash::FxHashMap;

/// Key for looking up a cached glyph in the atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_id: FontId,
    pub glyph_id: u32,
    pub size_px: u16, // Quantized font size in pixels
}

/// Region within the atlas texture.
#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

/// Simple shelf-packing texture atlas for glyph bitmaps.
pub struct TextureAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: u32,
    entries: FxHashMap<GlyphKey, AtlasRegion>,
    // Shelf allocator state
    shelf_y: u32,
    shelf_height: u32,
    cursor_x: u32,
    dirty: bool,
}

impl TextureAtlas {
    pub fn new(device: &wgpu::Device, size: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            size,
            entries: FxHashMap::default(),
            shelf_y: 0,
            shelf_height: 0,
            cursor_x: 0,
            dirty: false,
        }
    }

    /// Get a cached glyph region, or return None if not cached.
    pub fn get(&self, key: &GlyphKey) -> Option<&AtlasRegion> {
        self.entries.get(key)
    }

    /// Insert a glyph bitmap into the atlas. Returns the region.
    pub fn insert(
        &mut self,
        queue: &wgpu::Queue,
        key: GlyphKey,
        width: u32,
        height: u32,
        bearing_x: f32,
        bearing_y: f32,
        data: &[u8],
    ) -> Option<AtlasRegion> {
        if width == 0 || height == 0 {
            let region = AtlasRegion {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                bearing_x,
                bearing_y,
            };
            self.entries.insert(key, region);
            return Some(region);
        }

        // Check if glyph fits on current shelf
        if self.cursor_x + width > self.size {
            // Move to next shelf
            self.shelf_y += self.shelf_height;
            self.shelf_height = 0;
            self.cursor_x = 0;
        }

        if self.shelf_y + height > self.size {
            // Atlas is full
            tracing::warn!("Glyph atlas full ({}x{}), glyph dropped", self.size, self.size);
            return None;
        }

        // Allocate region
        let region = AtlasRegion {
            x: self.cursor_x,
            y: self.shelf_y,
            width,
            height,
            bearing_x,
            bearing_y,
        };

        // Upload bitmap
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.cursor_x += width + 1; // +1 pixel padding
        self.shelf_height = self.shelf_height.max(height + 1);
        self.entries.insert(key, region);
        self.dirty = true;

        Some(region)
    }
}
