use core::{mem::transmute, ops::RangeInclusive};

use spin::{Mutex, MutexGuard};
use x86_64::{instructions::{interrupts::enable, port::Port}, registers::control::Cr2, set_general_handler, structures::idt::{EntryOptions, ExceptionVector, InterruptDescriptorTable, InterruptStackFrame}, PrivilegeLevel};

use crate::{println, time::Time};

static HANDLER: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());

// SAFETY: ONLY USED HERE
static PIC: Mutex<Pic> = Mutex::new(unsafe { Pic::new() });

struct Pic {
    first_command: Port<u8>,
    first_data: Port<u8>,
    second_command: Port<u8>,
    second_data: Port<u8>,
    io_wait: Port<u8>,
}

impl Pic {
    const OFFSET: u8 = 0x20;

    /// SAFETY: NEEDS TO BE UNIQUE
    const unsafe fn new() ->  Self {
        Self {
            first_command: Port::new(0x20),
            first_data: Port::new(0x21),
            second_command: Port::new(0xA0),
            second_data: Port::new(0xA1),
            io_wait: Port::new(0x80),
        }
    }

    fn init(&mut self) {
        // SAFETY: VALID
        unsafe {
            self.first_command.write(0x11);// ICW1_ICW4 | ICW1_INIT
            self.io_wait();
            self.second_command.write(0x11);// ICW1_ICW4 | ICW1_INIT
            self.io_wait();
            self.first_data.write(Self::OFFSET);// OFFSET1
            self.io_wait();
            self.second_data.write(Self::OFFSET + 8);// OFFSET2
            self.io_wait();
            self.first_data.write(4);// SECOND PIC AT 0b0000_0100
            self.io_wait();
            self.second_data.write(2);// IDENTITY 0b0000_0010
            self.io_wait();
            self.first_data.write(0x01);// ICW4_8086
            self.io_wait();
            self.second_data.write(0x01);// ICW4_8086
            self.io_wait();
            self.mask();
        };
        // SAFETY: VALID
        unsafe {
            let mut pit_cmd = Port::<u8>::new(0x43);
            pit_cmd.write(0b0011_0110);// Channel 0b00, Access mode both 0b11, Mode 3 0b011, Binary Mode 0b0
            let mut pit_data = Port::<u8>::new(0x40);
            const PIT_RELOAD: u16 = 1193;// 1000 Hz (1000.1524 Hz) (999847.619 ns)
            // const PIT_RELOAD: u16 = 120;// 10000 Hz (9943.18182 Hz) (100571.429 ns)
            pit_data.write((PIT_RELOAD & 0xff) as u8);
            pit_data.write((PIT_RELOAD >> 8) as u8);

            Time::set_ps_tick_step(999847619);// 1000 Hz
            // Time::set_ps_tick_step(100571429);// 10000 Hz
        }
    }

    fn io_wait(&mut self) {
        // SAFETY: VALID
        unsafe { self.io_wait.write(0) };
    }

    /// SAFETY: NO PROCESS CAN BE ACTIVE
    unsafe fn mask(&mut self) {
        unsafe {
            self.first_data.write(0b1110_0000);// Disable Lpt1, Lpt2 and Floppy
            self.second_data.write(0b0010_1110);// Disable Processor, Free3, Free2 and Free1
        }
    }

    /// SAFETY: NEEDS TO BE IN THE INTERRUPT
    unsafe fn interrupt(&mut self, irq: PicInterrupt, _kernel: bool) {
        // SAFETY: VALID ONLY HERE
        let pic_guard = unsafe { PicEnd::new(irq) };

        match irq {
            PicInterrupt::Timer => Time::tick_step(pic_guard),//TODO: SCHEDULE? MAYBE CHECK FOR INTERRUPT IN INTERRUPT WITH LOCK?
            PicInterrupt::Keyboard => todo!("{:?}", irq),
            PicInterrupt::Com2 => todo!("{:?}", irq),
            PicInterrupt::Com1 => todo!("{:?}", irq),
            PicInterrupt::Cmos => todo!("{:?}", irq),
            PicInterrupt::Mouse => todo!("{:?}", irq),
            PicInterrupt::PrimaryAta => todo!("{:?}", irq),
            PicInterrupt::SecondaryAta => todo!("{:?}", irq),
            _ => unreachable!("Unexpected irq {:?}", irq),
        }
    }

