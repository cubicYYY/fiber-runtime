use std::time::Instant;

use fiber_runtime::executor::block_on;
use sha1::Sha1;
use tokio::spawn;

#[tokio::main]
async fn main() {
    const TASK_NUM: u32 = 32768;
    println!("===MY IMPL===");
    let start_time = Instant::now();
    // Spawn a task to print before and after waiting on a timer.
    for _ in 0..TASK_NUM {
        block_on(async move {
            let data = "Hello, World!";
            let bytes = data.as_bytes();
            let hasher = Sha1::from(bytes);
            let _ = Sha1::from(hasher.digest().bytes());
        });
    }

    println!(
        "===Time: {}ms ===",
        start_time.elapsed().as_millis()
    );

    println!("===TOKIO===");
    let start_time = Instant::now();
    for _ in 0..TASK_NUM {
        let _ = spawn(async move {
            let data = "Hello, World!";
            let bytes = data.as_bytes();
            let hasher = Sha1::from(bytes);
            let _ = Sha1::from(hasher.digest().bytes());
        }).await;
    }
    println!(
        "===Time: {}ms ===",
        start_time.elapsed().as_millis()
    );
}
