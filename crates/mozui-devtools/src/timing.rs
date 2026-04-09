use std::collections::VecDeque;

/// Per-frame timing data collected from the render loop.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameTiming {
    /// Microseconds spent in the layout phase.
    pub layout_us: u64,
    /// Microseconds spent in the paint phase.
    pub paint_us: u64,
    /// Microseconds spent in GPU render.
    pub render_us: u64,
    /// Full frame time (layout + paint + render + overhead).
    pub total_us: u64,
    /// Number of draw commands emitted.
    pub draw_call_count: u32,
    /// Number of interactive elements (interaction entries).
    pub element_count: u32,
    /// Number of taffy layout nodes allocated.
    pub node_count: u32,
}

/// Ring buffer of recent frame timings.
pub struct FrameTimings {
    ring: VecDeque<FrameTiming>,
    capacity: usize,
}

impl FrameTimings {
    pub fn new(capacity: usize) -> Self {
        Self {
            ring: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a new frame timing, evicting the oldest if at capacity.
    pub fn push(&mut self, timing: FrameTiming) {
        if self.ring.len() >= self.capacity {
            self.ring.pop_front();
        }
        self.ring.push_back(timing);
    }

    /// Iterate over all stored timings (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &FrameTiming> {
        self.ring.iter()
    }

    /// Get the most recent frame timing.
    pub fn latest(&self) -> Option<&FrameTiming> {
        self.ring.back()
    }

    /// Average FPS over the stored frames.
    pub fn avg_fps(&self) -> f32 {
        if self.ring.is_empty() {
            return 0.0;
        }
        let total: u64 = self.ring.iter().map(|t| t.total_us).sum();
        if total == 0 {
            return 0.0;
        }
        let avg_frame_us = total as f64 / self.ring.len() as f64;
        (1_000_000.0 / avg_frame_us) as f32
    }

    /// Number of stored timings.
    pub fn len(&self) -> usize {
        self.ring.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }
}
