#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod vga_buffer;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(_kernel_entry_point);

#[no_mangle]
#[allow(unreachable_code)]
pub fn _kernel_entry_point(_boot_info: &'static BootInfo) -> ! {
    // The 'ö' here demostrates how non-ASCII characters are handled
    println!("Hello, World{}", "! ö");

    #[cfg(test)]
    test_main();

    loop {}
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    exit_qemu(QemuExitCode::Success);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
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

#[test_case]
fn trivial_assertion() {
    print!("trivial assertion... ");
    assert_eq!(true, true);
    println!("[ok]");
}
