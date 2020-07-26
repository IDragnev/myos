use super::{
    Locked,
};
use alloc::alloc::{
    Layout,
    GlobalAlloc,
};
use core::{
    ptr::{
        self,
        NonNull,
    },
    mem,
};

struct Node {
    next: Option<&'static mut Node>,
}

/// Represents the layout of a fixed-size block.
#[derive(Copy, Clone, Debug)]
struct BlockLayout {
    size: usize,
    align: usize,
}

/// A block layout table with an entry for the block class of each free list.
const BLOCK_LAYOUTS: &[BlockLayout] = &[
    BlockLayout{ size: 8, align: 8 },
    BlockLayout{ size: 16, align: 16 },
    BlockLayout{ size: 32, align: 32 },
    BlockLayout{ size: 64, align: 64 },
    BlockLayout{ size: 128, align: 128 },
    BlockLayout{ size: 256, align: 256 },
    BlockLayout{ size: 512, align: 512 },
    BlockLayout{ size: 1024, align: 1024 },
    BlockLayout{ size: 2048, align: 2048 },
];

/// The number of the free lists used by the allocator.
const FREE_LISTS_COUNT: usize = BLOCK_LAYOUTS.len();

pub struct FixedSizeBlockAllocator {
    free_list_heads: [Option<&'static mut Node>; FREE_LISTS_COUNT],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    /// Creates an empty allocator. All alloc calls will return null.
    pub const fn empty() -> Self {
        FixedSizeBlockAllocator {
            free_list_heads: [None; FREE_LISTS_COUNT],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Creates a new allocator with the given heap bounds.
    ///
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused.
    pub unsafe fn new(heap_start: usize, heap_size: usize) -> Self {
        let mut allocator = Self::empty();
        allocator.init(heap_start, heap_size);

        allocator
    }

    /// Initialize an empty allocator with the given heap bounds.
    ///
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once and on an empty allocator.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.fallback_allocator.init(heap_start, heap_size);
    }

    /// Allocates a block of memory with the required layout.
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.free_list_index(&layout) {
            Some(i) => self.free_list_alloc(i),
            None    => self.fallback_alloc(layout),
        }
    }

    /// Allocates a block using the corresponding free list
    /// or the fallback allocator in case that list is empty.
    ///
    /// Panics if index >= FREE_LISTS_COUNT
    fn free_list_alloc(&mut self, index: usize) -> *mut u8 {
        assert!(index < FREE_LISTS_COUNT);

        match self.free_list_heads[index].take() {
            Some(node) => {
                self.free_list_heads[index] = node.next.take();

                node as *mut Node 
                     as *mut u8
            },
            None => {
                let block_layout = &BLOCK_LAYOUTS[index]; 
                let layout = Layout::from_size_align(block_layout.size, block_layout.align)
                             .unwrap();

                self.fallback_alloc(layout)
            }
        }
    }

     /// Allocates a block using the fallback allocator.
     fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        self.fallback_allocator
            .allocate_first_fit(layout)
            .map(|ptr| ptr.as_ptr())
            .unwrap_or(ptr::null_mut())
    }

    /// Frees the given block of memory.
    ///
    /// block_ptr must be a pointer returned by a call to the alloc function with identical layout.
    /// Undefined behavior may occur for invalid arguments, thus this function is unsafe.
    pub unsafe fn dealloc(&mut self, block_ptr: *mut u8, layout: Layout) {
        if block_ptr == ptr::null_mut() {
            return;
        }

        match self.free_list_index(&layout) {
            Some(index) => {
                assert!(mem::size_of::<Node>() <= BLOCK_LAYOUTS[index].size);
                assert!(mem::align_of::<Node>() <= BLOCK_LAYOUTS[index].align);

                let old_head = self.free_list_heads[index].take();
                let new_head = block_ptr as *mut Node;
                new_head.write(Node {
                    next: old_head,
                });

                self.free_list_heads[index] = Some(&mut *new_head);
            }
            None => {
                let block_ptr = NonNull::new(block_ptr).unwrap();
                self.fallback_allocator.deallocate(block_ptr, layout);
            }
        }
    }

    /// Choose an appropriate free list for the given layout.
    fn free_list_index(&self, layout: &Layout) -> Option<usize> {
        let heap_size = self.fallback_allocator.size(); 

        BLOCK_LAYOUTS
        .iter()
        .filter(|block| {
            block.size < heap_size
        })
        .position(|block| {
            layout.size()  <= block.size &&
            layout.align() <= block.align
        })
    }
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, block_ptr: *mut u8, layout: Layout) {
        self.lock().dealloc(block_ptr, layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn empty_allocator_always_returns_null() {
        let mut allocator = FixedSizeBlockAllocator::empty();
        let layout = Layout::from_size_align(14, 8).unwrap();

        assert!(allocator.alloc(layout) == ptr::null_mut());
    }

    #[test_case]
    fn alloc_with_too_big_size_returns_null() {
        let mut buffer = [0; 256];
        let heap_start: *mut u8 = buffer.as_mut_ptr();
        let mut allocator = unsafe {
            FixedSizeBlockAllocator::new(heap_start as usize, buffer.len())
        };
        let layout = Layout::from_size_align(2 * buffer.len(), 8).unwrap();

        assert!(allocator.alloc(layout) == ptr::null_mut());
    }

    #[test_case]
    fn alloc_with_fittable_size_succeeds() {
        let mut buffer = [0; 256];
        let heap_start: *mut u8 = buffer.as_mut_ptr();
        let mut allocator = unsafe {
            FixedSizeBlockAllocator::new(heap_start as usize, buffer.len())
        };
        let layout = Layout::from_size_align(buffer.len() / 2, 8).unwrap();

        assert!(allocator.alloc(layout) != ptr::null_mut());
    }

    #[test_case]
    fn different_allocations_return_different_blocks() {
        let mut buffer = [0; 256];
        let heap_start: *mut u8 = buffer.as_mut_ptr();
        let mut allocator = unsafe {
            FixedSizeBlockAllocator::new(heap_start as usize, buffer.len())
        };
        let layout = Layout::from_size_align(4, 8).unwrap();

        let first_block  = allocator.alloc(layout);
        let second_block = allocator.alloc(layout);

        assert!(first_block != ptr::null_mut());
        assert!(second_block != ptr::null_mut());
        assert!(first_block != second_block);
    }

    #[test_case]
    fn deallocated_memory_can_be_reused() {
        let mut buffer = [0; 256];
        let heap_start: *mut u8 = buffer.as_mut_ptr();
        let mut allocator = unsafe {
            FixedSizeBlockAllocator::new(heap_start as usize, buffer.len())
        };
        let layout = Layout::from_size_align(150, 8).unwrap();

        let block = allocator.alloc(layout);
        assert!(block != ptr::null_mut());
        unsafe {
            allocator.dealloc(block, layout);
        }
        let block = allocator.alloc(layout);
        assert!(block != ptr::null_mut());
    }
}