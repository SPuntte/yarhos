#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yarhos::test_runner_should_panic)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};

entry_point!(_test_start);

#[no_mangle]
pub fn _test_start(_boot_info: &'static BootInfo) -> ! {
    test_main();
    yarhos::halt();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    yarhos::test_panic_handler_should_panic(info)
}

#[test_case]
fn should_fail() {
    assert_eq!(true, false);
}
