use std::time::Duration;

use fiber_runtime::{block_on, fiber_main, timer_future::TimerFuture};
use futures::{join, select};
use futures_util::FutureExt;

async fn demo() {
    TimerFuture::new(Duration::from_secs(5)).await;
    println!("Hello world");
}

// async main
fiber_main! {
    // join! will combined them to 1 future, with less overhead.
    block_on(async move {
        join!(
            async move {
                println!("Task1 start (done after 5s)!");
                demo().await;
                println!("Task1 done!");
            },
            async move {
                println!("Task2 start (done after 5s)!");
                demo().await;
                println!("Task2 done!");
            }
        )
    });

    // select! racing result. You will see the unfinished 
    for _ in 0..32{
        let launch1 = async {
        TimerFuture::new(Duration::from_secs(1)).await;
        };
        let launch2 = async {
            TimerFuture::new(Duration::from_secs(1)).await;
        };
        let res = block_on(async move {
            select! {
                _x = launch1.fuse() => 114514,
                _y = launch2.fuse() => 1919810
            }
        });
        println!("res= {}", res);
    }
}
