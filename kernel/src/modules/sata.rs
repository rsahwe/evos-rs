use core::{
    alloc::{GlobalAlloc, Layout},
    cmp::max,
    hint::black_box,
    slice,
};

use spin::Mutex;
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{Mapper, Page, PageSize, PageTableFlags, PhysFrame, Size4KiB},
};

use crate::{
    ffi::FFIStr,
    interrupts::{PIC, PicEnd},
    mem::{self, VIRT_ALLOCATOR, VIRT_MAPPER},
    pci::{Pci, PciDevice},
    pfree, remap,
    time::Time,
};

use super::{Module, ModuleMetadata};

pub(super) static SATA_MODULE: Module = Module {
    metadata: sata_metadata,
    init: sata_init,
};

static CONTROLLER: Mutex<Option<SataController>> = Mutex::new(None);

extern "sysv64" fn sata_metadata() -> ModuleMetadata {
    ModuleMetadata {
        name: FFIStr::from("sata"),
        version_string: FFIStr::from("0.1.0"),
    }
}

macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::debug!("    /- [{}] {}", sata_metadata(), ::core::format_args!($($arg)*))
    };
}

macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::warn!("    /- [{}] {}", sata_metadata(), ::core::format_args!($($arg)*))
    };
}

macro_rules! error {
    ($($arg:tt)*) => {
        $crate::error!("    /- [{}] {}", sata_metadata(), ::core::format_args!($($arg)*))
    };
}

extern "sysv64" fn sata_init() -> bool {
    let mut controllers = Pci::own_by_class(0x01, 0x06).filter(|device| device.prog_if() == 0x1);

    match controllers.next() {
        Some(controller) => {
            debug!("Found `{}`", controller);

            controllers.for_each(|controller| {
                debug!("Ignoring `{}`", controller);
            });

            *CONTROLLER.lock() = SataController::new(controller);
            CONTROLLER.lock().is_some()
        }
        None => {
            warn!("Missing controller");
            false
        }
    }
}

fn sata_interrupt_handler(_pic_guard: PicEnd) {
    todo!("Sata interrupt called!");
}

struct SataController {
    ahci: &'static mut Ahci,
    //TODO:
}

