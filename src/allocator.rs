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
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

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