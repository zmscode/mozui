#![forbid(unsafe_code)]

mod signal;

pub use signal::{MutationCallback, SetSignal, Signal, SignalStore};
