#![no_std]
#![no_main]

use core::panic::PanicInfo;
use wasabi::{println, info, warn, error};
use wasabi::qemu::{exit_qemu, QemuExitCode};

#[no_mangle]
fn efi_main(_image_handle: usize, _system_table: usize) {
    println!("Booting WasabiOS...");
    info!("This is an info message");
    warn!("This is a warning");
    error!("This is an error");

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {:?}", info);
    exit_qemu(QemuExitCode::Fail);
}
