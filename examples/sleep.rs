use std::time::Duration;

use fiber_runtime::{executor::new_executor_and_spawner, timer_future::TimerFuture};

fn main() {
    let (executor, spawner) = new_executor_and_spawner(0);

    for i in 0..4 {
        spawner.spawn(async move {
            println!("Task {i}: waiting {} seconds...", 5 - i);
            TimerFuture::new(Duration::from_secs(5 - i)).await;
            println!("Task {i}: done!");
        });
    }

    drop(spawner);
    executor.run();
}
