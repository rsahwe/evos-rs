#[macro_export]
macro_rules! _print {
    ($($arg:tt)*) => {{
        let _ = $crate::log::Log::print(::core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! _println {
    ($($arg:tt)*) => {{
        let _ = $crate::_print!("{}\n", ::core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! print_init_msg {
    () => {{
        let _ = $crate::_println!("Evos v{}-{} build {} UTC", ::core::env!("CARGO_PKG_VERSION"), ::core::env!("EVOS_BUILD_ID"), ::compile_time::datetime_str!());
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        if $crate::config::LOG_LEVEL >= $crate::config::LogLevel::Error {
            let color = $crate::log::Log::swap_color(($crate::text::format::Color(255, 0, 0), $crate::text::format::Color(0, 0, 0)));
            let _ = $crate::_println!("ERROR: {}", ::core::format_args!($($arg)*));
            let _ = $crate::log::Log::swap_color(color);
        }
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        if $crate::config::LOG_LEVEL >= $crate::config::LogLevel::Warn {
            let color = $crate::log::Log::swap_color(($crate::text::format::Color(255, 255, 0), $crate::text::format::Color(0, 0, 0)));
            let _ = $crate::_println!("WARN : {}", ::core::format_args!($($arg)*));
            let _ = $crate::log::Log::swap_color(color);
        }
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        if $crate::config::LOG_LEVEL >= $crate::config::LogLevel::Info {
            let color = $crate::log::Log::swap_color(($crate::text::format::Color(0, 255, 0), $crate::text::format::Color(0, 0, 0)));
            let _ = $crate::_println!("INFO : {}", ::core::format_args!($($arg)*));
            let _ = $crate::log::Log::swap_color(color);
        }
    }};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        if $crate::config::LOG_LEVEL >= $crate::config::LogLevel::Debug {
            let color = $crate::log::Log::swap_color(($crate::text::format::Color(128, 128, 255), $crate::text::format::Color(0, 0, 0)));
            let _ = $crate::_println!("DEBUG: {}", ::core::format_args!($($arg)*));
            let _ = $crate::log::Log::swap_color(color);
        }
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
