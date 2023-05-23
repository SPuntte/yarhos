#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yarhos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use yarhos::{println, vga_buffer};

entry_point!(_kernel_entry_point);

#[no_mangle]
#[allow(unreachable_code)]
pub fn _kernel_entry_point(_boot_info: &'static BootInfo) -> ! {
    use vga_buffer::{Color, ControlCharMode};

    vga_buffer::set_color(Color::LightCyan, Color::Black);
    // TODO: code page 437 conversion from Unicode
    println!("Hello, World{}", "! ö");

    // Set up IDT
    yarhos::init();

    #[allow(dead_code)]
    #[allow(unconditional_recursion)]
    fn stack_overflow() {
        stack_overflow();
    }

    // Uncomment for a stack overflow
    //stack_overflow();

    vga_buffer::set_color(Color::LightGreen, Color::DarkGray);
    vga_buffer::print_character_set();

    vga_buffer::set_fg_color(Color::Red);
    vga_buffer::set_control_mode(ControlCharMode::Glyph);
    println!("A\tB\rC\nD");

    vga_buffer::set_bg_color(Color::White);
    vga_buffer::set_control_mode(ControlCharMode::Control);
    println!("A\tB\rC\nD");

    // HACK: busy wait for some time
    for _ in 0..5_000_000 {
        x86_64::instructions::nop();
    }

    vga_buffer::set_color(Color::Pink, Color::Black);
    vga_buffer::clear();
    println!("Ohai :3");

    #[cfg(test)]
    test_main();

    println!("It did not crash :o");
    yarhos::halt()
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    yarhos::halt()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    yarhos::test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(true, true);
}
