use core::{alloc::{GlobalAlloc, Layout}, marker::PhantomData, mem::{ManuallyDrop, MaybeUninit}, ops::{Deref, DerefMut}, ptr::NonNull};

use bitvec::array::BitArray;
use linked_list_allocator::Heap;
use spin::Mutex;
use x86_64::{structures::paging::{FrameAllocator, FrameDeallocator, Page, PageSize, PageTableFlags, PhysFrame, Size4KiB}, VirtAddr};

use crate::{map_range, palloc, pfree};

use super::{HEAP_BLOCK_SIZE, HEAP_VIRT_BASE, OFFSET};

pub struct VirtFrame<T> {
    phys: PhysFrame,
    _phantom: PhantomData<T>,
}

impl<T> VirtFrame<T> {
    pub fn new(element: T) -> Self {
        assert!(size_of::<T>() <= Size4KiB::SIZE as usize);

        let mut frame = VirtFrame {
            phys: palloc!(),
            _phantom: PhantomData,
        };

        // SAFETY: POINTER IS VALID
        unsafe { &mut *(&mut *frame as *mut T as *mut MaybeUninit<T>) }.write(element);

        frame
    }

    #[allow(dead_code)]
    pub fn new_init<U>(init: U) -> Self
    where 
        for<'a> (U, &'a Self): Into<T>,    
    {
        assert!(size_of::<T>() <= Size4KiB::SIZE as usize);

        let mut frame = VirtFrame {
            phys: palloc!(),
            _phantom: PhantomData,
        };

        // SAFETY: POINTER IS VALID
        unsafe { &mut *(&mut *frame as *mut T as *mut MaybeUninit<T>) }.write((init, &frame).into());

        frame
    }

    pub fn new_default() -> Self
    where 
        T: Default  
    {
        assert!(size_of::<T>() <= Size4KiB::SIZE as usize);

        let mut frame = VirtFrame {
            phys: palloc!(),
            _phantom: PhantomData,
        };

        // SAFETY: POINTER IS VALID
        unsafe { &mut *(&mut *frame as *mut T as *mut MaybeUninit<T>) }.write(Default::default());

        frame
    }

    fn into_inner(self) -> T {
        let this = ManuallyDrop::new(self);
        // SAFETY: FRAME IS MAPPED, ALLOCATED AND LARGE ENOUGH
        unsafe { VirtAddr::new(this.phys.start_address().as_u64() + OFFSET).as_mut_ptr::<T>().read() }
    }

    #[allow(dead_code)]
    pub fn leak(self) -> &'static mut T {
        // SAFETY: FRAME IS MAPPED, ALLOCATED AND LARGE ENOUGH
        let res = unsafe { &mut *VirtAddr::new(self.phys.start_address().as_u64() + OFFSET).as_mut_ptr::<T>() };

        // Make sure inner does not get dropped and the page does not get deallocated
        let _drop = ManuallyDrop::new(self);

        res
    }
}

impl<T> Default for VirtFrame<T>
where
    T: Default
{
    fn default() -> Self {
        Self::new_default()
    }
}

impl<T> Drop for VirtFrame<T> {
    fn drop(&mut self) {
        // Incase T has drop glue
        // SAFETY: FRAME IS MAPPED, ALLOCATED AND LARGE ENOUGH
        let _drop = unsafe { VirtAddr::new(self.phys.start_address().as_u64() + OFFSET).as_mut_ptr::<T>().read_volatile() };
        // SAFETY: ALLOCATED BY THIS ALLOCATOR
        unsafe { pfree!(self.phys) };
    }
}

impl<T> Deref for VirtFrame<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: FRAME IS MAPPED, ALLOCATED AND LARGE ENOUGH
        unsafe { &*VirtAddr::new(self.phys.start_address().as_u64() + OFFSET).as_ptr() }
    }
}

impl<T> DerefMut for VirtFrame<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: FRAME IS MAPPED, ALLOCATED AND LARGE ENOUGH
        unsafe { &mut *VirtAddr::new(self.phys.start_address().as_u64() + OFFSET).as_mut_ptr() }
    }
}

const BITARRAY_MAX: usize = 16; // 4096 / 32

struct Slab {
    size: usize,
    first: Option<VirtFrame<SlabElementSlab>>,
}

impl Slab {
    fn new(size: usize) -> Self {
        Self { size, first: None }
    }

