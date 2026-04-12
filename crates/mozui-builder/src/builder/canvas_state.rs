use mozui::{Pixels, Point, point, px};

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 5.0;

pub struct CanvasState {
    pub offset: Point<Pixels>,
    pub zoom: f32,
    pub interaction: InteractionMode,
    pub space_held: bool,
}

pub enum InteractionMode {
    None,
    Panning {
        start_offset: Point<Pixels>,
        start_mouse: Point<Pixels>,
    },
}

impl CanvasState {
    pub fn new() -> Self {
        Self {
            offset: point(px(0.0), px(0.0)),
            zoom: 1.0,
            interaction: InteractionMode::None,
            space_held: false,
        }
    }

    pub fn apply_pan_delta(&mut self, delta: Point<Pixels>) {
        self.offset.x += delta.x;
        self.offset.y += delta.y;
    }

    pub fn apply_zoom(&mut self, delta: f32, cursor_screen: Point<Pixels>) {
        let old_zoom = self.zoom;
        self.zoom = (self.zoom * (1.0 + delta)).clamp(MIN_ZOOM, MAX_ZOOM);
        let new_zoom = self.zoom;

        if (old_zoom - new_zoom).abs() > f32::EPSILON {
            // Keep the document point under the cursor fixed.
            // Content origin on screen = offset + (24, 24).
            // Document point under cursor = (cursor - offset - 24) / old_zoom.
            // After zoom, that point should still be at cursor, so:
            // new_offset = offset + (cursor - offset - 24) * (1 - new_zoom / old_zoom)
            let content_offset = px(24.0);
            let scale = 1.0 - new_zoom / old_zoom;
            self.offset.x += (cursor_screen.x - self.offset.x - content_offset) * scale;
            self.offset.y += (cursor_screen.y - self.offset.y - content_offset) * scale;
        }
    }

    pub fn is_panning(&self) -> bool {
        matches!(self.interaction, InteractionMode::Panning { .. })
    }
}
