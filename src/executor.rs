use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use futures_util::future::Pending;
use std::{
    any::Any,
    cell::{RefCell, UnsafeCell},
    error::Error,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};

/// TODO: replace it with `impl Send` when Type Alias Impl Trait(TAIT) feature becomes stable
pub type SendableResultBox = Box<dyn Any + Send>;

/// Either a boxed Future with dynamic typing, or a None acts as a terminator for a task queue.
///
/// WARNING: user should NEVER sync Task between threads:
///
/// this trait is added only to implement ArcWake trait.
/// The safety is ensured by lib developers as a library private/inner class and should never be exposed.
pub struct Task {
    /// If the Option = None, it indicates a terminate signal for the task queue.
    /// Pinned, since Task may be self-pointed.
    ///
    /// This is the only source that causes Task `!Sync`.
    /// We ensure the safety by only allow a future inside Task
    /// can only be unboxed by one specfic worker thread.
    future: RefCell<Option<BoxFuture<'static, SendableResultBox>>>,

    // result: RefCell<Option<SendableResult>>,
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
        match arc_self.loopback_entrance.send(cloned) {
            Ok(_) => (),
            Err(_) => {
                println!("[DEBUG] Unfinished task dropped.");
            }
        }
    }
}

/// Multiple comsumers
#[derive(Clone)]
pub struct Executor {
    pub task_queue: Receiver<Arc<Task>>,
    /// `!Send` and `!Sync`
    _marker: PhantomData<Receiver<SendableResultBox>>,
}

impl Executor {
    /// `result_sender`: None if answer not needed
    pub fn run(&self, result_sender: Option<Sender<SendableResultBox>>) {
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
                match future.as_mut().poll(context) {
                    Poll::Pending => {
                        // If the future is not ready, replace the old future with the new one that will do next steps(i.e. continuation).
                        // You can call them "monads" (with continuation insides),
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
                    Poll::Ready(result) => {
                        println!("[DEBUG] Future ready");
                        if let Some(receiver) = result_sender.clone() {
                            receiver.send(result);
                        } else {
                            /// For debug only
                            println!("[DEBUG] Unused result: {:?}", result)
                        }
                    }
                }
            }
        }
    }

    pub fn run_once(&self) -> SendableResultBox {
        while let Ok(task) = self.task_queue.recv() {
            let mut future_slot = task.future.borrow_mut(); // The only position where a future unboxing occured.
            if let Some(mut future) = future_slot.take() {
                // Store the waker for itself (i.e. a fancy callback to re-push itself into the task queue)
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);
                match future.as_mut().poll(context) {
                    Poll::Pending => {
                        *future_slot = Some(future);
                    }
                    Poll::Ready(result) => {
                        return result;
                    }
                }
            }
        }
        panic!("No future to execute");
    }
}

/// Multiple producers
#[derive(Clone)]
pub struct Spawner<T> {
    queue_entrance: Sender<Arc<Task>>,

    /// `!Send` and `!Sync`
    _marker: PhantomData<Sender<T>>,
}

impl<T: Send + 'static> Spawner<T> {
    pub fn spawn(&self, future: impl Future<Output = T> + Send + 'static) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: RefCell::new(Some(Box::pin(
                future.map(|t| Box::new(t) as SendableResultBox),
            ))),
            loopback_entrance: self.queue_entrance.clone(),
            // result: RefCell::new(None),
        });
        self.queue_entrance.send(task).expect("Task queue full!");
    }
}

/// `capacity`: 0 for infinity
pub fn new_executor_and_spawner<T>(capacity: usize) -> (Executor, Spawner<T>) {
    // MPMC executor and spawner, use it every where by .clone()
    let (task_sender, task_receiver) = {
        if capacity == 0 {
            unbounded()
        } else {
            bounded(capacity)
        }
    };
    (
        Executor {
            task_queue: task_receiver,
            _marker: PhantomData,
        },
        Spawner {
            queue_entrance: task_sender,
            _marker: PhantomData,
        },
    )
}

pub fn block_on<T: Send + 'static>(future: impl Future<Output = T> + 'static + Send) -> T {
    let (ex, sp) = new_executor_and_spawner(1);
    sp.spawn(future);
    drop(sp); // Neccessary to close the channel, otherwise the executor will wait forever
    match ex.run_once().downcast::<T>() {
        Ok(res) => *res,
        Err(_) => panic!(),
    }
}
