use crate::{
    GLOBAL_THREAD_TIMINGS, PlatformDispatcher, Priority, RunnableMeta, RunnableVariant, TaskTiming,
    ThreadTaskTimings, add_task_timing,
};
use async_task::Runnable;
use dispatch2::{DispatchQueue, DispatchQueueGlobalPriority, DispatchTime, GlobalQueueIdentifier};
use std::{
    ffi::c_void,
    ptr::NonNull,
    thread,
    time::{Duration, Instant},
};

/// Minimal dispatcher used by the iOS scaffold.
///
/// This intentionally uses plain threads for now. UIKit main-thread affinity and
/// real run loop integration will replace this in the lifecycle phase.
pub(crate) struct IosDispatcher;

impl IosDispatcher {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl PlatformDispatcher for IosDispatcher {
    fn get_all_timings(&self) -> Vec<ThreadTaskTimings> {
        let global_timings = GLOBAL_THREAD_TIMINGS.lock();
        ThreadTaskTimings::convert(&global_timings)
    }

    fn get_current_thread_timings(&self) -> ThreadTaskTimings {
        crate::profiler::get_current_thread_task_timings()
    }

    fn is_main_thread(&self) -> bool {
        unsafe { libc::pthread_main_np() != 0 }
    }

    fn dispatch(&self, runnable: RunnableVariant, priority: Priority) {
        let context = runnable.into_raw().as_ptr() as *mut c_void;

        let queue_priority = match priority {
            Priority::RealtimeAudio => {
                panic!("RealtimeAudio priority should use spawn_realtime, not dispatch")
            }
            Priority::High => DispatchQueueGlobalPriority::High,
            Priority::Medium => DispatchQueueGlobalPriority::Default,
            Priority::Low => DispatchQueueGlobalPriority::Low,
        };

        unsafe {
            DispatchQueue::global_queue(GlobalQueueIdentifier::Priority(queue_priority))
                .exec_async_f(context, trampoline);
        }
    }

    fn dispatch_on_main_thread(&self, runnable: RunnableVariant, _priority: Priority) {
        let context = runnable.into_raw().as_ptr() as *mut c_void;
        unsafe {
            DispatchQueue::main().exec_async_f(context, trampoline);
        }
    }

    fn dispatch_after(&self, duration: Duration, runnable: RunnableVariant) {
        let context = runnable.into_raw().as_ptr() as *mut c_void;
        let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::Priority(
            DispatchQueueGlobalPriority::High,
        ));
        let when = DispatchTime::NOW.time(duration.as_nanos() as i64);
        unsafe {
            DispatchQueue::exec_after_f(when, &queue, context, trampoline);
        }
    }

    fn spawn_realtime(&self, f: Box<dyn FnOnce() + Send>) {
        thread::spawn(move || f());
    }
}

extern "C" fn trampoline(context: *mut c_void) {
    let runnable =
        unsafe { Runnable::<RunnableMeta>::from_raw(NonNull::new_unchecked(context as *mut ())) };

    let location = runnable.metadata().location;
    let start = Instant::now();
    let mut timing = TaskTiming {
        location,
        start,
        end: None,
    };

    add_task_timing(timing);
    runnable.run();
    timing.end = Some(Instant::now());
    add_task_timing(timing);
}
