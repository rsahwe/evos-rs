use core::ops::Add;

use bootloader_api::{config::Mapping, info::MemoryRegions};
use phys::PageFrameAllocator;
use spin::Mutex;
use virt::GAlloc;
use x86_64::{registers::control::Cr3, structures::paging::{OffsetPageTable, Page, PageSize, PageTable, PageTableFlags, Size4KiB}, VirtAddr};

use crate::debug;

pub mod phys;
pub mod virt;

pub const MIN_PHYSICAL_FREE: usize = 1024 * 1024 * 10; // 10 MiB
pub const OFFSET: u64 = 0xffff800000000000;
pub const HEAP_VIRT_SIZE: usize = 1024 * 1024 * 1024 * 1; // 1 GiB
pub const HEAP_VIRT_BASE: usize = 0usize.wrapping_sub(HEAP_VIRT_SIZE);
pub const HEAP_BLOCK_SIZE: usize = 1024 * 1024 * 1; // 1 MiB

pub const STACK_SIZE: usize = 100 * 1024;

/// LOCK SAFETY: NOT USED IN KERNEL INTERRUPTS
pub static PHYS_ALLOCATOR: Mutex<Option<PageFrameAllocator>> = Mutex::new(None);
/// LOCK SAFETY: NOT USED IN KERNEL INTERRUPTS
#[global_allocator]
pub static VIRT_ALLOCATOR: GAlloc = GAlloc::new();
/// LOCK SAFETY: NOT USED IN KERNEL INTERRUPTS
pub static VIRT_MAPPER: Mutex<Option<OffsetPageTable<'static>>> = Mutex::new(None);

#[macro_export]
macro_rules! palloc {
    () => {
        ::x86_64::structures::paging::FrameAllocator::allocate_frame($crate::mem::PHYS_ALLOCATOR.lock().as_mut().expect("Allocator missing!!!")).expect("Physical OOM!!!")
    };
}

#[macro_export]
macro_rules! palloc_loop {
    ($range:expr, $closure:expr) => {
        let (range, closure) = ($range, $closure);

        let mut phys_guard = $crate::mem::PHYS_ALLOCATOR.lock();
        let phys = phys_guard.as_mut().expect("Allocator missing!!!");
        let phys_raw = &raw mut *phys;

        for el in range {
            // SAFETY: STILL LOCKED
            let phys = unsafe { &mut *phys_raw };

            let frame = phys.allocate_frame().expect("Physical OOM!!!");

            closure(phys, frame, el);
        }
    };
}

#[macro_export]
macro_rules! pfree {
    ($frame:expr) => {
        $crate::mem::PHYS_ALLOCATOR.lock().as_mut().expect("Allocator missing!!!").deallocate_frame($frame)
    };
}

#[macro_export]
macro_rules! map {
    ($page:expr, $frame:expr, $flags:expr) => {
        unsafe {
            ::x86_64::structures::paging::mapper::Mapper::map_to(
                $crate::mem::VIRT_MAPPER.lock().as_mut().expect("Mapper missing!!!"),
                $page,
                $frame,
                $flags,
                $crate::mem::PHYS_ALLOCATOR.lock().as_mut().expect("Allocator missing!!!")
            ).expect("Mapping failed!!!").flush()
        }
    };
}

#[macro_export]
macro_rules! map_range {
    ($pages:expr, $flags:expr) => {
        let (pages, flags) = ($pages, $flags);

        let mut map_guard = $crate::mem::VIRT_MAPPER.lock();
        let map = map_guard.as_mut().expect("Mapper missing!!!");
        let map_raw = &raw mut *map;

        $crate::palloc_loop!(pages, |palloc, frame, page| {
            // SAFETY: STILL LOCKED
            let map = unsafe { &mut *map_raw };

            unsafe { ::x86_64::structures::paging::mapper::Mapper::map_to(map, page, frame, flags, palloc).expect("Mapping failed!!!").flush() }
        })
    };
}

#[macro_export]
macro_rules! unmap {
    ($page:expr) => {
        {
            let (frame, flush) = ::x86_64::structures::paging::Mapper::unmap($crate::mem::VIRT_MAPPER.lock().as_mut().expect("Mapper missing!!!"), $page).expect("Unmapping failed!!!");
            flush.flush();
            frame
        }
    };
}

