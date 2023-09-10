use std::time::Duration;

use fiber_runtime::{executor::new_executor_and_spawner, timer_future::TimerFuture};
async fn demo() {
    TimerFuture::new(Duration::from_secs(5)).await;
    println!("Hello world");
}
fn main() {
    let (executor, spawner) = new_executor_and_spawner(0);

    // Spawn a task to print before and after waiting on a timer.
    for i in 0..4 {
        spawner.spawn(async move {
            println!("Task {}: will start after {} seconds.", i, 5 - i);
            // Wait for our timer future to complete after two seconds.
            TimerFuture::new(Duration::from_secs(5 - i)).await;
            println!("Task {} start (done after 5s)!", i);
            demo().await;
            println!("Task {} done!", i);
        });
    }

    // No more jobs
    drop(spawner);

    executor.run(None);
}
