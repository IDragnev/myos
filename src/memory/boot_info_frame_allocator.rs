use x86_64::{
    PhysAddr,
    structures::{
        paging::{
            PhysFrame,
            FrameAllocator,
            Size4KiB,
        },
    },
};
use bootloader::{
    bootinfo::{
        MemoryMap,
        MemoryRegionType,
    },
};
use super::{
    PAGE_SIZE,
};

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        self.memory_map.iter()
            .filter(|region| {
                region.region_type == MemoryRegionType::Usable
            })
            .map(|region| {
                region.range.start_addr()..region.range.end_addr()
            })
            .flat_map(|address_range| {
                //all usable regions are page-aligned by the bootloader
                address_range.step_by(PAGE_SIZE)
            })
            .map(|frame_address| {
                PhysFrame::containing_address(
                    PhysAddr::new(frame_address)
                )
            })
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;

        frame
    }
}