    fn allocate(&mut self) -> *mut u8 {
        let mut current_slab_el_slab = &mut self.first;

        loop {
            match current_slab_el_slab {
                some @ Some(_) => {
                    let inner = some.as_mut().unwrap();

                    if inner.length < inner.elements.len() {
                        return {
                            let el = inner.not_full_or_push(self.size);
                            el.alloc(self.size)
                        }
                    } else {
                        match inner.find_not_full(self.size) {
                            Some(el) => return el.alloc(self.size),
                            None => current_slab_el_slab = &mut inner.next,
                        }
                    }
                },
                none @ None => {
                    none.replace(Default::default());
                    let inner = none.as_mut().unwrap();
                    let el = inner.push();
                    return el.alloc(self.size)
                },
            }
        }
    }

    fn try_deallocate(&mut self, ptr: *mut u8) -> bool {
        let mut current_slab_el_slab = &mut self.first;

        loop {
            match current_slab_el_slab {
                some @ Some(_) => {
                    if some.as_mut().unwrap().try_deallocate(ptr, self.size) {
                        if some.as_ref().unwrap().length == 0 {
                            let old = some.take();
                            let old = old.unwrap().into_inner();
                            let next = old.next;
                            *some = next;
                        }
                        return true
                    } else {
                        current_slab_el_slab = &mut some.as_mut().unwrap().next;
                    }
                },
                None => return false,
            }
        }
    }
}

struct SlabElementSlab {
    elements: [MaybeUninit<SlabElement>; (Size4KiB::SIZE as usize - size_of::<Option<VirtFrame<SlabElementSlab>>>() - size_of::<usize>() * 2) / size_of::<MaybeUninit<SlabElement>>()],
    length: usize,
    _pad: usize,
    next: Option<VirtFrame<SlabElementSlab>>,
}

impl SlabElementSlab {
    fn push(&mut self) -> &mut SlabElement {
        assert!(self.elements.len() > self.length);

        let el = &mut self.elements[self.length];
        self.length += 1;
        let el = el.write(Default::default());

        el
    }

    fn not_full_or_push(&mut self, size: usize) -> &mut SlabElement {
        assert!(self.elements.len() > self.length);

        // SAFETY: ELEMENT IS VALID
        match (&mut self.elements[..self.length]).iter_mut().enumerate().map(|(index, el)| unsafe { (index, el.assume_init_ref()) }).find(|(_, el)| !el.full(size)).map(|(index, _)| index) {
            // SAFETY: INDEX IS VALID
            Some(index) => unsafe { self.elements[index].assume_init_mut() },
            None => self.push(),
        }
    }

    fn find_not_full(&mut self, size: usize) -> Option<&mut SlabElement> {
        // SAFETY: ELEMENT IS VALID
        (&mut self.elements[..self.length]).iter_mut().map(|el| unsafe { el.assume_init_mut() }).find(|el| !el.full(size))
    }

    fn try_deallocate(&mut self, ptr: *mut u8, size: usize) -> bool {
        // SAFETY: ELEMENT IS VALID
        if (&mut self.elements[..self.length]).iter_mut().map(|el| unsafe { el.assume_init_mut() }).find_map(|el| if el.try_deallocate(ptr, size) { Some(()) } else { None }).is_some() {
            // SAFETY: ELEMENT IS VALID
            if (&mut self.elements[..self.length]).iter().map(|el| unsafe { el.assume_init_ref() }).all(|el| el.empty(size)) {
                for el in (&mut self.elements[..self.length]).iter_mut() {
                    // SAFETY: ELEMENT IS VALID
                    unsafe { el.assume_init_drop() };
                }
                self.length = 0;
            }
            true
        } else {
            false
        }
    }
}

impl Default for SlabElementSlab {
    fn default() -> Self {
        Self {
            elements: [const { MaybeUninit::uninit() }; (Size4KiB::SIZE as usize - size_of::<Option<VirtFrame<SlabElementSlab>>>() - size_of::<usize>() * 2) / size_of::<MaybeUninit<SlabElement>>()],
            length: Default::default(),
            _pad: Default::default(),
            next: Default::default(),
        }
    }
}

struct SlabElement {
    data: VirtFrame<[u8; Size4KiB::SIZE as usize]>,
    bitmap: BitArray<[u8; BITARRAY_MAX]>,
}

impl Default for SlabElement {
    fn default() -> Self {
        Self { data: VirtFrame::new([0; Size4KiB::SIZE as usize]), bitmap: Default::default() }
    }
}

