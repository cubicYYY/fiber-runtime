//! Demonstrates `spawn_with_handle` -- spawning tasks that return values.
//!
//! Run: cargo run --example join_handle

use fiber_runtime::executor::new_executor_and_spawner;
use fiber_runtime::timer_future::TimerFuture;
use std::time::Duration;

fn main() {
    let (executor, spawner) = new_executor_and_spawner(0);

    let sp = spawner.clone();
    spawner.spawn(async move {
        // Spawn two compute tasks and await their results.
        let h1 = sp.spawn_with_handle(async {
            TimerFuture::new(Duration::from_secs(1)).await;
            println!("task 1 done");
            10
        });
        let h2 = sp.spawn_with_handle(async {
            TimerFuture::new(Duration::from_secs(2)).await;
            println!("task 2 done");
            32
        });

        // Both tasks run concurrently. We await their results here.
        let sum = h1.await + h2.await;
        println!("sum = {sum}");
        assert_eq!(sum, 42);
    });

    drop(spawner); // drop the original; `sp` was moved into the async block
    executor.run();
}
