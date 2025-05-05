#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let _ = $crate::log::Log::print(::core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        let _ = $crate::print!("{}\n", ::core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => {{
        let _ = $crate::log::Log::emergency_print(::core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! eprintln {
    ($($arg:tt)*) => {{
        let _ = $crate::eprint!("{}\n", ::core::format_args!($($arg)*));
    }};
}
