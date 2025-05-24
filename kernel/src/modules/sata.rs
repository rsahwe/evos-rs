use crate::{ffi::FFIStr, warn};

use super::{Module, ModuleMetadata};

pub(super) static SATA_MODULE: Module = Module {
    metadata: sata_metadata,
    init: sata_init,
};

extern "C" fn sata_metadata() -> ModuleMetadata {
    ModuleMetadata { name: FFIStr::from("sata"), version_string: FFIStr::from("0.1.0") }
}

extern "C" fn sata_init() -> bool {
    warn!("    [{}] TODO: IMPLEMENTATION", sata_metadata());
    false
}
