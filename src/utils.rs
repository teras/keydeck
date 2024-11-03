#[macro_export]
macro_rules! verbose_log {
    ($($arg:tt)*) => {
        if crate::DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! error_log {
    ($($arg:tt)*) => {
        eprintln!($($arg)*)
    };
}
#[macro_export]
macro_rules! info_log {
    ($($arg:tt)*) => {
        println!($($arg)*)
    };
}