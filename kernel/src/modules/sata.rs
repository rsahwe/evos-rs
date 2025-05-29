use core::{alloc::{GlobalAlloc, Layout}, cmp::max, slice};

use spin::Mutex;
use x86_64::{structures::paging::{Mapper, Page, PageSize, PageTableFlags, PhysFrame, Size4KiB}, PhysAddr, VirtAddr};

use crate::{debug, error, ffi::FFIStr, mem::{self, VIRT_ALLOCATOR, VIRT_MAPPER}, pci::{Pci, PciDevice}, pfree, remap, warn};

use super::{Module, ModuleMetadata};

pub(super) static SATA_MODULE: Module = Module {
    metadata: sata_metadata,
    init: sata_init,
};

static CONTROLLER: Mutex<Option<SataController>> = Mutex::new(None);

extern "sysv64" fn sata_metadata() -> ModuleMetadata {
    ModuleMetadata { name: FFIStr::from("sata"), version_string: FFIStr::from("0.1.0") }
}

extern "sysv64" fn sata_init() -> bool {
    let mut controllers = Pci::own_by_class(0x01, 0x06)
        .filter(|device| device.prog_if() == 0x1);

    match controllers.next() {
        Some(controller) => {
            debug!("    /- [{}] Found `{}`", sata_metadata(), controller);

            controllers.for_each(|controller| {
                debug!("    /- [{}] Ignoring `{}`", sata_metadata(), controller);
            });

            *CONTROLLER.lock() = SataController::new(controller);
            CONTROLLER.lock().is_some()
        },
        None => {
            warn!("    /- [{}] Missing controller", sata_metadata());
            false
        },
    }
}

struct SataController {
    //TODO:
}

impl SataController {
    fn new(device: PciDevice) -> Option<Self> {
        if device.irq() == 0xff {
            warn!("    /- [{}] SATA IRQ not configured!!!", sata_metadata());
            return None;
        }

        device.set_command((device.command() | 0x2) & !(1 << 10));// Memory enable and not interrupt disable

        let bars = device.bars();

        let abar = match bars[5] {
            Some(abar) => abar,
            None => {
                warn!("    /- [{}] Abar not found on device", sata_metadata());
                return None;
            },
        };

        let mut abar = match abar.memory_region() {
            Some(memory) => {
                debug!("    /- [{}] Abar in memory at 0x{:016x}-0x{:016x}", sata_metadata(), memory.as_ptr() as usize, memory.as_ptr() as usize + memory.len() - 1);
                memory
            },
            None => {
                warn!("    /- [{}] Abar in IO space!!!", sata_metadata());
                return None;
            },
        };

        //TODO: BETTER MAPPING CODE
        {
            let region_size = max(abar.len(), Size4KiB::SIZE as usize * 2);
            let region = unsafe { VIRT_ALLOCATOR.alloc(Layout::from_size_align(region_size, Size4KiB::SIZE as usize).unwrap()) };

            for page in Page::<Size4KiB>::range(Page::containing_address(VirtAddr::from_ptr(region)), Page::containing_address(VirtAddr::from_ptr(region.wrapping_add(region_size)))) {
                let phys = PhysFrame::containing_address(PhysAddr::new(page.start_address().as_u64() - region as u64 + abar.as_ptr() as u64 - mem::OFFSET));
                // SAFETY: VALID
                unsafe { pfree!(VIRT_MAPPER.lock().as_mut().unwrap().translate_page(page).expect("Virtual allocator mapped incorrectly")) };
                remap!(page, phys, PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE | PageTableFlags::GLOBAL);
            }

            // SAFETY: VALID
            abar = unsafe { slice::from_raw_parts_mut(region, abar.len()) };
        }

        // SAFETY: MEMORY WITH 0 PORTS IS VALID
        let base_size = unsafe { size_of_val(&*(slice::from_raw_parts(abar.as_ptr(), 0) as *const [u8] as *const Ahci)) };
        let port_amount = (abar.len() - base_size) / size_of::<AhciPort>();
        // SAFETY: MEMORY WITH port_amount PORTS IS VALID
        let mut ahci = unsafe { &mut *(slice::from_raw_parts_mut(abar.as_mut_ptr(), port_amount) as *mut [u8] as *mut Ahci) };

        if ahci.ports.len() != port_amount {
            warn!("    /- [{}] Generated invalid reference to AHCI struct!!!", sata_metadata());
            return None;
        }

        //TODO: REPLACE ILOG2 AS IT PANICS IF THE HIGHEST BIT IS SET
        let port_bits = ahci.port_implemented;
        if ((port_bits << 1) + 1).ilog2() as usize != ahci.ports.len() {
            if port_bits.ilog2() + 1 != port_bits.count_ones() {
                warn!("    /- [{}] Ports implemented are not contiguous!!!", sata_metadata());
            }

            // SAFETY: MEMORY WITH port_bits.count_ones() PORTS IS EXTRA VALID
            ahci = unsafe { &mut *(slice::from_raw_parts_mut(ahci as *mut Ahci as *mut u8, ((port_bits << 1) + 1).ilog2() as usize) as *mut [u8] as *mut Ahci) };

            if ahci.ports.len() != ((port_bits << 1) + 1).ilog2() as usize {
                warn!("    /- [{}] Generated invalid reference to AHCI struct!!!", sata_metadata());
                return None;
            }
        }

        if ahci.global_host_control.ilog2() != usize::MAX.ilog2() {
            warn!("    /- [{}] Ahci is in IDE mode", sata_metadata());
        }

        debug!("    /- [{}] Got valid reference to AHCI struct with {}({}) ports", sata_metadata(), ahci.port_implemented.count_ones(), ahci.ports.len());

        Self::init(ahci, device.irq())
    }

    fn init(ahci: &'static mut Ahci, irq: u8) -> Option<Self> {
        ahci.global_host_control &= !0x2;//Interrupt enable

        error!("    /- [{}] TODO: INIT IMPLEMENTATION", sata_metadata());

        None
    }
}

#[repr(C)]
struct Ahci {
    host_capabilities: u32,
    global_host_control: u32,
    interrupt_status: u32,
    port_implemented: u32,
    version: u32,
    command_completion_coalescing_control: u32,
    command_completion_coalescing_ports: u32,
    enclosure_management_location: u32,
    enclosure_management_control: u32,
    host_capabilities_extended: u32,
    bios_handoff_control_and_status: u32,
    reserved: [u8; 0xA0-0x2C],
    vendor_specific: [u8; 0x100-0xA0],
    ports: [AhciPort],
}

#[repr(C)]
struct AhciPort {
    command_list_base_l: u32,
    command_list_base_h: u32,
    fis_base_l: u32,
    fis_base_h: u32,
    interrupt_status: u32,
    interrupt_enable: u32,
    command_and_status: u32,
    reserved: u32,
    task_file_data: u32,
    signature: u32,
    sata_status: u32,
    sata_control: u32,
    sata_error: u32,
    sata_active: u32,
    command_issue: u32,
    sata_notification: u32,
    fis_based_switch_control: u32,
    reserved_again: [u32; 11],
    vendor_specific: [u32; 4],
}