    /// SAFETY: NEEDS TO BE IN AN INTERRUPT
    unsafe fn eoi(&mut self, irq: PicInterrupt) {
        if PIC_SECOND_RANGE.contains(&irq) {
            // SAFETY: VALID
            unsafe { self.second_command.write(0x20) };
        } else {
            // SAFETY: VALID
            unsafe { self.first_command.write(0x20) };
        }
    }
}

pub struct PicEnd {
    irq: PicInterrupt,
}

impl PicEnd {
    /// SAFETY: ONLY CONSTRUCTED BY PIC DUE TO FORCE_UNLOCK
    unsafe fn new(irq: PicInterrupt) -> Self {
        Self { irq }
    }
}

impl Drop for PicEnd {
    fn drop(&mut self) {
        // SAFETY: UNLOCKED AFTERWARDS ANYWAY
        unsafe { PIC.force_unlock() };
        // SAFETY: VALID
        unsafe { PIC.lock().eoi(self.irq) };
    }
}

const PIC_SECOND_RANGE: RangeInclusive<PicInterrupt> = PicInterrupt::Cmos..=PicInterrupt::SecondaryAta;

#[allow(unused)]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum PicInterrupt {
    Timer           = 0x00,
    Keyboard        = 0x01,
    Cascade         = 0x02,// Do not use.
    Com2            = 0x03,
    Com1            = 0x04,
    Lpt2            = 0x05,// Not important?
    Floppy          = 0x06,// Not important
    Lpt1            = 0x07,// Unreliable
    Cmos            = 0x08,
    Free1           = 0x09,// Not important?
    Free2           = 0x0A,// Not important?
    Free3           = 0x0B,// Not important?
    Mouse           = 0x0C,
    Processor       = 0x0D,// Not important?
    PrimaryAta      = 0x0E,
    SecondaryAta    = 0x0F,
}

impl From<PicInterrupt> for u8 {
    fn from(value: PicInterrupt) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for PicInterrupt {
    type Error = ();

    fn try_from(interrupt: u8) -> Result<Self, Self::Error> {
        if interrupt.wrapping_sub(Pic::OFFSET) < 16 {
            // SAFETY: SAFE
            Ok(unsafe { transmute(interrupt.wrapping_sub(Pic::OFFSET)) })
        } else {
            Err(())
        }
    }
}

pub fn init() {
    // LOCK SAFETY: ONLY ACCESSED HERE
    let mut idt = HANDLER.lock();
    set_general_handler!(&mut idt, handler_func);
    
    macro_rules! change_entry_options {
        ($entry:ident, $closure:expr) => {
            let mut entry = idt.$entry;
            // SAFETY: ADDR MATCHES
            let options = unsafe { entry.set_handler_addr(entry.handler_addr()) };
            $closure(options);
            idt.$entry = entry;
        };
    }
    
    change_entry_options!(double_fault, |options: &mut EntryOptions| {
        // SAFETY: INDEX IS VALID
        unsafe { options.set_stack_index(0) };
    });

    change_entry_options!(breakpoint, |options: &mut EntryOptions| {
        options.set_privilege_level(PrivilegeLevel::Ring3);
    });

    MutexGuard::leak(idt).load();
    
    PIC.lock().init();
    enable();
}

fn handler_func(frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    if frame.code_segment.rpl() == PrivilegeLevel::Ring0 {
        match ExceptionVector::try_from(index) {
            Ok(vector) => {
                match vector {
                    ExceptionVector::Page => panic!("kernel page fault e {} with frame:\n{:#?}\nand addr: {:?}", error_code.unwrap(), frame, Cr2::read()),
                    _ => unreachable!("Unexpected interrupt {:?} with frame:\n{:#?}", vector, frame),//Should be unreachable right?
                }
            },
            Err(_) => {
                match PicInterrupt::try_from(index) {
                    // SAFETY: VALID AND ONLY LOCKED HERE
                    Ok(irq) => unsafe { PIC.lock().interrupt(irq, true) },
                    Err(_) => panic!("Unexpected kernel interrupt {}", index),
                }
            }
        }
    } else {
        match ExceptionVector::try_from(index) {
            Ok(vector) => {
                match vector {
                    // TODO: COLLECT FATAL
                    _ => println!("EMERGENCY WARN: unhandled user exception {:?}", vector),
                }
            },
            Err(_) => {
                match PicInterrupt::try_from(index) {
                    // SAFETY: VALID AND ONLY LOCKED HERE
                    Ok(irq) => unsafe { PIC.lock().interrupt(irq, false) },
                    Err(_) => panic!("Unexpected kernel interrupt {}", index),
                }
            }
        }
    }
}
