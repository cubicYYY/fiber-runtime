use fiber_runtime::executor::block_on;

async fn demo() {
    println!("Hello world");
}
fn main() {
    let universe = block_on(async {
        demo().await;
        println!("done demo");
        42
    });
    assert_eq!(universe, 42);
}
