use core::fmt::{self, Arguments, Write};

use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::interrupts::without_interrupts;

const COM1: u16 = 0x3f8;

// SAFETY: COM1 IS VALID
static SERIAL: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(COM1) });

pub struct SerialPrinter {}

impl SerialPrinter {
    pub fn init() {
        // DEADLOCK SAFETY: ONLY USED HERE
        SERIAL.lock().init();
    }

    pub fn print(args: Arguments) -> fmt::Result {
        without_interrupts(|| {
            // AVOID DEADLOCK
            match SERIAL.try_lock() {
                Some(mut guard) => guard.write_fmt(args),
                None => Err(fmt::Error),
            }
        })
    }

    pub fn emergency_print(args: Arguments) -> fmt::Result {
        // SAFETY: ONLY USED IN EMERGENCY (IE PANIC OR SMTH)
        unsafe { SERIAL.force_unlock() };
        Self::print(args)
    }
}
