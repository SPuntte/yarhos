#![no_std]
#![no_main]

mod vga_buffer;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(_kernel_entry_point);

#[no_mangle]
pub fn _kernel_entry_point(_boot_info: &'static BootInfo) -> ! {
    vga_buffer::print_something();

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
