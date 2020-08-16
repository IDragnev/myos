mod boot_info_frame_allocator;

use boot_info_frame_allocator::BootInfoFrameAllocator;
use bootloader::BootInfo;
use x86_64::{
    VirtAddr,
    structures::{
        paging::{
            PageTable,
            OffsetPageTable,
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

/// The Page size in bytes
const PAGE_SIZE: usize = 4096;

/// The start of the region of Virtual Memory allocated for the Heap
pub const HEAP_START: usize = 0x_4444_4444_0000;

/// The size of the Heap in bytes
pub const HEAP_SIZE: usize = 100 * 1024;

/// Further sets up the Kernel virtual memory.
///
/// Maps the region allocated for the Heap to physical memory.
pub fn init(boot_info: &'static BootInfo) {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { 
        init_page_table_mapper(phys_mem_offset)
    };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    map_heap_to_physical_memory(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed");
}

/// Initialize a new OffsetPageTable.
///
/// ## Safety
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn init_page_table_mapper(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 table.
///
/// ## Safety
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable 
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

fn map_heap_to_physical_memory<M, F>(
    mapper: &mut M,
    frame_allocator: &mut F,
) -> Result<(), MapToError<Size4KiB>>
where 
    M: Mapper<Size4KiB>,
    F: FrameAllocator<Size4KiB>,
{
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let heap_pages = region_pages(
        VirtAddr::new(HEAP_START as u64),
        VirtAddr::new(
            (HEAP_START + HEAP_SIZE - 1) as u64
        ),
    );

    map_pages_to_physical_memory(
        mapper,
        frame_allocator,
        heap_pages,
        flags,
    )
}

/// Converts a virtual memory region to a range of its constituent pages
///
/// `end_address` is the last valid address of the region.
fn region_pages(start_address: VirtAddr, end_address: VirtAddr) -> PageRangeInclusive<Size4KiB> {
    let start_page = Page::containing_address(start_address);
    let end_page   = Page::containing_address(end_address);

    Page::range_inclusive(start_page, end_page)
}

/// Maps the given pages to physical memory.
///
/// For each page, the function allocates a new physical frame with the `frame_allocator`
/// and then uses the `map_to` function of the `mapper` to map the page to that frame with `flags` and `frame_allocator`.
fn map_pages_to_physical_memory<M, F>(
    mapper: &mut M,
    frame_allocator: &mut F,
    region: PageRangeInclusive<Size4KiB>,
    flags: PageTableFlags,
) -> Result<(), MapToError<Size4KiB>>
where 
    M: Mapper<Size4KiB>,
    F: FrameAllocator<Size4KiB>,
{
    for page in region {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;

        unsafe {
            let fl = mapper.map_to(page, frame, flags, frame_allocator)?;
            fl.flush();
        }
    }

    Ok(())
}