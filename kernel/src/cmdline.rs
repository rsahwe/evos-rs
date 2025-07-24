use core::sync::atomic::Ordering;

use crate::{config::{self, LogLevel}, debug, error, initramfs::InitRamFs, warn};

pub fn init() {
    let cmdline = match InitRamFs::open_text_file("cmdline") {
        Some(cmdline) => {
            match cmdline {
                Ok(cmdline) => cmdline,
                Err(error) => {
                    error!("'cmdline' contains invalid Utf8: {}", error);
                    error!("Ignoring cmdline..."); // TODO: CHANGE THIS WHEN CMDLINE CONTAINS IMPORTANT INFO
                    ""
                },
            }
        },
        None => {
            warn!("No 'cmdline' in initramfs");
            ""
        },
    };

    for assignment in cmdline.split("\n") {
        let assignment = assignment.trim();
        if assignment == "" {
            continue;
        }

        let mut pair = assignment.split("=");

        let key = pair.next().unwrap();

        let value = match pair.next() {
            Some(value) => value,
            None => {
                error!("Missing value for key `{}` in cmdline, ignoring...", key);
                continue;
            },
        };

        if pair.next().is_some() {
            error!("Too many '=' for key `{}` in cmdline, ignoring...", key);
            continue;
        }

        let key = key.trim();
        let value = value.trim();

        match key {
            "log_level" => {
                let new_log_level = match value.parse::<u8>() {
                    Ok(value) => {
                        match value {
                            0 => LogLevel::Critical,
                            1 => LogLevel::Error,
                            2 => LogLevel::Warn,
                            3 => LogLevel::Info,
                            4 => LogLevel::Debug,
                            _ => {
                                warn!("Invalid log_level, maximum is 4, using 4...");
                                LogLevel::Debug
                            }
                        }
                    },
                    Err(_) => {
                        error!("Invalid log_level, must be 0-4 for Critical-Debug, ignoring...");
                        continue;
                    },
                };

                debug!("Setting log_level to {:?}", new_log_level);
                config::LOG_LEVEL.store(new_log_level, Ordering::Relaxed);
                debug!("log_level is now {:?}", new_log_level);
            },
            _ => {
                error!("Unknown key `{}` in cmdline, ignoring...", key);
                continue;
            }
        }
    }
}