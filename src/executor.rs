use crossbeam_channel::{unbounded, Receiver, Sender};
use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use std::{
    cell::{RefCell, UnsafeCell},
    future::Future,
    marker::PhantomData,
    sync::{Arc, Mutex},
    task::Context,
    time::Duration,
};

use crate::timer_future::TimerFuture;

/// Either a boxed Future with dynamic typing, or a None acts as a terminator for a task queue.
///
/// WARNING: user should NEVER sync Task between threads:
///
/// this trait is added only to implement ArcWake trait.
/// The safety is ensured by lib developers as a library private/inner class and should never be exposed.
struct Task {
    /// If the Option = None, it indicates a terminate signal for the task queue.
    /// Pinned, since Task may be self-pointed.
    ///
    /// This is the only source that causes Task `!Sync`.
    /// We ensure the safety by only allow a future inside Task
    /// can only be unboxed by one specfic worker thread.
    future: RefCell<Option<BoxFuture<'static, ()>>>,

    /// Entrance to the queue
    loopback_entrance: Sender<Arc<Task>>,
}

/// SAFETY: We only access a task.future in a corresponding worker thread
unsafe impl Sync for Task {}

impl ArcWake for Task {
    /// Modern async runtimes(e.g. tokio) using wake ref to avoid heap allocation.
    /// So DO NOT use ::will_wake().
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Push self back to the task queue
        let cloned = arc_self.clone();
        arc_self
            .loopback_entrance
            .send(cloned)
            .expect("Task queue full!");
    }
}

/// Multiple comsumers
#[derive(Clone)]
pub struct Executor {
    task_queue: Receiver<Arc<Task>>,

    /// `!Send` and `!Sync`
    _marker: PhantomData<Receiver<()>>,
}

impl Executor {
    pub fn run(&self) {
        while let Ok(task) = self.task_queue.recv() {
            // WARNING: this blocks a thread using thread::park(), which ultimately interact with the OS,
            // like futex(fast userspace mutex) for Unix/FreeBSD/Android.
            // It is way faster than the condvar version implementation,
            // Since a short spin(exponential) will avoid most syscalls under intensive workload.

            let mut future_slot = task.future.borrow_mut(); // The only position where a future unboxing occured.
            if let Some(mut future) = future_slot.take() {
                // Store the waker for itself (i.e. a fancy callback to re-push itself into the task queue)
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);
                if future.as_mut().poll(context).is_pending() {
                    // If the future is not ready, replace the old future with the new one(i.e. next step).
                    // Remember: an "async" is just monads(with continuation insides),
                    // that is to perform a transform from a functor to another functor.
                    // E.g. a Future: (input)A->Result, with a single awaiting inside(accept type B)
                    // |---Functor old (Monad A): A->Monad B
                    // ↓
                    *future_slot = Some(future);
                    // ↓
                    // →---Functor new (Monad B): B->Result

                    // Each transform is (equivalent statements):
                    // A step in the finite-state machine bind to the Future;
                    // or a stage across an await(inside Future function body);
                    // or replace the old, finished Future inside a Task with the new Future to be executed(next step)
                }
            }
        }
    }
}

/// Multiple producers
#[derive(Clone)]
pub struct Spawner {
    queue_entrance: Sender<Arc<Task>>,

    /// `!Send` and `!Sync`
    _marker: PhantomData<Sender<()>>,
}

impl Spawner {
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: RefCell::new(Some(future)),
            loopback_entrance: self.queue_entrance.clone(),
        });
        self.queue_entrance.send(task).expect("Task queue full!");
    }
}

pub fn new_executor_and_spawner() -> (Executor, Spawner) {
    // MPMC executor and spawner, use it every where by .clone()
    let (task_sender, ready_queue) = unbounded();
    (
        Executor {
            task_queue: ready_queue,
            _marker: PhantomData,
        },
        Spawner {
            queue_entrance: task_sender,
            _marker: PhantomData,
        },
    )
}
