#![no_std]
#![feature(ascii_char)]
#![feature(ascii_char_variants)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(abi_x86_interrupt)]

use bootloader_api::BootInfo;

pub mod text;
pub(crate) mod framebuffer;
pub(crate) mod serial;
pub mod macros;
pub mod config;
pub(crate) mod interrupts;
pub(crate) mod descriptors;
pub(crate) mod mem;
mod panic;
pub mod log;
pub mod time;

pub use mem::CONFIG as BOOT_CONFIG;

pub fn init(boot_info: &'static mut BootInfo) {
    log::init(&mut boot_info.framebuffer);
    println!("INFO: Logging initialized");
    descriptors::init();
    println!("INFO: GDT & TSS initialized");
    interrupts::init();
    println!("INFO: IDT initialized");
    // SAFETY: MEMORY REGIONS ARE VALID AND LATER UNUSED
    unsafe { mem::init(&mut boot_info.memory_regions) };
}
