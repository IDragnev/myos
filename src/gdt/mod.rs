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

#[allow(dead_code)]
struct TaskStateSegmentWithStacks {
    interrupt_stacks: ist::InterruptStackTable,
    tss: TaskStateSegment,
}

lazy_static! {
    static ref TSS: TaskStateSegmentWithStacks = {
        let interrupt_stacks = ist::InterruptStackTable::new();
        let mut tss = TaskStateSegment::new();

        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            let stack = interrupt_stacks[0]
                        .as_deref()
                        .expect("Empty InterruptStackTable entry");
            let stack_start = VirtAddr::from_ptr(stack);
            let stack_end = stack_start + ist::STACK_SIZE;

            stack_end
        };

        TaskStateSegmentWithStacks {
            interrupt_stacks,
            tss,
        }
    };
}

lazy_static! {
    static ref GDT: GdtWithSelectors = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector  = gdt.add_entry(Descriptor::tss_segment(&TSS.tss));
        
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