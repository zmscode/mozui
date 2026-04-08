use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(usize);

struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
}

/// Simple single-threaded executor for main-thread async tasks.
pub struct Executor {
    tasks: Vec<Option<Task>>,
    ready_queue: VecDeque<TaskId>,
    next_id: usize,
    // Shared queue for wakers to push task IDs back
    wake_queue: Arc<Mutex<VecDeque<TaskId>>>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            ready_queue: VecDeque::new(),
            next_id: 0,
            wake_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Spawn a future on the executor. It will be polled on the main thread.
    pub fn spawn(&mut self, future: impl Future<Output = ()> + 'static) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;

        let task = Task {
            future: Box::pin(future),
        };

        // Find a free slot or push
        if id.0 < self.tasks.len() {
            self.tasks[id.0] = Some(task);
        } else {
            self.tasks.push(Some(task));
        }

        self.ready_queue.push_back(id);
        id
    }

    /// Poll all ready tasks. Returns true if any task made progress.
    pub fn poll_ready(&mut self) -> bool {
        // Drain wake_queue into ready_queue
        {
            let mut wq = self.wake_queue.lock().unwrap();
            while let Some(id) = wq.pop_front() {
                self.ready_queue.push_back(id);
            }
        }

        let mut did_work = false;

        while let Some(id) = self.ready_queue.pop_front() {
            if let Some(Some(task)) = self.tasks.get_mut(id.0) {
                let waker = make_waker(id, self.wake_queue.clone());
                let mut cx = Context::from_waker(&waker);

                match task.future.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => {
                        self.tasks[id.0] = None;
                        did_work = true;
                    }
                    Poll::Pending => {
                        did_work = true;
                    }
                }
            }
        }

        did_work
    }

    /// Check if there are pending tasks.
    pub fn has_pending(&self) -> bool {
        self.tasks.iter().any(|t| t.is_some())
    }
}

// Simple waker implementation that pushes task ID to wake queue
fn make_waker(id: TaskId, wake_queue: Arc<Mutex<VecDeque<TaskId>>>) -> Waker {
    let data = Arc::new(WakerData { id, wake_queue });
    let raw = Arc::into_raw(data) as *const ();
    let vtable = &WAKER_VTABLE;
    unsafe { Waker::from_raw(RawWaker::new(raw, vtable)) }
}

struct WakerData {
    id: TaskId,
    wake_queue: Arc<Mutex<VecDeque<TaskId>>>,
}

const WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

unsafe fn waker_clone(data: *const ()) -> RawWaker {
    let arc = unsafe { Arc::from_raw(data as *const WakerData) };
    let cloned = arc.clone();
    std::mem::forget(arc);
    let ptr = Arc::into_raw(cloned) as *const ();
    RawWaker::new(ptr, &WAKER_VTABLE)
}

unsafe fn waker_wake(data: *const ()) {
    let arc = unsafe { Arc::from_raw(data as *const WakerData) };
    let mut queue = arc.wake_queue.lock().unwrap();
    queue.push_back(arc.id);
}

unsafe fn waker_wake_by_ref(data: *const ()) {
    let arc = unsafe { Arc::from_raw(data as *const WakerData) };
    {
        let mut queue = arc.wake_queue.lock().unwrap();
        queue.push_back(arc.id);
    }
    std::mem::forget(arc);
}

unsafe fn waker_drop(data: *const ()) {
    let _arc = unsafe { Arc::from_raw(data as *const WakerData) };
}
