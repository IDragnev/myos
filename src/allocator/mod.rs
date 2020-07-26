use x86_64::{
    VirtAddr,
    structures::{
        paging::{
            mapper::MapToError,
            FrameAllocator,
            Mapper,
            Page,
            PageTableFlags,
            Size4KiB,
            page::PageRangeInclusive,
        },
    },
};

pub mod bump;
pub mod fixed_size_block;

use fixed_size_block::FixedSizeBlockAllocator;

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::empty());

/// The start of the region of Virtual Memory allocated for the Heap
const HEAP_START: usize = 0x_4444_4444_0000;

/// The size of the Heap in bytes
pub const HEAP_SIZE: usize = 100 * 1024;

pub fn init_heap<M, F>(mapper: &mut M, frame_allocator: &mut F) -> Result<(), MapToError<Size4KiB>>
where 
    M: Mapper<Size4KiB>,
    F: FrameAllocator<Size4KiB>,
{
    for page in heap_region_pages() {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            let fl = mapper.map_to(page, frame, flags, frame_allocator)?;
            fl.flush();
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
    
    Ok(())
}

fn heap_region_pages() -> PageRangeInclusive<Size4KiB> {
    let heap_start = VirtAddr::new(HEAP_START as u64);
    let heap_end = heap_start + HEAP_SIZE - 1u64;
    let start_page = Page::containing_address(heap_start);
    let end_page = Page::containing_address(heap_end);

    Page::range_inclusive(start_page, end_page)
}

/// A wrapper around spin::Mutex to permit trait implementations.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn align_up_with_already_aligned_address() {
        let align = 4096;
        let addr = 4 * align;

        assert!(addr == align_up(addr, align));
    }

    #[test_case]
    fn align_up_with_non_aligned_address() {
        let align = 4096;
        let addr = align + 100;

        assert!(align_up(addr, align) == 2 * align);
    }
}