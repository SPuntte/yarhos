#![no_std]
#![no_main]

mod vga_buffer;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(_kernel_entry_point);

#[no_mangle]
#[allow(unreachable_code)]
pub fn _kernel_entry_point(_boot_info: &'static BootInfo) -> ! {
    println!("Hello, World{}", "!");
    panic!("Whoopsie ;)");

    // We'll never get this far, thus #[allow(unreachable_code)]
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
