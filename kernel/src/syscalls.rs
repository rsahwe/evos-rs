use core::{arch::naked_asm, fmt::{Debug, Display}, mem::{offset_of, transmute}, ops::Index};

use spin::{Mutex, MutexGuard};
use x86_64::{instructions::interrupts::{disable, enable}, registers::{control::{Efer, EferFlags}, model_specific::{GsBase, KernelGsBase, LStar, SFMask, Star}, rflags::RFlags, segmentation::{Segment, GS}}, structures::gdt::SegmentSelector, VirtAddr};

use crate::{debug, descriptors::{KCS, KDS, UCS, UDS}, mem::STACK_SIZE};

static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

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
        debug!("Syscall entry at 0x{:016x}", syscall_entry as usize);
        debug!("Stack base 0x{:016x}", kernel_stack as *const _ as usize);
        debug!("Setting stack 0x{:016x}", kernel_stack as *const _ as usize + STACK_SIZE);
        self.kernel_stack = kernel_stack as *const _ as usize + STACK_SIZE;
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
            "push rcx",//SAVE START (WHY?)
            "push rdx",
            "push rdi",
            "push rsi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",//SAVE END
            "push 0",//RCX
            "push rdx",
            "mov r11, 0",
            "push rbp",
            "mov rbp, rsp",
            "and rsp, ~0xf",
            "push rax",//ARGS
            "push r9",
            "push r8",
            "push r10",
            "push rdx",
            "push rsi",
            "push rdi",
            "mov ax, {kernel_data_segment}",//RELOAD DS
            "mov ds, ax",
            "lea rax, [rip + {syscall_handler}]",
            "call rax",
            "add rsp, 7 * 8",
            "mov rsp, rbp",
            "pop rbp",
            "pop rdx",
            "pop rcx",
            "pop r11",//RESTORE START (WHY?)
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rsi",
            "pop rdi",
            "pop rdx",
            "pop rcx",//RESTORE END
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

#[repr(C)]
struct Combined(SyscallArgs, usize);//WHY?

extern "cdecl" fn syscall_handler(combined: Combined) -> usize {
    let (args, number) = (combined.0, combined.1);

    //TODO:
    enable();//TODO: ????

    debug!("Got syscall {} with args {}", number, args);

    disable();//TODO: ????

    0
}

pub fn init() {
    let mut gs_lock = GS_VARS.lock();

    // SAFETY: STACK IS A UNIQUE REFERENCE
    #[allow(static_mut_refs)]
    unsafe { gs_lock.init(&STACK) };

    Star::write(UCS, UDS, KCS, KDS).expect("Invalid GDT for syscalls!!!");
    LStar::write(VirtAddr::new(syscall_entry as u64));
    SFMask::write(RFlags::INTERRUPT_FLAG | RFlags::DIRECTION_FLAG);
    // SAFETY: VALID
    unsafe { Efer::update(|flags| flags.set(EferFlags::SYSTEM_CALL_EXTENSIONS, true)) };
    // SAFETY: VALID
    unsafe { GS::set_reg(KDS) };
    KernelGsBase::write(VirtAddr::new(MutexGuard::leak(gs_lock) as *const _ as u64));
    GsBase::write(VirtAddr::new(0));//USER CHANGES THIS
}
