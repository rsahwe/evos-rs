use core::{panic::PanicInfo, sync::atomic::{AtomicBool, Ordering}};

static HAS_PANICKED: AtomicBool = AtomicBool::new(false);
static HAS_PANICKED_AGAIN: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn kernel_panic(panic_info: &PanicInfo) -> ! {
    use crate::eprintln;
    use x86_64::instructions::interrupts::disable;

    disable();

    if HAS_PANICKED.load(Ordering::Relaxed) {
        if HAS_PANICKED_AGAIN.load(Ordering::Relaxed) {
            loop {}
        }

        HAS_PANICKED_AGAIN.store(true, Ordering::Relaxed);

        eprintln!("\nDOUBLE PANIC!!!\n{}", panic_info);

        loop {}
    }

    HAS_PANICKED.store(true, Ordering::Relaxed);

    eprintln!("\n{}", panic_info);

    loop {}
}
