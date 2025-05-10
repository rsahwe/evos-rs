#![no_std]
#![no_main]

use bootloader_api::BootInfo;
use evkrnl::{info, init, print_init_msg, BOOT_CONFIG};

bootloader_api::entry_point!(kernel_main, config = &BOOT_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);

    info!("Initialization complete!");

    print_init_msg!();
    evkrnl::syscall_test();

    panic!("Kernel main exited!")
}
