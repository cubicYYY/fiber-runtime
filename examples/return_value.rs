use fiber_runtime::executor::new_executor_and_spawner;
async fn demo(i: u32) -> u32{
    println!("Hello world");
    i + 42
}
fn main() {
    // Get executor and spawner, you can clone them and use it in multiple threads
    let (executor, spawner) = new_executor_and_spawner();

    // 
    for i in 0..4 {
        spawner.spawn(async move {
            let a = demo(i).await;
            println!("{}", a);
            a * 2
        });        
    }

    // No more jobs
    drop(spawner);

    // Block current threadm and start listening on the task queue
    executor.run();
    println!("{:?}", executor.task_queue.len())
}
