use std::{
    thread::{self, available_parallelism},
    time::Instant,
};

use fiber_runtime::executor::new_executor_and_spawner;
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
    println!("===SINGLE===");
    let start_time = Instant::now();
    for _ in 0..3 {
        // 3 jobs
        cpu_bounded_job();
    }
    println!(
        "Single thread time per job: {}ms",
        start_time.elapsed().as_millis() / 3
    );

    println!("============");
    println!(
        "===MULTI===[parallelism={}]",
        available_parallelism().unwrap().get()
    );
    let (executor, spawner) = new_executor_and_spawner(0);

    let start_time = Instant::now();
    for i in 0..available_parallelism().unwrap().get() * 3 {
        // each thread should get 3 jobs
        spawner.spawn(async move {
            println!("Task {} started by {:?}.", i, thread::current().id());
            cpu_bounded_job();
            println!("Task {} done!", i);
        });
    }

    // No more jobs
    drop(spawner);

    let mut threads = vec![];
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
    println!(
        "Multi-thread async time per job: {}ms",
        start_time.elapsed().as_millis() / available_parallelism().unwrap().get() as u128 / 3
    );
    println!("======");
}