impl SataController {
    fn new(device: PciDevice) -> Option<Self> {
        if device.irq() == 0xff {
            warn!("SATA IRQ not configured!!!");
            return None;
        }

        device.set_command((device.command() | 0x2) & !(1 << 10)); // Memory enable and not interrupt disable

        let bars = device.bars();

        let abar = match bars[5] {
            Some(abar) => abar,
            None => {
                warn!("Abar not found on device");
                return None;
            }
        };

        let mut abar = match abar.memory_region() {
            Some(memory) => {
                debug!(
                    "Abar in memory at 0x{:016x}-0x{:016x}",
                    memory.as_ptr() as usize,
                    memory.as_ptr() as usize + memory.len() - 1
                );
                memory
            }
            None => {
                warn!("Abar in IO space!!!");
                return None;
            }
        };

        //TODO: BETTER MAPPING CODE
        {
            let region_size = max(abar.len(), Size4KiB::SIZE as usize * 2);
            let region = unsafe {
                VIRT_ALLOCATOR
                    .alloc(Layout::from_size_align(region_size, Size4KiB::SIZE as usize).unwrap())
            };

            for page in Page::<Size4KiB>::range(
                Page::containing_address(VirtAddr::from_ptr(region)),
                Page::containing_address(VirtAddr::from_ptr(region.wrapping_add(region_size))),
            ) {
                let phys = PhysFrame::containing_address(PhysAddr::new(
                    page.start_address().as_u64() - region as u64 + abar.as_ptr() as u64
                        - mem::OFFSET,
                ));
                // SAFETY: VALID
                unsafe {
                    pfree!(
                        VIRT_MAPPER
                            .lock()
                            .as_mut()
                            .unwrap()
                            .translate_page(page)
                            .expect("Virtual allocator mapped incorrectly")
                    )
                };
                remap!(
                    page,
                    phys,
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::NO_CACHE
                        | PageTableFlags::GLOBAL
                );
            }

            // SAFETY: VALID
            abar = unsafe { slice::from_raw_parts_mut(region, abar.len()) };
        }

        // SAFETY: MEMORY WITH 0 PORTS IS VALID
        let base_size = unsafe {
            size_of_val(&*(core::ptr::slice_from_raw_parts(abar.as_ptr(), 0) as *const Ahci))
        };
        let port_amount = (abar.len() - base_size) / size_of::<AhciPort>();
        // SAFETY: MEMORY WITH port_amount PORTS IS VALID
        let mut ahci = unsafe {
            &mut *(core::ptr::slice_from_raw_parts_mut(abar.as_mut_ptr(), port_amount) as *mut Ahci)
        };

        if ahci.ports.len() != port_amount {
            warn!("Generated invalid reference to AHCI struct!!!");
            return None;
        }

        //TODO: REPLACE ILOG2 AS IT PANICS IF THE HIGHEST BIT IS SET
        let port_bits = ahci.port_implemented;
        if ((port_bits << 1) + 1).ilog2() as usize != ahci.ports.len() {
            if port_bits.ilog2() + 1 != port_bits.count_ones() {
                warn!("Ports implemented are not contiguous!!!");
            }

            // SAFETY: MEMORY WITH port_bits.count_ones() PORTS IS EXTRA VALID
            ahci = unsafe {
                &mut *(core::ptr::slice_from_raw_parts_mut(
                    ahci as *mut Ahci as *mut u8,
                    ((port_bits << 1) + 1).ilog2() as usize,
                ) as *mut Ahci)
            };

            if ahci.ports.len() != ((port_bits << 1) + 1).ilog2() as usize {
                warn!("Generated invalid reference to AHCI struct!!!");
                return None;
            }
        }

        if ahci.global_host_control.ilog2() != u32::MAX.ilog2() {
            warn!("Ahci is in IDE mode");
        }

        debug!(
            "Got valid reference to AHCI struct with {}({}) ports",
            ahci.port_implemented.count_ones(),
            ahci.ports.len()
        );

        Self::init(ahci, device.irq())
    }

    fn init(ahci: &'static mut Ahci, irq: u8) -> Option<Self> {
        ahci.global_host_control &= !0x2; //Interrupt enable

        let saved_capabilities =
            ahci.host_capabilities & ((1 << 28) | (1 << 27) | (1 << 17) | (1 << 6) | (1 << 5));
        let saved_ports_implemented = ahci.port_implemented;

        ahci.global_host_control |= 1 << 31;
        ahci.flush_writes();
        ahci.global_host_control |= 1;
        ahci.flush_writes();

        if !Time::timeout_poll_s(1, || ahci.global_host_control & 1 == 0) {
            error!("Ahci Reset timed out after 1 second!!!");
            return None;
        }

        ahci.global_host_control |= 1;
        ahci.flush_writes();
        ahci.host_capabilities |= saved_capabilities;
        ahci.port_implemented |= saved_ports_implemented;
        ahci.flush_writes();

        PIC.lock()
            .set_override(&(sata_interrupt_handler as fn(PicEnd)), irq);

        //TODO: PORTS INIT1
        error!("TODO: PORTS INIT1 IMPLEMENTATION");

        ahci.interrupt_status = black_box(ahci.interrupt_status);
        ahci.flush_writes();

        // enable interrupts
        ahci.global_host_control |= 0x2; //Interrupt enable
        ahci.flush_writes();

        //TODO: PORTS INIT2
        error!("TODO: PORTS INIT2 IMPLEMENTATION");

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
    reserved: [u8; 0xA0 - 0x2C],
    vendor_specific: [u8; 0x100 - 0xA0],
    ports: [AhciPort],
}

impl Ahci {
    fn flush_writes(&mut self) {
        let ptr = (&mut self.global_host_control) as *mut u32;
        // SAFETY: VALID
        unsafe { ptr.read_volatile() };
    }
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
