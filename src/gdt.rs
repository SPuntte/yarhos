use lazy_static::lazy_static;
use x86_64::{
    registers::segmentation::Segment,
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        // NOTE: tss.privilege_stack_table required for user mode
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            // NOTE: this stack has no guard page below it
            const STACK_SIZE: usize = 4096 * 5;
            // TODO: memory allocation
            // NOTE: `mut` needed in order for the bootloader to map the stach to a writable page
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            // SAFETY: compiler can't guarantee race freedom for `static mut` accesses
            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            #[allow(clippy::let_and_return)]
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}

#[derive(Debug, Clone, Copy)]
struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            },
        )
    };
}

pub fn init() {
    let (
        gdt,
        Selectors {
            code_selector,
            tss_selector,
        },
    ) = (&GDT.0, GDT.1);

    gdt.load();
    // SAFETY: selectors created using the x86_64 crate are valid
    unsafe {
        x86_64::instructions::segmentation::CS::set_reg(code_selector);
        x86_64::instructions::tables::load_tss(tss_selector);
    }
}
