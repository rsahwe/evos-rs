use core::{mem::MaybeUninit, ops::{Deref, DerefMut}, slice};

use bitvec::slice::BitSlice;
use bootloader_api::info::{MemoryRegion, MemoryRegionKind, MemoryRegions};
use x86_64::{structures::paging::{frame::PhysFrameRange, FrameAllocator, FrameDeallocator, PageSize, PhysFrame, Size4KiB}, PhysAddr, VirtAddr};

use crate::println;

use super::OFFSET;

struct SingleRegionPageFrameAllocator<'a> {
    frames: PhysFrameRange,
    bitmap: &'a mut BitSlice<u8>,
    next_free: Option<usize>,
}

impl SingleRegionPageFrameAllocator<'static> {
    /// SAFETY: MEMORYREGION MUST BE VALID AND USABLE AND AT LEAST EIGHT PAGES
    unsafe fn new(mut region: MemoryRegion) -> &'static mut Self {
        region.start = PhysAddr::new(region.start).align_up(Size4KiB::SIZE).as_u64();
        region.end = PhysAddr::new(region.end).align_down(Size4KiB::SIZE).as_u64();

        let start = VirtAddr::new(region.start + OFFSET);
        let size_in_pages = ((region.end - region.start) / Size4KiB::SIZE) as usize;
        let slice_size = size_in_pages / 8;
        let offset = (size_of::<SingleRegionPageFrameAllocator>() + slice_size + Size4KiB::SIZE as usize - 1) / Size4KiB::SIZE as usize;
        let this = start.as_mut_ptr::<MaybeUninit<Self>>();
        // SAFETY: OFFSET AND THIS IMPLEMENTATION GUARANTEES THAT THIS SLICE IS MAPPED AND UNIQUE
        let slice = unsafe { slice::from_raw_parts_mut(this.add(1).cast(), slice_size) };
        let bitmap = BitSlice::from_slice_mut(slice);
        bitmap.fill(false);

        // SAFETY: MEMORYREGION IS VALID AND USABLE
        let this = (unsafe { &mut *this }).write(SingleRegionPageFrameAllocator {
            next_free: None,
            bitmap,
            frames: PhysFrame::range(PhysFrame::containing_address(PhysAddr::new(region.start)), PhysFrame::containing_address(PhysAddr::new(region.end)))
        });
        
        this.bitmap[..offset].fill(true);

        this.next_free = this.bitmap.first_zero();

        this
    }

    fn allocate(&mut self) -> Option<PhysFrame> {
        self.next_free.map(|this| {
            self.bitmap.set(this, true);
            self.next_free = self.bitmap[this..].first_zero().map(|val| val + this);
            PhysFrame::from_start_address(PhysAddr::new(self.frames.start.start_address().as_u64() + Size4KiB::SIZE * this as u64)).unwrap()
        })
    }

    /// Returns true if page was deallocated, panics if page is deallocated already
    fn deallocate(&mut self, frame: PhysFrame) -> bool {
        let start = self.frames.start.start_address().as_u64();
        let end = self.frames.end.start_address().as_u64();
        let frame = frame.start_address().as_u64();

        if start <= frame && frame < end {
            let index = ((frame - start) / Size4KiB::SIZE) as usize;
            if !self.bitmap.get(index).unwrap() {
                panic!("Invalid frame index {} for region @ Phys 0x{:016x} deallocated in SingleRegionPageFrameAllocator!!!", index, start)
            } else {
                self.bitmap.set(index, false);
                match self.next_free {
                    Some(old) => if old > index { self.next_free = Some(index) },
                    None => self.next_free = Some(index),
                }
                true
            }
        } else {
            false
        }
    }

    fn size(&self) -> usize {
        self.frames.size() as usize
    }

    fn free(&self) -> usize {
        self.bitmap.count_zeros() * Size4KiB::SIZE as usize
    }
}

