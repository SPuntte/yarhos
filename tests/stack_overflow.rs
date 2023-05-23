#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use bootloader::{entry_point, BootInfo};
use core::{ops::Deref, panic::PanicInfo};
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

entry_point!(_test_start);

#[no_mangle]
pub fn _test_start(_boot_info: &'static BootInfo) -> ! {
    yarhos::serial_print!("stack_overflow::stack_overflow...\t");

    yarhos::gdt::init();
    init_test_idt();

    stack_overflow();

    panic!("Execution continued after stack overflow");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    yarhos::test_panic_handler(info)
}

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    yarhos::serial_println!("[ok]");
    yarhos::exit_qemu(yarhos::QemuExitCode::Success);
    yarhos::halt();
}

// Define a test-specific interrupt descriptor table to catch a double fault induced by a kernel
// stack overflow
lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        // SAFETY: yarhos::gdt::DOUBLE_FAULT_IST_INDEX is valid and unique
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(yarhos::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

struct Integer<T>(T);

impl<T> Deref for Integer<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow();
    // Prevent tail call optimization
    volatile::Volatile::new(Integer(0)).read();
}
