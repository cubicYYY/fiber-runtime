use std::{
    thread::{self, available_parallelism, JoinHandle},
    time::Duration,
};

use fiber_runtime::{executor::new_executor_and_spawner, timer_future::TimerFuture};

fn main() {
    let (executor, spawner) = new_executor_and_spawner(0);

    // Spawn a task to print before and after waiting on a timer.
    for i in 0..512 {
        spawner.spawn(async move {
            TimerFuture::new(Duration::from_secs(0)).await;
            println!("{} done by {:?}", i, thread::current().id());
        });
    }

    // No more jobs
    drop(spawner);

    let mut threads: Vec<JoinHandle<_>> = vec![];
    for _ in 0..available_parallelism().unwrap().get() {
        let cloned = executor.clone();
        threads.push(thread::spawn(move || {
            cloned.run(None);
            println!("Thread {:?} quits.", thread::current().id());
        }));
    }

    for handle in threads {
        handle.join().expect("Thread panicked!");
    }
}
