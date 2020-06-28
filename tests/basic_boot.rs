#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(myos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use myos::println;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    myos::test_panic_handler(info)
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[test_case]
fn println_works_right_after_booting() {
    println!("output");
}