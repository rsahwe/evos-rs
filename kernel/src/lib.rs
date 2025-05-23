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
pub(crate) mod syscalls;
pub mod modules;
pub mod initramfs;
pub mod ffi;

pub use mem::CONFIG as BOOT_CONFIG;

pub fn init(boot_info: &'static mut BootInfo) {
    log::init(&mut boot_info.framebuffer);
    info!("Logging initialized");
    initramfs::init(boot_info.ramdisk_addr.into_option().expect("Ramdisk missing!!!"), boot_info.ramdisk_len);
    info!("InitRamFs initialized with {} files", initramfs::InitRamFs::iter().len());
    descriptors::init();
    info!("GDT & TSS initialized");
    interrupts::init();
    info!("IDT initialized");
    // SAFETY: MEMORY REGIONS ARE VALID AND LATER UNUSED
    unsafe { mem::init(&mut boot_info.memory_regions) };
    syscalls::init();
    info!("SYSCALLS initialized");
    let (successful, total) = modules::init();
    info!("Modules initialized ({}/{})", successful, total);
    info!("Initialization complete!");
    print_init_msg!();
}
