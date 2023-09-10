use std::thread;

use fiber_runtime::executor::new_executor_and_spawner;

fn main() {
    // Check memory usage
    let (_executor, spawner) = new_executor_and_spawner(0);
    // Spawn a task to print before and after waiting on a timer.
    for _ in 0..100_000_00 { // 1.33GB on Ubuntu, 143 Bytes per task
        spawner.spawn(async move {});
    }

    // No more jobs
    drop(spawner);
    println!("Gen finished");
    // executor.run(None);
    loop {
        thread::yield_now();
    }
}
