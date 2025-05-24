use spin::Mutex;

use crate::{debug, ffi::FFIStr, pci::{Pci, PciDevice}, warn};

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
    let mut controllers = Pci::own_by_class(0x01, 0x01).chain(Pci::own_by_class(0x01, 0x06));

    match controllers.next() {
        Some(mut controller) => {
            debug!("    ///[{}] Found `{}`", sata_metadata(), controller);

            if controller.class().1 == 0x01 {
                let mut controllers = controllers.skip_while(|device| {
                    debug!("    ///[{}] Ignoring `{}`", sata_metadata(), device);
                    device.class().1 == 0x01
                });

                match controllers.next() {
                    Some(sata_controller) => {
                        controller = sata_controller;
                        debug!("    ///[{}] Found `{}` and using it instead", sata_metadata(), controller);
                    },
                    None => (),
                }

                controllers.for_each(|controller| {
                    debug!("    ///[{}] Ignoring `{}`", sata_metadata(), controller);
                });
            } else {
                controllers.for_each(|controller| {
                    debug!("    ///[{}] Ignoring `{}`", sata_metadata(), controller);
                });
            }

            *CONTROLLER.lock() = Some(SataController::init(controller));
            warn!("    ///[{}] TODO: IMPLEMENTATION", sata_metadata());
            false
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
    fn init(_device: PciDevice) -> Self {
        todo!("Sata init")
    }
}
