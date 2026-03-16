use fiber_runtime::executor::new_executor_and_spawner;

fn main() {
    // Check memory usage
    let (_executor, spawner) = new_executor_and_spawner(0);
    for _ in 0..10_000_000 {
        // ~1.33GB on Ubuntu, ~143 bytes per task
        spawner.spawn(async move {});
    }

    // No more jobs
    drop(spawner);
    println!("Gen finished. Press Enter to exit...");
    let _ = std::io::stdin().read_line(&mut String::new());
}
