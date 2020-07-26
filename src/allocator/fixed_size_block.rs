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
        match free_list_index(&layout) {
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
        match free_list_index(&layout) {
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
}

/// Choose an appropriate free list for the given layout.
///
/// Returns an index into the `BLOCK_LAYOUTS` array.
fn free_list_index(layout: &Layout) -> Option<usize> {
    BLOCK_LAYOUTS
    .iter()
    .position(|block| {
        layout.size()  <= block.size &&
        layout.align() <= block.align
    })
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, block_ptr: *mut u8, layout: Layout) {
        self.lock().dealloc(block_ptr, layout)
    }
}