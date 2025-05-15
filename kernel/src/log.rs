use core::fmt::{self, Arguments};

use bootloader_api::info::{FrameBuffer, Optional};
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{framebuffer::FramePrinter, debug, serial::SerialPrinter, text::format::Color};

pub fn init(framebuffer: &'static mut Optional<FrameBuffer>) {
    SerialPrinter::init();

    if let Optional::Some(fb) = framebuffer {
        FramePrinter::set_default_static(fb);
        debug!("Framebuffer initialized");
    }
}

pub struct Log {}

static COLORS: Mutex<(Color, Color)> = Mutex::new((Color(255, 255, 255), Color(0, 0, 0)));

impl Log {
    pub fn print(args: Arguments) -> fmt::Result {
        SerialPrinter::print(args)?;
        FramePrinter::print_default_static(args)
    }

    pub fn emergency_print(args: Arguments) -> fmt::Result {
        // SAFETY: EMERGENCY (AND HOPEFULLY NO PROBLEM)
        unsafe { COLORS.force_unlock() };
        let old = Self::swap_color((Color(255, 255, 255), Color(255, 0, 0)));
        SerialPrinter::emergency_print(args)?;
        FramePrinter::emergency_print_default_static(args)?;
        // SAFETY: EMERGENCY (AND HOPEFULLY NO PROBLEM)
        unsafe { COLORS.force_unlock() };
        let _ = Self::swap_color(old);

        Ok(())
    }

    pub fn swap_color(colors: (Color, Color)) -> (Color, Color) {
        without_interrupts(|| {
            let mut colors_guard = COLORS.lock();

            let old = *colors_guard;
    
            *colors_guard = colors;

            FramePrinter::set_default_static_colors(colors.0, colors.1);
    
            old
        })
    }
}
