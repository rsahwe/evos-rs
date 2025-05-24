use core::{fmt::Display, mem::MaybeUninit, slice, sync::atomic::{AtomicBool, Ordering}};

use spin::RwLock;
use x86_64::{instructions::port::Port, structures::port::{PortRead, PortWrite}};

use crate::{debug, error, mem::{self, virt::VirtFrame}, warn};

const VENDOR_OFFSET: u8             = 0x00;
const DEVICE_ID_OFFSET: u8          = 0x02;
const COMMAND_OFFSET: u8            = 0x04;
const STATUS_OFFSET: u8             = 0x06;
const PROG_IF_REV_OFFSET: u8        = 0x08;
const CLASS_SUBCLASS_OFFSET: u8     = 0x0a;
#[allow(unused)]
const TIMER_CACHE_LINE_OFFSET: u8   = 0x0c;
const BIST_HEADER_TYPE_OFFSET: u8   = 0x0e;

pub struct Pci;

impl Pci {
    fn read_config(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
        let address = ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | (offset as u32 & 0xfc) | 0x80000000;

        // SAFETY: SAFE
        unsafe { Port::new(0xcf8).write(address) };

        // SAFETY: SAFE
        ((unsafe { Port::<u32>::new(0xcfc).read() } >> ((offset & 2) * 8)) & 0xFFFF) as u16
    }

    fn write_config(bus: u8, slot: u8, func: u8, offset: u8, value: u16) {
        let address = ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | (offset as u32 & 0xfc) | 0x80000000;

        // SAFETY: SAFE
        unsafe { Port::new(0xcf8).write(address) };

        let mut port = Port::<u32>::new(0xcfc);
        let shift = (offset & 2) * 8;

        // SAFETY: SAFE
        let previous = unsafe { port.read() };
        // SAFETY: SAFE
        unsafe { port.write((previous & !(0xFF << shift)) | ((value as u32) << shift)); }
    }

    fn set_bar_address(bus: u8, slot: u8, func: u8, bar: u8) -> Port<u32> {
        let address = ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | ((0x10 + (bar as u32 * 4)) & 0xfc) | 0x80000000;

        // SAFETY: SAFE
        unsafe { Port::new(0xcf8).write(address) };

        Port::new(0xcfc)
    }

    fn read_bar(bus: u8, slot: u8, func: u8, bar: u8) -> u32 {
        let mut port = Self::set_bar_address(bus, slot, func, bar);

        // SAFETY: SAFE
        unsafe { port.read() }
    }

    fn write_bar(bus: u8, slot: u8, func: u8, bar: u8, value: u32) {
        let mut port = Self::set_bar_address(bus, slot, func, bar);

        // SAFETY: SAFE
        unsafe { port.write(value) }
    }

    fn with_memory_disabled<T, F: FnOnce() -> T>(bus: u8, slot: u8, func: u8, f: F) -> T {
        let cmd = Self::read_config(bus, slot, func, COMMAND_OFFSET);

        Self::write_config(bus, slot, func, COMMAND_OFFSET, cmd & !0x3);

        let res = f();

        Self::write_config(bus, slot, func, COMMAND_OFFSET, cmd);

        res
    }

    fn iter() -> PciDeviceIterator {
        PciDeviceIterator { bus: 0, slot: 0, func: 0 }
    }

    pub fn own_by_class(class: u8, subclass: u8) -> OwningPciDeviceIterator {
        OwningPciDeviceIterator { index: 0, class, subclass }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PciDevice {
    bus: u8,
    slot: u8,
    func: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bar {
    Memory { data: usize, len: usize },
    Port { base: u16, len: usize },
}

impl PciDevice {
    pub fn vendor(&self) -> u16 {
        Pci::read_config(self.bus, self.slot, self.func, VENDOR_OFFSET)
    }

    pub fn id(&self) -> u16 {
        Pci::read_config(self.bus, self.slot, self.func, DEVICE_ID_OFFSET)
    }

    pub fn command(&self) -> u16 {
        Pci::read_config(self.bus, self.slot, self.func, COMMAND_OFFSET)
    }

    pub fn status(&self) -> u16 {
        Pci::read_config(self.bus, self.slot, self.func, STATUS_OFFSET)
    }

    pub fn prog_if(&self) -> u8 {
        (Pci::read_config(self.bus, self.slot, self.func, PROG_IF_REV_OFFSET) >> 8) as u8
    }

    pub fn revision(&self) -> u8 {
        (Pci::read_config(self.bus, self.slot, self.func, PROG_IF_REV_OFFSET) & 0xFF) as u8
    }

    pub fn class(&self) -> (u8, u8) {
        let whole = Pci::read_config(self.bus, self.slot, self.func, CLASS_SUBCLASS_OFFSET);
        ((whole >> 8) as u8, (whole & 0xFF) as u8)
    }

    pub fn bist(&self) -> u8 {
        (Pci::read_config(self.bus as u8, self.slot, self.func, BIST_HEADER_TYPE_OFFSET) >> 8) as u8
    }

    pub fn bars(&self) -> [Option<Bar>; 6] {
        Pci::with_memory_disabled(self.bus, self.slot, self.func, || {
            let mut bars = [None; 6];

            let mut index = 0;

            while index < 6 {
                let bar = Pci::read_bar(self.bus, self.slot, self.func, index);

                Pci::write_bar(self.bus, self.slot, self.func, index, !0);

                let size = Pci::read_bar(self.bus, self.slot, self.func, index);

                Pci::write_bar(self.bus, self.slot, self.func, index, bar);

                if bar & 1 == 1 {
                    if (size & !3) != 0 {
                        match (bar & !3).try_into() {
                            Ok(portbase) => bars[index as usize] = Some(Bar::Port { base: portbase, len: (!(size & !3) + 1) as usize }),
                            Err(err) => error!("PCI Bar IO base too high: {}", err),
                        }
                    }
                } else {
                    if (size & !0xf) != 0 {
                        if bar & 0b110 == 0b100 {
                            index += 1;

                            if index == 6 {
                                error!("No slot left for 64-bit bar!");
                                continue;
                            }

                            let second_bar = Pci::read_bar(self.bus, self.slot, self.func, index);

                            Pci::write_bar(self.bus, self.slot, self.func, index, !0);

                            let second_size = Pci::read_bar(self.bus, self.slot, self.func, index);

                            Pci::write_bar(self.bus, self.slot, self.func, index, second_bar);

                            let base = ((second_bar as usize) << 32) | (bar as usize);
                            let size = ((second_size as usize) << 32) | (size as usize);

                            bars[index as usize] = Some(Bar::Memory { data: base & !0xf, len: !(size & !0xf) + 1 })
                        } else {
                            bars[index as usize] = Some(Bar::Memory { data: bar as usize & !0xf, len: !(size & !0xf) as usize + 1 })
                        }
                    }
                }

                index += 1;
            }

            bars
        })
    }
}

impl Display for PciDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PCI device {:04x}:{:04x} class {:02x}:{:02x}(:{:02x}) revision 0x{:02x} at {:02x}:{:02x}.{}",
            self.vendor(),
            self.id(),
            self.class().0,
            self.class().1,
            self.prog_if(),
            self.revision(),
            self.bus,
            self.slot,
            self.func
        )
    }
}

