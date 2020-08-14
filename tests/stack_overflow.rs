#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use lazy_static::lazy_static;
use myos::{
    gdt,
    serial_print,
    serial_println,
    QemuExitCode,
    exit_qemu,
};
use x86_64::structures::idt::{
    InterruptDescriptorTable,
    InterruptStackFrame,
};

use bootloader::{
    BootInfo,
    entry_point
};


#[cfg(test)]
entry_point!(main);

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        unsafe {
            idt.double_fault
               .set_handler_fn(test_double_fault_handler)
               .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

extern "x86-interrupt"
fn test_double_fault_handler(_: &mut InterruptStackFrame, _: u64) -> ! {
    serial_println!("[ok]");

    exit_qemu(QemuExitCode::Success);

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    myos::test_panic_handler(info)
}

#[no_mangle]
fn main(boot_info: &'static BootInfo) -> ! {
    serial_print!("stack_overflow::stack_overflow...\t");

    myos::init_heap(boot_info);
    myos::gdt::init();
    init_test_idt();

    stack_overflow();

    panic!("Execution continued after stack overflow");
}

fn init_test_idt() {
    TEST_IDT.load();
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow();

    // prevent tail recursion optimizations
    volatile::Volatile::new(0).read();
}