#[macro_export]
macro_rules! unmap_clean {
    ($page:expr) => {
        unsafe {
            let local_page = $page;
            let mut mapper_guard = $crate::mem::VIRT_MAPPER.lock();
            let mapper = mapper_guard.as_mut().expect("Mapper missing!!!");
            let (frame, flush) = ::x86_64::structures::paging::Mapper::unmap(mapper, local_page).expect("Unmapping failed!!!");
            ::x86_64::structures::paging::mapper::CleanUp::clean_up_addr_range(
                mapper,
                Page::range_inclusive($page, $page),
                $crate::mem::PHYS_ALLOCATOR.lock().as_mut().expect("Allocator missing!!!")
            );
            flush.flush();
            frame
        }
    };
}

#[macro_export]
macro_rules! remap {
    ($page:expr, $frame:expr, $flags:expr) => {
        let (page, frame, flags) = ($page, $frame, $flags);
        $crate::unmap!(page);
        $crate::map!(page, frame, flags)
    };
}

/// SAFETY: MEMORY REGIONS MUST BE VALID AND LATER UNUSED
pub unsafe fn init(memory_regions: &mut MemoryRegions) {
    // SAFETY: MEMORY REGIONS ARE VALID AND LATER UNUSED
    *PHYS_ALLOCATOR.lock() = Some(unsafe { PageFrameAllocator::new(memory_regions) });

    // SAFETY: l4table IS ONLY CALLED HERE AND IS VALID (OFFSET IS ALSO VALID)
    *VIRT_MAPPER.lock() = Some(unsafe { OffsetPageTable::new(l4table(), VirtAddr::from_ptr(OFFSET as *const ())) });

    let heap_range = Page::<Size4KiB>::range_inclusive(
        Page::containing_address(VirtAddr::from_ptr(HEAP_VIRT_BASE as *const ())),
        Page::containing_address(VirtAddr::from_ptr((0u64.wrapping_sub(Size4KiB::SIZE)) as *const ()))
    );

    {
        let mut mapper_guard = VIRT_MAPPER.lock();
        let mapper = mapper_guard.as_mut().unwrap();

        let start4 = heap_range.start.p4_index();
        let end4 = heap_range.end.p4_index();

        //TODO: BETTER CHECK
        // Reserved for kernel heap
        assert!(mapper.level_4_table().iter().skip(start4.into()).take(usize::from(end4) - usize::from(start4)).all(|entry| entry.flags().intersects(PageTableFlags::PRESENT)), "Level 4 entry present in Kernel Heap!!!");

        //TODO: MAKE CONST
        // Reserved for user
        assert!(mapper.level_4_table()[42].is_unused())
        //TODO: THIS
        //mapper.level_4_table().iter().find(|e| e.is_unused());
    }

    VIRT_ALLOCATOR.init();

    let size = PHYS_ALLOCATOR.lock().as_ref().unwrap().size();
    let free = PHYS_ALLOCATOR.lock().as_ref().unwrap().free();
    debug!("Usable memory 0x{:016x} physical bytes (0x{:016x} used)", size, size - free);
    assert!(free > MIN_PHYSICAL_FREE, "Not enough physical memory 0x{:x} free < 0x{:x} required!!!", free, MIN_PHYSICAL_FREE);
}

pub const CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.kernel_stack_size = STACK_SIZE as u64;
    config.mappings.physical_memory = Some(Mapping::FixedAddress(OFFSET));
    config.mappings.dynamic_range_start = Some(OFFSET);
    config.mappings.dynamic_range_end = Some(0u64.wrapping_sub(HEAP_VIRT_SIZE as u64));
    config
};

/// SAFETY: REFERENCE MUST BE USED WITH PAGE TABLE AND MULTIPLE MUTABLE REFERENCES IN MIND
unsafe fn l4table() -> &'static mut PageTable {
    // SAFETY: PAGE TABLE IS VALID (OTHERWISE A PAGE FAULT WOULD HAVE TRIPLE FAULTED ALREADY)
    unsafe { &mut *(Cr3::read().0.start_address().as_u64().add(OFFSET) as *mut PageTable) }
}

