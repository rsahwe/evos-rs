#![allow(unexpected_cfgs)]

use core::{fmt::Display, mem::MaybeUninit};

use spin::Mutex;

use crate::{debug, error, ffi::FFIStr};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleMetadata {
    pub name: FFIStr<'static>,
    pub version_string: FFIStr<'static>,
}

impl Display for ModuleMetadata {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} {}", <FFIStr as Into<&str>>::into(self.name), <FFIStr as Into<&str>>::into(self.version_string))
    }
}

/// Kernel module. Exist so that parts of the kernel can fail without panicking.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Module {
    metadata: extern "C" fn() -> ModuleMetadata,
    init: extern "C" fn() -> bool,
}

pub(crate) mod ps2;

static KERNEL_MODULES: &[&Module] = &[
    #[cfg(module_ps2)]
    &ps2::PS2_MODULE,
];

static EXTRA_KERNEL_MODULES: Mutex<([MaybeUninit<Module>; 255], usize)> = Mutex::new(([MaybeUninit::uninit(); 255], 0));

pub(crate) fn init() -> (usize, usize) {
    debug!("Initializing modules:");

    let mut count = 0;

    for module in KERNEL_MODULES {
        debug!("    Initializing module `{}`:", (module.metadata)());
        let success = (module.init)();
        debug!("    Module loaded {}", if success { "[OK]" } else { "[ERR]" });
        count += success as usize;
    }
    
    (count, KERNEL_MODULES.len())
}

pub fn register(module: Module) -> bool {
    debug!("Registering late module `{}`:", (module.metadata)());
    
    let mut guard = EXTRA_KERNEL_MODULES.lock();

    if guard.1 >= guard.0.len() {
        error!("No module space left!!!");
        false
    } else {
        let success = (module.init)();
        debug!("Module loaded {}", if success { "[OK]" } else { "[ERR]" });
        if success {
            let index = guard.1;
            guard.0[index].write(module);
            guard.1 += 1;
            true
        } else {
            false
        }
    }
}
