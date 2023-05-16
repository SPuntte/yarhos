#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod serial;
mod vga_buffer;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(_kernel_entry_point);

/// Conserve energy by halting the CPU
#[allow(unreachable_code)]
fn halt() -> ! {
    x86_64::instructions::hlt();
    unreachable!("Unexpected wakeup.");
    loop {}
}

#[no_mangle]
#[allow(unreachable_code)]
pub fn _kernel_entry_point(_boot_info: &'static BootInfo) -> ! {
    use vga_buffer::{Color, ControlCharMode};

    vga_buffer::set_color(Color::LightCyan, Color::Black);
    // TODO: code page 437 conversion from Unicode
    println!("Hello, World{}", "! ö");

    vga_buffer::set_color(Color::LightGreen, Color::DarkGray);
    vga_buffer::print_character_set();

    vga_buffer::set_fg_color(Color::Red);
    vga_buffer::set_control_mode(ControlCharMode::Glyph);
    println!("A\tB\rC\nD");

    vga_buffer::set_bg_color(Color::White);
    vga_buffer::set_control_mode(ControlCharMode::Control);
    println!("A\tB\rC\nD");

    #[cfg(test)]
    test_main();

    halt();
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failure);
    halt();
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
    assert_eq!(true, true);
}

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
