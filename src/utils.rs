#[macro_export]
macro_rules! verbose_log {
    ($($arg:tt)*) => {
        if crate::DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            println!($($arg)*);
        }
    };
}