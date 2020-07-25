use super::{
    align_up,
    Locked,
};
use core::{
    mem,
    ptr,
};
use alloc::alloc::{
    GlobalAlloc,
    Layout,
};

struct Node {
    size: usize,
    next: Option<&'static mut Node>,
}

impl Node {
    fn start_addr(&self) -> usize {
        self as *const Node
             as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

enum AllocFromRegionErr {
    AddressOverflow,
    RegionNotBigEnough,
    ExcessCannotFitANode,
}

pub struct LinkedListAllocator {
    head: Node,
}

impl LinkedListAllocator {
    /// Creates an empty LinkedListAllocator.
    pub const fn new() -> Self {
        Self {
            head: Node {
                size: 0,
                next: None,
            }
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    /// Adds the given memory region to the front of the list.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        use mem::{size_of, align_of};

        assert!(size >= size_of::<Node>());
        assert!(align_up(addr, align_of::<Node>()) == addr);

        let node_ptr = addr as *mut Node;
        node_ptr.write(Node{
            size,
            next: self.head.next.take(),
        });

        self.head.next = Some(&mut *node_ptr)
    }

     /// Looks for a free region with the given size and alignment and removes
    /// it from the list.
    ///
    /// Returns a tuple of the list node and the start address of the allocation.
    fn find_region(&mut self, size: usize, align: usize)
        -> Option<(&'static mut Node, usize)>
    {
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            match Self::alloc_from_region(&region, size, align) {
                Ok(alloc_start) => {
                    let alloc_region = current.next.take().unwrap();
                    current.next = alloc_region.next.take();

                    return Some((alloc_region, alloc_start));
                },
                Err(_) => {
                    current = current.next.as_mut().unwrap();
                },
            }
        }

        None
    }

    /// Try to use the given region for an allocation with given size and
    /// alignment.
    ///
    /// Returns the allocation start address on success.
    fn alloc_from_region(region: &Node, size: usize, align: usize) 
      -> Result<usize, AllocFromRegionErr> 
    {
        let alloc_start = align_up(region.start_addr(), align);

        alloc_start
            .checked_add(size)
            .ok_or(AllocFromRegionErr::AddressOverflow)
            .and_then(|alloc_end| {
                if alloc_end <= region.end_addr() {
                    Ok(alloc_end)
                }
                else {
                    Err(AllocFromRegionErr::RegionNotBigEnough)
                }
            })
            .and_then(|alloc_end| {
                let excess_size = region.end_addr() - alloc_end;
                if excess_size > 0 && excess_size < mem::size_of::<Node>() {
                    Err(AllocFromRegionErr::ExcessCannotFitANode)
                }
                else {
                    Ok(alloc_start)
                }
            })
    }

    /// Adjust the given layout so that the resulting allocated memory
    /// region is also capable of storing a `Node`.
    ///
    /// Returns the adjusted size and alignment as a (size, align) tuple.
    fn adjust_layout(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<Node>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<Node>());
        
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::adjust_layout(layout);
        let mut allocator = self.lock();

        allocator
        .find_region(size, align)
        .map(|(region, alloc_start)| {
            let alloc_end = alloc_start.checked_add(size)
               .expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }

            alloc_start as *mut u8
        })
        .unwrap_or(ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::adjust_layout(layout);

        self.lock().add_free_region(ptr as usize, size)
    }
}