/// Static SingleRegionPageFrameAllocator padded reference holder that has the same size as MemoryRegion.
struct SSRPFAReferenceStruct {
    raw: &'static mut SingleRegionPageFrameAllocator<'static>,
    _pad: [u8; size_of::<MemoryRegion>() - size_of::<&'static mut SingleRegionPageFrameAllocator>()],
}

impl From<&'static mut SingleRegionPageFrameAllocator<'static>> for SSRPFAReferenceStruct {
    fn from(value: &'static mut SingleRegionPageFrameAllocator<'static>) -> Self {
        Self {
            raw: value,
            _pad: Default::default(),
        }
    }
}

impl Deref for SSRPFAReferenceStruct {
    type Target = SingleRegionPageFrameAllocator<'static>;

    fn deref(&self) -> &Self::Target {
        self.raw
    }
}

impl DerefMut for SSRPFAReferenceStruct {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.raw
    }
}

pub struct PageFrameAllocator {
    allocators: &'static mut [SSRPFAReferenceStruct],
}

impl PageFrameAllocator {
    /// SAFETY: MEMORYREGIONS MUST BE VALID AND REMAIN MAPPED
    pub unsafe fn new(regions: &mut MemoryRegions) -> Self {
        println!("DEBUG: PageFrameAllocator::new():");

        let raw = &mut *regions;
        let mut start = 0;
        let mut end = raw.len() - 1;

        while start != end {
            match raw[start].kind {
                MemoryRegionKind::Usable => {
                    if (PhysAddr::new(raw[start].end).align_down(Size4KiB::SIZE).as_u64() - PhysAddr::new(raw[start].start).align_up(Size4KiB::SIZE).as_u64()) / Size4KiB::SIZE >= 8 {
                        start += 1;
                    } else {
                        raw.swap(start, end);
                        end -= 1;
                    }
                },
                _ => {
                    raw.swap(start, end);
                    end -= 1;
                },
            }
        }
        
        // Both start and end are amount of regions
        let raw = &mut raw[..start];

        for region in raw.iter_mut() {
            println!("DEBUG:     MemReg [0x{:016x}-0x{:016x}]", region.start, region.end);
            let ptr = region as *mut MemoryRegion as *mut SSRPFAReferenceStruct;

            #[allow(unused)]
            static STATIC_TRANSMUTABLITY_CHECK: () = assert!(size_of::<MemoryRegion>() == size_of::<SSRPFAReferenceStruct>());

            let region = *region;

            // SAFETY: REGION IS VALID AND AT LEAST EIGHT PAGES LARGE
            let value = unsafe { SingleRegionPageFrameAllocator::new(region) };

            // SAFETY: POINTER IS VALID
            unsafe { ptr.write(value.into()) };
        }

        // SAFETY: SLICE IS ALREADY VALID AND INITIALIZED
        let raw = unsafe { slice::from_raw_parts_mut(raw.as_mut_ptr().cast::<SSRPFAReferenceStruct>(), raw.len()) };

        Self {
            allocators: raw,
        }
    }

    pub fn size(&self) -> usize {
        self.allocators.iter().fold(0, |acc, allocator| acc + allocator.size())
    }

    pub fn free(&self) -> usize {
        self.allocators.iter().fold(0, |acc, allocator| acc + allocator.free())
    }
}

// SAFETY: THE ALLOCATOR SHOULD BE SAFE
unsafe impl FrameAllocator<Size4KiB> for PageFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.allocators.iter_mut().find_map(|allocator| allocator.allocate())
    }
}

impl FrameDeallocator<Size4KiB> for PageFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame) {
        match self.allocators.iter_mut().find_map(|allocator| if allocator.deallocate(frame) { Some(()) } else { None } ) {
            Some(_) => (),
            None => panic!("Invalid frame @ Phys 0x{:016x} deallocated in PageFrameAllocator!!!", frame.start_address().as_u64()),
        }
    }
}
