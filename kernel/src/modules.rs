#![allow(unexpected_cfgs)]

use core::fmt::Display;

use crate::debug;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleMetadata {
    pub name: &'static str,
    pub version_string: &'static str,
}

impl Display for ModuleMetadata {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} {}", self.name, self.version_string)
    }
}

/// Kernel module. Exist so that parts of the kernel can fail without panicking.
pub struct Module {
    metadata: fn() -> ModuleMetadata,
    init: fn() -> bool,
}

pub(crate) mod ps2;

static KERNEL_MODULES: &[&Module] = &[
    #[cfg(module_ps2)]
    &ps2::PS2_MODULE,
];

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
