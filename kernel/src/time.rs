use core::{hint::spin_loop, sync::atomic::{AtomicU16, AtomicU64, Ordering}};

use crate::interrupts::PicEnd;

static BOOT_NS: AtomicU64 = AtomicU64::new(0);
static PS_TICK_STEP: AtomicU64 = AtomicU64::new(0);
static BOOT_PS_PART: AtomicU16 = AtomicU16::new(0);

pub struct Time {}

impl Time {
    pub fn boot_time_ns() -> u64 {
        BOOT_NS.load(Ordering::Relaxed)
    }

    pub fn timeout_poll_ns<F: FnMut() -> bool>(timeout_ns: u64, mut poll: F) -> bool {
        let end_time_ns = Self::boot_time_ns() + timeout_ns;

        while Self::boot_time_ns() <= end_time_ns {
            if poll() {
                return true;
            }

            spin_loop();
        }

        false
    }

    pub fn timeout_poll_us<F: FnMut() -> bool>(timeout_us: u64, poll: F) -> bool {
        Self::timeout_poll_ns(timeout_us * 1000, poll)
    }

    pub fn timeout_poll_ms<F: FnMut() -> bool>(timeout_ms: u64, poll: F) -> bool {
        Self::timeout_poll_us(timeout_ms * 1000, poll)
    }

    pub fn timeout_poll_s<F: FnMut() -> bool>(timeout_s: u64, poll: F) -> bool {
        Self::timeout_poll_ms(timeout_s * 1000, poll)
    }

    pub(crate) fn set_ps_tick_step(step: u64) {
        PS_TICK_STEP.store(step, Ordering::Relaxed);

    }

    pub(crate) fn tick_step(_guard: PicEnd) {
        let mut step = PS_TICK_STEP.load(Ordering::Relaxed);

        BOOT_PS_PART.fetch_add((step % 1000) as u16, Ordering::Relaxed);
        if BOOT_PS_PART.load(Ordering::Relaxed) >= 1000 {
            BOOT_PS_PART.fetch_sub(1000, Ordering::Relaxed);
            step += 1000;
        }

        BOOT_NS.fetch_add(step / 1000, Ordering::Relaxed);
    }
}
