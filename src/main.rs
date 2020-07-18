#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(myos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use myos::println;
use core::panic::PanicInfo;
use bootloader::{
    BootInfo,
    entry_point
};

entry_point!(kernel_main);

fn kernel_main(_: &'static BootInfo) -> ! {
    myos::init();

    println!("Welcome to myos!");

    #[cfg(test)]
    test_main();

    myos::hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    myos::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    myos::test_panic_handler(info)
}