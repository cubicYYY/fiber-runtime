use std::{
    thread::{self, available_parallelism, JoinHandle},
    time::Duration,
};

use fiber_runtime::{executor::new_executor_and_spawner, timer_future::TimerFuture};
use sha1::Sha1;

fn cpu_bounded_job() {
    let data = "Hello, World!";
    let bytes = data.as_bytes();

    let mut hasher = Sha1::from(bytes);
    for _ in 0..1000000 {
        hasher = Sha1::from(hasher.digest().bytes());
    }
    println!("SHA-1 hash: {}", hasher.digest());
}
fn main() {
    let (executor, spawner) = new_executor_and_spawner();

    // Spawn a task to print before and after waiting on a timer.
    for i in 0..4 {
        let cloned = spawner.clone();
        cloned.spawn(async move {
            println!("Task {}: will start after 1 seconds.", i);
            // Wait for our timer future to complete after two seconds.
            TimerFuture::new(Duration::from_secs(1)).await;
            println!("Task {} started by {:?}.", i, thread::current().id());
            cpu_bounded_job();
            println!("Task {} done!", i);
        });
    }

    // No more jobs
    drop(spawner);

    let mut threads: Vec<JoinHandle<_>> = vec![];
    for _ in 0..available_parallelism().unwrap().get() {
        let cloned = executor.clone();
        threads.push(thread::spawn(move || {
            cloned.run();
            println!("Thread {:?} quits.", thread::current().id());
        }));
    }

    for handle in threads {
        handle.join().expect("Thread panicked!");
    }
}
