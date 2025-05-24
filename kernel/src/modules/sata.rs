use spin::Mutex;

use crate::{debug, error, ffi::FFIStr, pci::{Bar, Pci, PciDevice}, warn};

use super::{Module, ModuleMetadata};

pub(super) static SATA_MODULE: Module = Module {
    metadata: sata_metadata,
    init: sata_init,
};

static CONTROLLER: Mutex<Option<SataController>> = Mutex::new(None);

extern "C" fn sata_metadata() -> ModuleMetadata {
    ModuleMetadata { name: FFIStr::from("sata"), version_string: FFIStr::from("0.1.0") }
}

extern "C" fn sata_init() -> bool {
    let mut controllers = Pci::own_by_class(0x01, 0x06)
        .filter(|device| device.prog_if() == 0x1);

    match controllers.next() {
        Some(controller) => {
            debug!("    ///[{}] Found `{}`", sata_metadata(), controller);

            controllers.for_each(|controller| {
                debug!("    ///[{}] Ignoring `{}`", sata_metadata(), controller);
            });

            *CONTROLLER.lock() = SataController::init(controller);
            CONTROLLER.lock().is_some()
        },
        None => {
            warn!("    ///[{}] Missing controller", sata_metadata());
            false
        },
    }
}

struct SataController {
    //TODO:
}

impl SataController {
    fn init(device: PciDevice) -> Option<Self> {
        let bars = device.bars();

        let abar = match bars[5] {
            Some(abar) => abar,
            None => {
                warn!("    ///[{}] Abar not found on device", sata_metadata());
                return None;
            },
        };

        let _abar = match abar.memory_region() {
            Some(memory) => {
                debug!("    ///[{}] Abar in memory at 0x{:016x}-0x{:016x}", sata_metadata(), memory.as_ptr() as usize, memory.as_ptr() as usize + memory.len() - 1);
                memory
            },
            None => {
                warn!("    ///[{}] Abar in IO space!!!", sata_metadata());
                return None;
            },
        };

        error!("    ///[{}] TODO: IMPLEMENTATION", sata_metadata());
        None
    }
}
