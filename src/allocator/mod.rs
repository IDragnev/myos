pub mod fixed_size_block;

use fixed_size_block::FixedSizeBlockAllocator;

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::empty());

/// Initializes the global allocator with the given mapped Heap region.
/// 
/// ## Safety
///
/// The function is unsafe because the caller must guarantee that
/// the given Heap region is already mapped and is not used anywhere else.
pub unsafe fn init_heap(heap_start: usize, heap_size: usize) {
    ALLOCATOR.lock().init(heap_start, heap_size);
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