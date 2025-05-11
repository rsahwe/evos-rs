use core::fmt::Display;

use crate::{print, println};

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

static KERNEL_MODULES: &[&Module] = &[
    
];

pub(crate) fn init() {
    println!("DEBUG: Initializing modules:");
    for module in KERNEL_MODULES {
        print!("DEBUG:     Initializing module `{}`...", (module.metadata)());
        let success = (module.init)();
        println!("{}", if success { "[OK]" } else { "[ERR]" });
    }
}
