use core::sync::atomic::{AtomicBool, Ordering};

use pc_keyboard::{HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

use crate::{debug, ffi::FFIStr};

use super::{Module, ModuleMetadata};

pub(super) static PS2_MODULE: Module = Module {
    metadata: ps2_metadata,
    init: ps2_init,
};

static KEYBOARD: Mutex<Keyboard<crate::config::keyboard::Layout, ScancodeSet1>> = Mutex::new(Keyboard::new(ScancodeSet1::new(), crate::config::keyboard::new_layout(), HandleControl::MapLettersToUnicode));

static KEYBOARD_EXISTS: AtomicBool = AtomicBool::new(false);

const PS2_CONTROL: (
    // Data
    Port<u8>,
    // Status
    PortReadOnly<u8>,
    // Command
    PortWriteOnly<u8>
) = (Port::new(0x60), PortReadOnly::new(0x64), PortWriteOnly::new(0x64));

extern "C" fn ps2_metadata() -> ModuleMetadata {
    ModuleMetadata { name: FFIStr::from("ps2"), version_string: FFIStr::from("0.1.0") }
}

extern "C" fn ps2_init() -> bool {
    //TODO: CHECK
    let mut _ps2_control = PS2_CONTROL;

    KEYBOARD_EXISTS.store(true, Ordering::Relaxed);
    debug!("    [{}] Keyboard assumed to exist...", ps2_metadata());

    true
}

pub fn ps2_keyboard_interrupt() {
    if !cfg!(module_ps2) {
        return;
    }

    if !KEYBOARD_EXISTS.load(Ordering::Relaxed) {
        return;
    }
    
    let mut ps2_control = PS2_CONTROL;

    // SAFETY: PORT STUFF VALID
    let scancode = unsafe { ps2_control.0.read() };

    let mut keyboard_guard = KEYBOARD.lock();

    match keyboard_guard.add_byte(scancode) {
        Ok(key) => match key.map(|ke| keyboard_guard.process_keyevent(ke)) {
            Some(key) => match key {
                Some(key) => debug!("KEYBOARD: {:?}", key),//TODO:
                None => (),
            },
            None => (),
        },
        Err(_) => (),
    }
}
