pub mod inspector;
pub mod signals;
pub mod timing;

pub use inspector::{ElementInfo, ElementTreeEntry, InspectorState};
pub use signals::{SignalLog, SignalMutation, SignalSnapshot};
pub use timing::{FrameTiming, FrameTimings};

/// ~4 seconds at 60fps.
const DEFAULT_TIMING_CAPACITY: usize = 240;

/// Default capacity for the signal mutation ring buffer.
const DEFAULT_SIGNAL_LOG_CAPACITY: usize = 500;

/// Central devtools state, stored per-window.
pub struct DevtoolsState {
    pub perf_overlay_active: bool,
    pub signal_debugger_active: bool,
    pub inspector_active: bool,
    pub timings: FrameTimings,
    pub signal_log: SignalLog,
    pub inspector: InspectorState,
}

impl DevtoolsState {
    pub fn new() -> Self {
        Self {
            perf_overlay_active: false,
            signal_debugger_active: false,
            inspector_active: false,
            timings: FrameTimings::new(DEFAULT_TIMING_CAPACITY),
            signal_log: SignalLog::new(DEFAULT_SIGNAL_LOG_CAPACITY),
            inspector: InspectorState {
                tree: Vec::new(),
                hovered: None,
                selected: None,
            },
        }
    }

    /// Returns true if any devtools panel is active.
    pub fn any_active(&self) -> bool {
        self.perf_overlay_active || self.signal_debugger_active || self.inspector_active
    }
}
