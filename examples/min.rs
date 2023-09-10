use fiber_runtime::fiber_main;

async fn demo() {
    println!("Hello world");
}
fiber_main! {
    let universe = async {
        demo().await;
        println!("done demo");
        42
    }.await;
    assert_eq!(universe, 42);
}
