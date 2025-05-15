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
macro_rules! error {
    ($($arg:tt)*) => {{
        let color = $crate::log::Log::swap_color(($crate::text::format::Color(255, 0, 0), $crate::text::format::Color(0, 0, 0)));
        let _ = $crate::println!("ERROR: {}", ::core::format_args!($($arg)*));
        let _ = $crate::log::Log::swap_color(color);
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        let color = $crate::log::Log::swap_color(($crate::text::format::Color(255, 255, 0), $crate::text::format::Color(0, 0, 0)));
        let _ = $crate::println!("WARN : {}", ::core::format_args!($($arg)*));
        let _ = $crate::log::Log::swap_color(color);
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        let color = $crate::log::Log::swap_color(($crate::text::format::Color(0, 255, 0), $crate::text::format::Color(0, 0, 0)));
        let _ = $crate::println!("INFO : {}", ::core::format_args!($($arg)*));
        let _ = $crate::log::Log::swap_color(color);
    }};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        let color = $crate::log::Log::swap_color(($crate::text::format::Color(128, 128, 255), $crate::text::format::Color(0, 0, 0)));
        let _ = $crate::println!("DEBUG: {}", ::core::format_args!($($arg)*));
        let _ = $crate::log::Log::swap_color(color);
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
