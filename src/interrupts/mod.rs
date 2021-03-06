mod interrupt_index;

use x86_64::structures::idt::{
    InterruptDescriptorTable,
    InterruptStackFrame,
    PageFaultErrorCode,
};
use lazy_static::lazy_static;
use crate::{
    println,
    print,
    gdt,
    hlt_loop,
};
use pic8259_simple::{
    ChainedPics,
};
use interrupt_index::{
    InterruptIndex,
};

const PIC_1_OFFSET: u8 = 32;
const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe { 
            ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) 
        });

/// Initializes the chained Programmable Interrupt Controllers
pub fn init_pics() { 
    unsafe { 
        PICS.lock().initialize()
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
               .set_handler_fn(double_fault_handler)
               .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);

        idt[InterruptIndex::Timer.as_usize()]
           .set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()]
           .set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

/// Sets up and Loads the Interrupt descriptor table
pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt"
fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt"
fn double_fault_handler(stack_frame: &mut InterruptStackFrame, _error_code: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" 
fn timer_interrupt_handler(_: &mut InterruptStackFrame) {
    print!(".");

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt"
fn keyboard_interrupt_handler(_: &mut InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: spin::Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            spin::Mutex::new(
                Keyboard::new(
                    layouts::Us104Key,
                    ScancodeSet1,
                    HandleControl::Ignore,
                )
            );
    }

    let mut keyboard = KEYBOARD.lock();
    let mut ps2_data_port = Port::new(0x60);

    let scancode: u8 = unsafe { ps2_data_port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(c) => print!("{}", c),
                DecodedKey::RawKey(k)  => print!("{:?}", k),
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt"
fn page_fault_handler(stack_frame: &mut InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn breakpoint_exception_is_handled() {
        x86_64::instructions::interrupts::int3();
    }
}