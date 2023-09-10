#[macro_export]
macro_rules! fiber_main {
    ($($body:tt)*) => {
        fn main() {
            fiber_runtime::block_on(async move { $($body)* });
        }
    };
}
