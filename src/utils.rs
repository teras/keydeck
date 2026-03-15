// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

#[macro_export]
macro_rules! timestamp {
    () => {
        chrono::Local::now().format("%H:%M:%S%.3f")
    };
}

#[macro_export]
macro_rules! verbose_log {
    ($($arg:tt)*) => {
        if crate::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed) >= 2 {
            println!("[{}] {}", crate::timestamp!(), format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! detail_log {
    ($($arg:tt)*) => {
        if crate::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed) >= 1 {
            println!("[{}] {}", crate::timestamp!(), format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! error_log {
    ($($arg:tt)*) => {
        eprintln!("[{}] ERROR: {}", crate::timestamp!(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warn_log {
    ($($arg:tt)*) => {
        eprintln!("[{}] WARNING: {}", crate::timestamp!(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! info_log {
    ($($arg:tt)*) => {
        println!("[{}] {}", crate::timestamp!(), format!($($arg)*))
    };
}
