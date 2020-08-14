mod ist;

use x86_64::{
    VirtAddr,
    structures::{
        tss::TaskStateSegment,
        gdt::{
            GlobalDescriptorTable,
            Descriptor,
            SegmentSelector,
        },
    },
};
use lazy_static::lazy_static;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref IST: spin::Mutex<ist::InterruptStackTable> = spin::Mutex::new(ist::InterruptStackTable::new());
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            let table = IST.lock();

            let stack = table[0].as_deref().unwrap();
            let stack_start = VirtAddr::from_ptr(stack);
            let stack_end = stack_start + ist::STACK_SIZE;

            stack_end
        };

        tss
    };
}

lazy_static! {
    static ref GDT: GdtWithSelectors = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector  = gdt.add_entry(Descriptor::tss_segment(&TSS));
        
        GdtWithSelectors{
            gdt,
            tss_selector,
            code_selector, 
        }
    };
}

struct GdtWithSelectors {
    gdt: GlobalDescriptorTable,
    tss_selector: SegmentSelector,
    code_selector: SegmentSelector,
}

/// Sets up and loads the Global descriptor table
pub fn init() {
    use x86_64::instructions::{
        segmentation::set_cs,
        tables::load_tss,
    };

    GDT.gdt.load();
    unsafe {
        set_cs(GDT.code_selector);
        load_tss(GDT.tss_selector);
    }
}