#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(myos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{
    entry_point,
    BootInfo,
};
use core::panic::PanicInfo;
use alloc::{
    boxed::Box,
    vec::Vec,
};

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    myos::init(boot_info);

    test_main();

    myos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    myos::test_panic_handler(info)
}

#[test_case]
fn simple_allocations_are_handled() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);
    assert!(*heap_value_1 == 41);
    assert!(*heap_value_2 == 13);
}

#[test_case]
fn large_allocations_and_mulitple_reallocations_are_handled() {
    let n = 1_000;

    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }

    assert_eq!(
        vec.iter().sum::<u64>(),
        (n - 1) * n / 2
    );
}

#[test_case]
fn allocated_memory_is_freed_and_reused() {
    use myos::memory::HEAP_SIZE;

    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert!(*x == i);
    }
}