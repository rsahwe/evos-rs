use core::{panic::PanicInfo, sync::atomic::{AtomicBool, Ordering}};

use x86_64::instructions::{hlt, interrupts::disable};

use crate::eprintln;

static HAS_PANICKED: AtomicBool = AtomicBool::new(false);
static HAS_PANICKED_AGAIN: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn kernel_panic(panic_info: &PanicInfo) -> ! {
    disable();

    if HAS_PANICKED.load(Ordering::Relaxed) {
        if HAS_PANICKED_AGAIN.load(Ordering::Relaxed) {
            loop {
                hlt();
            }
        }

        HAS_PANICKED_AGAIN.store(true, Ordering::Relaxed);

        eprintln!("\nDOUBLE PANIC!!!\n{}", panic_info);

        loop {
            hlt();
        }
    }

    HAS_PANICKED.store(true, Ordering::Relaxed);

    eprintln!("\n{}", panic_info);

    loop {
        hlt();
    }
}
