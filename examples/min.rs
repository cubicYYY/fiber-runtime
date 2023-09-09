use fiber_runtime::executor::new_executor_and_spawner;
async fn demo() {
    println!("Hello world");
}
fn main() {
    // Get executor and spawner, you can clone them and use it in multiple threads
    let (executor, spawner) = new_executor_and_spawner();

    // 
    for i in 0..4 {
        spawner.spawn(async move {
            demo().await;
            println!("Task {} done!", i);
        });        
    }

    // No more jobs
    drop(spawner);

    // Block current threadm and start listening on the task queue
    executor.run();
}