impl SlabElement {
    #[allow(dead_code)]
    fn new() -> Self {
        Default::default()
    }

    fn full(&self, size: usize) -> bool {
        (&self.bitmap[..(self.data.len() / size)]).all()
    }

    fn empty(&self, size: usize) -> bool {
        (&self.bitmap[..(self.data.len() / size)]).not_any()
    }

    fn alloc(&mut self, size: usize) -> *mut u8 {
        match (&self.bitmap[..(self.data.len() / size)]).first_zero() {
            Some(index) => {
                self.bitmap.set(index, true);
                &raw mut self.data[(index * size)..((index + 1) * size)] as *mut u8
            },
            None => panic!("SlabElement was empty when alloc was called!!!"),
        }
    }

    fn try_deallocate(&mut self, ptr: *mut u8, size: usize) -> bool {
        if self.empty(size) {
            false
        } else {
            let offset = isize::wrapping_sub(ptr as isize, self.data.as_ptr() as isize);
            if offset < 0 {
                false
            } else if offset as usize >= self.data.len() {
                false
            } else {
                let index = offset as usize / size;
                if !self.bitmap.get(index).unwrap() {
                    panic!("Double free in SlabElement try_deallocate!!!");
                }
                self.bitmap.set(index, false);
                true
            }
        }
    }
}

struct KAlloc {
    slabs: [Slab; 8],
    big: Heap,
}

impl KAlloc {
    fn new() -> Self {
        let new_bottom = HEAP_VIRT_BASE as *mut u8;

        Self::map_block(new_bottom);

        Self {
            slabs: [
                Slab::new(32),
                Slab::new(64),
                Slab::new(128),
                Slab::new(256),
                Slab::new(512),
                Slab::new(1024),
                Slab::new(2048),
                Slab::new(4096),
            ],
            big: unsafe { Heap::new(new_bottom, HEAP_BLOCK_SIZE) }
        }
    }

    fn allocate_big(&mut self, layout: Layout) -> *mut u8 {
        let mut res = self.big.allocate_first_fit(layout);

        while res.is_err() {
            Self::map_block(self.big.bottom().wrapping_add(self.big.size()));
            // SAFETY: MAPPED AND UNIQUE
            unsafe { self.big.extend(HEAP_BLOCK_SIZE) };

            res = self.big.allocate_first_fit(layout);
        }

        res.unwrap().as_ptr()
    }

    fn deallocate_big(&mut self, ptr: *mut u8, layout: Layout) {
        // SAFETY: PTR IS VALID AND ALLOCATED BY THIS
        unsafe { self.big.deallocate(NonNull::new_unchecked(ptr), layout) }
    }

    fn map_block(new_bottom: *mut u8) {
        let new_top = new_bottom.wrapping_add(HEAP_BLOCK_SIZE);

        assert!(!new_top.is_null(), "Kernel Big Heap OOM!!!");

        let range = Page::<Size4KiB>::range(Page::from_start_address(VirtAddr::from_ptr(new_bottom)).unwrap(), Page::from_start_address(VirtAddr::from_ptr(new_top)).unwrap());

        map_range!(range, PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::GLOBAL);
    }
}

pub struct GAlloc {
    inner: Mutex<Option<KAlloc>>,
}

impl GAlloc {
    pub const fn new() -> Self {
        Self { inner: Mutex::new(None) }
    }

    pub fn init(&self) {
        let alloc = KAlloc::new();

        self.inner.lock().replace(alloc);
    }
}

unsafe impl GlobalAlloc for GAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut lock = self.inner.lock();
        let alloc = lock.as_mut().expect("GlobalAlloc missing!!!");

        let pow2 = layout.size().next_power_of_two();
        if pow2 <= 4096 {
            alloc.slabs[pow2.ilog2().saturating_sub(32usize.ilog2()) as usize].allocate()
        } else {
            alloc.allocate_big(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut lock = self.inner.lock();
        let alloc = lock.as_mut().expect("GlobalAlloc missing!!!");

        let pow2 = layout.size().next_power_of_two();
        if pow2 <= 4096 {
            assert!(alloc.slabs[pow2.ilog2().saturating_sub(32usize.ilog2()) as usize].try_deallocate(ptr), "Double free for GAlloc!!!");
        } else {
            alloc.deallocate_big(ptr, layout)
        }
    }
}
