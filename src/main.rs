#![no_std]
#![no_main]

mod vga_buffer;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(_kernel_entry_point);

#[no_mangle]
pub fn _kernel_entry_point(_boot_info: &'static BootInfo) -> ! {
    use core::fmt::Write;
    let writer = &vga_buffer::WRITER;
    writer.lock().write_byte(b'H');
    writer.lock().write_string("ello from '_kernel_entry_point()' B)\n");
    writeln!(writer.lock(), "I can haz {} cheezburgerz?", 42).unwrap();

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
