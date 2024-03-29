#![no_std]
#![cfg_attr(test, no_main)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod gdt;
pub mod interrupts;
pub mod serial;
pub mod vga_buffer;

use core::panic::PanicInfo;

#[cfg(test)]
use bootloader::{entry_point, BootInfo};

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failure = 0x11,
}

/// the I/O port number for the QEMU debug exit device.
const QEMU_ISA_DEBUG_EXIT_PORT: u16 = 0xF4;

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(QEMU_ISA_DEBUG_EXIT_PORT);
        port.write(exit_code as u32);
    }
}

/// Conserve energy by halting the CPU.
pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::interrupts::enable_and_hlt();
    }
}

/// Hang by disabling interrupts and invoking the HLT instruction.
pub fn hang() -> ! {
    x86_64::instructions::interrupts::without_interrupts(x86_64::instructions::hlt);
    unreachable!("Unexpected wakeup with interrupts disabled");
}

pub fn init() {
    gdt::init();
    interrupts::init_idt();
    interrupts::init_pics();
    x86_64::instructions::interrupts::enable();
}

#[cfg(test)]
entry_point!(_test_start);

#[cfg(test)]
#[no_mangle]
pub fn _test_start(_boot_info: &'static BootInfo) -> ! {
    init();
    test_main();
    halt_loop();
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_runner_should_panic(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
        serial_println!("[test did not panic]");
        exit_qemu(QemuExitCode::Failure);
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failure);
    hang();
}

pub fn test_panic_handler_should_panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    hang();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
