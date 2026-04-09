pub mod timing;

pub use timing::{FrameTiming, FrameTimings};

/// ~4 seconds at 60fps.
const DEFAULT_TIMING_CAPACITY: usize = 240;

/// Central devtools state, stored per-window.
pub struct DevtoolsState {
    pub perf_overlay_active: bool,
    pub signal_debugger_active: bool,
    pub inspector_active: bool,
    pub timings: FrameTimings,
}

impl DevtoolsState {
    pub fn new() -> Self {
        Self {
            perf_overlay_active: false,
            signal_debugger_active: false,
            inspector_active: false,
            timings: FrameTimings::new(DEFAULT_TIMING_CAPACITY),
        }
    }

    /// Returns true if any devtools panel is active.
    pub fn any_active(&self) -> bool {
        self.perf_overlay_active || self.signal_debugger_active || self.inspector_active
    }
}
