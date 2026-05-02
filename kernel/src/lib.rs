#![no_std]
#![feature(ascii_char)]
#![feature(ascii_char_variants)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(abi_x86_interrupt)]
#![allow(clippy::identity_op)]

use bootloader_api::BootInfo;

pub mod cmdline;
pub mod config;
pub(crate) mod descriptors;
pub mod ffi;
pub(crate) mod framebuffer;
pub mod future;
pub mod initramfs;
pub(crate) mod interrupts;
pub mod log;
pub mod macros;
pub(crate) mod mem;
pub mod modules;
mod panic;
pub mod pci;
pub(crate) mod serial;
pub(crate) mod syscalls;
pub mod text;
pub mod time;

pub use mem::CONFIG as BOOT_CONFIG;

pub fn init(boot_info: &'static mut BootInfo) {
    log::init(&mut boot_info.framebuffer);
    info!("Logging initialized");
    initramfs::init(
        boot_info
            .ramdisk_addr
            .into_option()
            .expect("Ramdisk missing!!!"),
        boot_info.ramdisk_len,
    );
    info!(
        "InitRamFs initialized with {} files",
        initramfs::InitRamFs::iter().len()
    );
    cmdline::init();
    info!("Loaded kernel parameters");
    descriptors::init();
    info!("GDT & TSS initialized");
    interrupts::init();
    info!("IDT initialized");
    // SAFETY: MEMORY REGIONS ARE VALID AND LATER UNUSED
    unsafe { mem::init(&mut boot_info.memory_regions) };
    info!("Memory initialized");
    syscalls::init();
    info!("SYSCALLS initialized");
    let devices = pci::init();
    info!("Pci initialized with {} devices", devices);
    let (successful, total) = modules::init();
    info!("Modules initialized ({}/{})", successful, total);
    info!("Initialization complete!");
    print_init_msg!();
}
