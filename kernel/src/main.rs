#![no_std]
#![no_main]

use bootloader_api::BootInfo;
use evkrnl::{info, init, BOOT_CONFIG};

bootloader_api::entry_point!(kernel_main, config = &BOOT_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);

    info!("Initialization complete!");

    panic!("Kernel main exited!")
}
