use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use std::{
    future::Future,
    sync::{Arc, Mutex},
    task::Context,
};

/// A task wrapping a boxed future that can be re-enqueued via its waker.
///
/// All tasks produce `()`. To retrieve a typed result from `block_on`,
/// the future is wrapped in an adapter that sends the value through a channel.
struct Task {
    /// `None` after the future completes or is taken for polling.
    future: Mutex<Option<BoxFuture<'static, ()>>>,

    /// Cloned sender used by the waker to re-enqueue this task.
    loopback_entrance: Sender<Arc<Task>>,
}

impl ArcWake for Task {
    /// Modern async runtimes (e.g. tokio) use waker refs to avoid heap allocation,
    /// so DO NOT rely on `Waker::will_wake()`.
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        // Send errors occur during normal shutdown when the receiver is dropped.
        let _ = arc_self.loopback_entrance.send(cloned);
    }
}

/// Pulls tasks from a shared queue and polls them to completion.
///
/// Multiple `Executor` clones can run on different threads (MPMC pattern).
#[derive(Clone)]
pub struct Executor {
    task_queue: Receiver<Arc<Task>>,
}

impl Executor {
    /// Run the executor loop until all senders (spawners + in-flight task wakers) are dropped.
    ///
    /// This blocks the calling thread. The channel's `recv()` spins briefly before
    /// falling back to `thread::park()`, avoiding expensive syscalls under load.
    pub fn run(&self) {
        while let Ok(task) = self.task_queue.recv() {
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);
                // If still pending, put the continuation back for the next wake.
                //
                // Each poll may advance the future's internal state machine by one step
                // (i.e. across one `.await` point). The waker re-enqueues the task
                // when the awaited resource becomes ready.
                if future.as_mut().poll(context).is_pending() {
                    *future_slot = Some(future);
                }
            }
        }
    }
}

/// Submits futures as tasks into the shared queue.
///
/// Multiple `Spawner` clones can submit from different threads (MPMC pattern).
#[derive(Clone)]
pub struct Spawner {
    queue_entrance: Sender<Arc<Task>>,
}

impl Spawner {
    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let task = Arc::new(Task {
            future: Mutex::new(Some(future.boxed())),
            loopback_entrance: self.queue_entrance.clone(),
        });
        self.queue_entrance
            .send(task)
            .expect("Executor has been dropped");
    }
}

/// Create a paired executor and spawner.
///
/// `capacity`: 0 for an unbounded queue.
pub fn new_executor_and_spawner(capacity: usize) -> (Executor, Spawner) {
    let (task_sender, task_receiver) = if capacity == 0 {
        unbounded()
    } else {
        bounded(capacity)
    };
    (
        Executor {
            task_queue: task_receiver,
        },
        Spawner {
            queue_entrance: task_sender,
        },
    )
}

/// Run a future to completion on a fresh single-task executor, returning its result.
///
/// A typed crossbeam channel is used to shuttle the result back, avoiding
/// `Box<dyn Any>` and `downcast`.
pub fn block_on<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> T {
    let (result_tx, result_rx) = bounded(1);
    let (ex, sp) = new_executor_and_spawner(1);
    sp.spawn(async move {
        let result = future.await;
        let _ = result_tx.send(result);
    });
    drop(sp); // Necessary to close the channel, otherwise the executor blocks forever
    ex.run();
    result_rx.recv().expect("Future did not produce a result")
}
