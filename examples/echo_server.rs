//! TCP echo server — demonstrates the reactor-driven async I/O pipeline.
//!
//! Run:  cargo run --example echo_server
//! Test: nc 127.0.0.1 8080       (then type lines, they are echoed back)
//!   or: echo "hello" | nc 127.0.0.1 8080

use fiber_runtime::{
    executor::new_executor_and_spawner,
    tcp::TcpListener,
};

fn main() {
    let (executor, spawner) = new_executor_and_spawner(0);

    // Spawn the accept loop as a task.
    let sp = spawner.clone();
    spawner.spawn(async move {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let listener = TcpListener::bind(addr).unwrap();
        println!("Echo server listening on {addr}");
        println!("Test with: nc 127.0.0.1 8080");

        loop {
            let (stream, peer) = listener.accept().await.unwrap();
            println!("[{peer}] connected");

            // Spawn a new task for this connection.
            // All connection tasks run concurrently on the single executor
            // thread — the reactor wakes only the tasks whose sockets are ready.
            sp.spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    let n = match stream.read(&mut buf).await {
                        Ok(0) | Err(_) => break, // EOF or error
                        Ok(n) => n,
                    };
                    if stream.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                println!("[{peer}] disconnected");
            });
        }
    });

    drop(spawner);
    executor.run();
}
