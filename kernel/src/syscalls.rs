use core::{arch::naked_asm, fmt::{Debug, Display}, mem::{offset_of, transmute}, ops::Index};

use spin::{Mutex, MutexGuard};
use x86_64::{instructions::interrupts::{disable, enable}, registers::{model_specific::{KernelGsBase, LStar, SFMask, Star}, rflags::RFlags, segmentation::{Segment, GS}}, structures::gdt::SegmentSelector, VirtAddr};

use crate::{descriptors::{KCS, KDS, UCS, UDS}, mem::STACK_SIZE};

static STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

struct GSVars {
    user_stack_scratch: usize,
    kernel_stack: usize,
}

impl GSVars {
    const fn new_uninit() -> Self {
        Self {
            user_stack_scratch: 0,
            kernel_stack: 0
        }
    }

    /// SAFETY: STACK MUST BE A UNIQUE REFERENCE
    unsafe fn init(&mut self, kernel_stack: &[u8; STACK_SIZE]) {
        self.kernel_stack = kernel_stack as *const _ as usize;
    }
}

static GS_VARS: Mutex<GSVars> = Mutex::new(GSVars::new_uninit());

#[repr(C)]
#[derive(Clone, Copy, Hash)]
pub struct SyscallArgs(pub usize, pub usize, pub usize, pub usize, pub usize, pub usize);

impl Debug for SyscallArgs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SyscallArgs")
            .field("first", &self.0)
            .field("second", &self.1)
            .field("third", &self.2)
            .field("fourth", &self.3)
            .field("fifth", &self.4)
            .field("sixth", &self.5)
            .finish()
    }
}

impl Display for SyscallArgs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Index<usize> for SyscallArgs {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.0,
            1 => &self.1,
            2 => &self.2,
            3 => &self.3,
            4 => &self.4,
            5 => &self.5,
            _ => panic!("Indexed SyscallArgs out of bounds 0..6!!!"),
        }
    }
}

#[unsafe(naked)]
pub extern "sysv64" fn syscall_entry() -> ! {
    #[allow(unused_unsafe)]
    unsafe {
        naked_asm!(
            "swapgs",//GET KERNEL POINTER
            "mov gs:[{user_stack_scratch}], rsp",
            "mov rsp, gs:[{kernel_stack}]",
            "push {user_stack_segment}",
            "push gs:[{user_stack_scratch}]",//USER STACK
            "swapgs",//SAVE KERNEL POINTER
            "push r11",//RFLAGS
            "push {user_code_segment}",
            "push rcx",//USER RIP
            "push 0",//RCX
            "push rdx",
            "push 0",//R11
            "push r9",//ARGS
            "push r8",
            "push r10",
            "push rdx",
            "push rsi",
            "push rdi",
            "push rax",
            "mov ax, {kernel_data_segment}",//RELOAD DS
            "mov ds, ax",
            "lea rax, [rip + {syscall_handler}]",
            "call rax",
            "add rsp, 7 * 8",
            "pop rdx",
            "pop rcx",
            "iretq",
            kernel_stack = const offset_of!(GSVars, kernel_stack),
            user_stack_scratch = const offset_of!(GSVars, user_stack_scratch),
            syscall_handler = sym syscall_handler,
            kernel_data_segment = const transmute::<SegmentSelector, u16>(KDS),
            user_stack_segment = const transmute::<SegmentSelector, u16>(UDS),
            user_code_segment = const transmute::<SegmentSelector, u16>(UCS),
        )
    }
}

extern "cdecl" fn syscall_handler(number: usize, args: SyscallArgs) {
    //TODO:
    enable();//TODO: ????

    panic!("Got syscall {} with args {}", number, args);

    #[allow(unreachable_code)]
    disable();//TODO: ????
}

pub fn init() {
    let mut gs_lock = GS_VARS.lock();

    // SAFETY: STACK IS A UNIQUE REFERENCE
    unsafe { gs_lock.init(&STACK) };

    Star::write(UCS, UDS, KCS, KDS).expect("Invalid GDT for syscalls!!!");
    LStar::write(VirtAddr::new(syscall_entry as u64));
    SFMask::write(RFlags::INTERRUPT_FLAG | RFlags::DIRECTION_FLAG);
    unsafe { GS::set_reg(KDS) };
    KernelGsBase::write(VirtAddr::new(MutexGuard::leak(gs_lock) as *const _ as u64));
}
