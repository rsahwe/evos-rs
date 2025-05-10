use spin::{Mutex, MutexGuard};
use x86_64::{instructions::tables::load_tss, registers::segmentation::{Segment, CS, DS, SS}, structures::{gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector}, tss::TaskStateSegment}, PrivilegeLevel, VirtAddr};

use crate::mem::STACK_SIZE;

static GLOBAL: Mutex<GlobalDescriptorTable> = Mutex::new(GlobalDescriptorTable::new());
static TASK: Mutex<TaskStateSegment> = Mutex::new(TaskStateSegment::new());

pub const KCS: SegmentSelector = SegmentSelector::new(1, PrivilegeLevel::Ring0);
pub const KDS: SegmentSelector = SegmentSelector::new(2, PrivilegeLevel::Ring0);
pub const UDS: SegmentSelector = SegmentSelector::new(3, PrivilegeLevel::Ring3);
pub const UCS: SegmentSelector = SegmentSelector::new(4, PrivilegeLevel::Ring3);
pub const TSS: SegmentSelector = SegmentSelector::new(5, PrivilegeLevel::Ring0);

pub fn init() {
    // LOCK SAFETY: ONLY LOCKED HERE
    let mut tss = TASK.lock();

    tss.interrupt_stack_table[0] = {
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

        VirtAddr::from_ptr(&raw const STACK) + STACK_SIZE as u64
    };
    tss.privilege_stack_table[0] = {
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

        VirtAddr::from_ptr(&raw const STACK) + STACK_SIZE as u64
    };
    // LOCK SAFETY: ONLY LOCKED HERE
    let mut gdt = GLOBAL.lock();

    assert_eq!(gdt.append(Descriptor::kernel_code_segment()), KCS);
    assert_eq!(gdt.append(Descriptor::kernel_data_segment()), KDS);
    assert_eq!(gdt.append(Descriptor::user_data_segment()), UDS);
    assert_eq!(gdt.append(Descriptor::user_code_segment()), UCS);
    assert_eq!(gdt.append(Descriptor::tss_segment(MutexGuard::leak(tss))), TSS);

    MutexGuard::leak(gdt).load();

    // SAFETY: SEGMENTS ARE VALID AND LOADED
    unsafe {
        DS::set_reg(KDS);
        SS::set_reg(KDS);
        CS::set_reg(KCS);
    }

    // SAFETY: TSS IS VALID
    unsafe { load_tss(TSS) };
}
