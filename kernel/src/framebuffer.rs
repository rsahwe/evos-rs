use core::{ascii::Char, fmt::{Arguments, Write}};

use bootloader_api::info::{FrameBuffer, FrameBufferInfo};
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{text::{font::Font, format::Color}, time::Time};

type FramePrinterFont = crate::config::framebuffer::Font;

pub struct FramePrinter {
    framebuffer: &'static mut FrameBuffer,
    info: FrameBufferInfo,
    line_count: usize,
    line_pos: usize,
    fg_color: Color,
    bg_color: Color,
    newline: bool,
}

static FRAMEBUFFER: Mutex<Option<FramePrinter>> = Mutex::new(None);

impl FramePrinter {
    pub fn set_default_static(framebuffer: &'static mut FrameBuffer) {
        // DEADLOCK SAFETY: ONLY USED ONCE BEFORE ANY PRINTS
        let mut framebuffer_guard = FRAMEBUFFER.lock();

        *framebuffer_guard = Some(FramePrinter {
            info: framebuffer.info(),
            framebuffer,
            line_count: 0,
            line_pos: 0,
            newline: true,
            fg_color: Color(255, 255, 255),
            bg_color: Color(0, 0, 0),
        });

        framebuffer_guard.as_mut().unwrap().framebuffer.buffer_mut().fill(0);

        drop(framebuffer_guard);
    }

    pub fn print_default_static(args: Arguments) -> core::fmt::Result {
        without_interrupts(|| {
            // AVOID DEADLOCK
            match FRAMEBUFFER.try_lock() {
                Some(mut guard) => match *guard {
                    Some(ref mut fb) => fb.write_fmt(args),
                    // A missing frame printer is ok
                    None => Ok(()),
                },
                // Screen printing is commonly needed so avoid deadlock
                None => Err(core::fmt::Error),
            }
        })
    }

    pub fn emergency_print_default_static(args: Arguments) -> core::fmt::Result {
        // SAFETY: ONLY USED IN EMERGENCY (IE PANIC OR SMTH)
        unsafe { FRAMEBUFFER.force_unlock() };
        Self::print_default_static(args)
    }
}

impl FramePrinter {
    fn set_color_at(&mut self, x: usize, y: usize, col: Color) -> core::fmt::Result {
        let base_pos = ((self.info.height - FramePrinterFont::height() + y) * self.info.stride + (self.line_pos * FramePrinterFont::width() + x)) * self.info.bytes_per_pixel;
        let buffer = self.framebuffer.buffer_mut();
        match self.info.pixel_format {
            bootloader_api::info::PixelFormat::Rgb => {
                buffer[base_pos + 0] = col.0;
                buffer[base_pos + 1] = col.1;
                buffer[base_pos + 2] = col.2;
                Ok(())
            },
            bootloader_api::info::PixelFormat::Bgr => {
                buffer[base_pos + 0] = col.2;
                buffer[base_pos + 1] = col.1;
                buffer[base_pos + 2] = col.0;
                Ok(())
            },
            bootloader_api::info::PixelFormat::U8 => {
                buffer[base_pos] = ((col.0 as u16 * 21 + col.1 as u16 * 72 + col.2 as u16 * 7) / 100) as u8;
                Ok(())
            },
            bootloader_api::info::PixelFormat::Unknown { red_position, green_position, blue_position } => {
                buffer[base_pos + red_position as usize]    = col.0;
                buffer[base_pos + green_position as usize]  = col.1;
                buffer[base_pos + blue_position as usize]   = col.2;
                Ok(())
            },
            _ => Err(core::fmt::Error),
        }
    }
}

impl Write for FramePrinter {
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        if self.newline {
            self.newline = false;
            let time = Time::boot_time_ns();
            self.write_fmt(format_args!("[{:03}.{:03}] ", (time / 1000000000) % 1000, (time / 1000000) % 1000))?;
        }

        let c = c.as_ascii().unwrap_or(Char::EndOfTransmission /* SQUARE */);
        match c {
            Char::LineFeed => {
                self.framebuffer.buffer_mut().copy_within(self.info.stride * self.info.bytes_per_pixel * FramePrinterFont::height().., 0);
                self.framebuffer.buffer_mut().split_at_mut((self.info.height - FramePrinterFont::height()) * self.info.stride * self.info.bytes_per_pixel).1.fill(0);
                self.line_pos = 0;
                self.newline = true;
                self.line_count += 1;
                Ok(())
            },
            Char::CarriageReturn => {
                self.line_pos = 10;
                Ok(())
            },
            //TODO: ANSI OR SMTH FOR COLORS
            _ => {
                let c = FramePrinterFont::get_char(c);
                if self.line_pos == self.info.width / FramePrinterFont::width() {
                    write!(self, "\n\r")?;
                }
                for y in 0..FramePrinterFont::height() {
                    for x in 0..FramePrinterFont::width() {
                        let select = c[y * FramePrinterFont::width() + (FramePrinterFont::width() - x - 1)];
                        self.set_color_at(x, y, if select { self.fg_color } else { self.bg_color })?;
                    }
                }
                self.line_pos += 1;
                Ok(())
            }
        }
    }
    
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        };

        Ok(())
    }
}