impl Bar {
    pub fn memory_region(&self) -> Option<&'static mut [u8]> {
        match self {
            // SAFETY: ALLOCATED BY BIOS
            Bar::Memory { data, len } => unsafe { Some(slice::from_raw_parts_mut((data + mem::OFFSET as usize) as *mut u8, *len)) },
            Bar::Port { base: _, len: _ } => None,
        }
    }

    pub fn port<T: PortRead + PortWrite>(&self, offset: u16) -> Option<Result<Port<T>, &'static str>> {
        match self {
            Bar::Memory { data: _, len: _ } => None,
            Bar::Port { base, len } => if offset as usize >= *len { Some(Err("index out of bounds")) } else { Some(Ok(Port::new(*base + offset))) },
        }
    }
}

struct PciDeviceIterator {
    bus: u16,
    slot: u8,
    func: u8,
}

impl Iterator for PciDeviceIterator {
    type Item = PciDevice;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.bus == 256 {
                return None
            }

            let device = PciDevice { bus: self.bus as u8, slot: self.slot, func: self.func };

            let res = device.vendor() != u16::MAX;

            if self.func == 7 || (self.func == 0 && !(res && (Pci::read_config(self.bus as u8, self.slot, self.func, BIST_HEADER_TYPE_OFFSET) & 0x0080) != 0)) {
                self.func = 0;
                self.slot += 1;
                if self.slot == 32 {
                    self.slot = 0;
                    self.bus += 1;
                }
            } else {
                self.func += 1;
            }

            if res && device.class().0 != 6 {
                return Some(device)
            }
        }
    }
}

struct PciDeviceCollector {
    devices: [(MaybeUninit<PciDevice>, AtomicBool); 1000],
    count: usize,
}

impl FromIterator<PciDevice> for VirtFrame<PciDeviceCollector> {
    fn from_iter<T: IntoIterator<Item = PciDevice>>(iter: T) -> Self {
        let mut this = Self::new(PciDeviceCollector { devices: [const { (MaybeUninit::uninit(), AtomicBool::new(false)) }; 1000], count: 0 });

        for el in iter {
            if this.count == this.devices.len() {
                error!("Got more PCI devices than expected!!!");
                warn!("All extra devices will be ignored!!!");
                break;
            }
            let index = this.count;
            this.devices[index].0.write(el);
            this.count += 1;
        }

        this
    }
}

static PCI_DEVICES: RwLock<Option<VirtFrame<PciDeviceCollector>>> = RwLock::new(None);

pub fn init() -> usize {
    debug!("Enumerating Pci bus:");

    let frame = Pci::iter().inspect(|device| {
        debug!("    Found `{}`", device);
    }).collect::<VirtFrame<PciDeviceCollector>>();

    *PCI_DEVICES.write() = Some(frame);

    PCI_DEVICES.read().as_ref().unwrap().count
}

pub struct OwningPciDeviceIterator {
    index: usize,
    class: u8,
    subclass: u8,
}

impl Iterator for OwningPciDeviceIterator {
    type Item = PciDevice;

    fn next(&mut self) -> Option<Self::Item> {
        let guard = PCI_DEVICES.read();
        let devices = guard.as_ref().unwrap();

        while self.index < devices.count {
            let full_ref = &devices.devices[self.index];

            if !full_ref.1.swap(true, Ordering::Relaxed) {
                    // SAFETY: VALID IN FROMITERATOR IMPLEMENTATION
                let device_ref = unsafe { full_ref.0.assume_init_ref() };

                if device_ref.class() == (self.class, self.subclass) {
                    return Some(*device_ref)
                } else {
                    full_ref.1.store(false, Ordering::Relaxed);
                }
            }

            self.index += 1;
        }

        None
    }
}
