use fiber_runtime::executor::block_on;
async fn demo(i: u32) -> u32 {
    println!("Hello world");
    i + 42
}
fn main() {
    for i in 0..4 {
        let res = block_on(async move {
            let a = demo(i).await;
            println!("{}", a);
            a * 2
        });
        println!("res= {}", res);
    }
    println!("Bye");
}
