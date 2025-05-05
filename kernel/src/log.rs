use core::fmt::{self, Arguments};

use bootloader_api::info::{FrameBuffer, Optional};

use crate::{framebuffer::FramePrinter, println, serial::SerialPrinter};

pub fn init(framebuffer: &'static mut Optional<FrameBuffer>) {
    SerialPrinter::init();

    if let Optional::Some(fb) = framebuffer {
        FramePrinter::set_default_static(fb);
        println!("DEBUG: Framebuffer initialized");
    }
}

pub struct Log {}

impl Log {
    pub fn print(args: Arguments) -> fmt::Result {
        SerialPrinter::print(args)?;
        FramePrinter::print_default_static(args)
    }

    pub fn emergency_print(args: Arguments) -> fmt::Result {
        SerialPrinter::emergency_print(args)?;
        FramePrinter::emergency_print_default_static(args)
    }
}
