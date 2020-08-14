use alloc::boxed::Box;
use core::ops::{
    Index,
    IndexMut,
};

/// The size of the stacks in the IST in bytes
pub const STACK_SIZE: usize = 4096;

type InterruptStackPtr = Option<Box<InterruptStack>>;

#[repr(transparent)]
pub struct InterruptStackTable([InterruptStackPtr; 7]);

impl InterruptStackTable {
    pub fn new() -> Self {
        let mut table = [None; 7];

        for i in 0..7 {
            table[i] = Some(Box::new(InterruptStack::new()));
        }

        Self(table)
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct InterruptStack([u8; STACK_SIZE]);

impl InterruptStack {
    pub fn new() -> Self {
        Self([0; STACK_SIZE])
    }
}

impl Index<usize> for InterruptStackTable {
    type Output = InterruptStackPtr;

    fn index(&self, index: usize) -> &Self::Output {
        &(self.0)[index]
    }
}

impl IndexMut<usize> for InterruptStackTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut (self.0)[index]
    }
}