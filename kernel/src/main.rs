#![no_std]
#![no_main]

use bootloader_api::BootInfo;
use evkrnl::{init, BOOT_CONFIG, syscall_test};

bootloader_api::entry_point!(kernel_main, config = &BOOT_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);

    syscall_test();

    panic!("Kernel main exited!")
}
