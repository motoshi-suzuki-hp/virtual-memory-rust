#![no_std]
#![no_main]
#![feature(offset_of)]

use core::panic::PanicInfo;
use wasabi::{println, info, error};
use wasabi::uefi::{EfiHandle, EfiSystemTable, EfiMemoryType, MemoryMapHolder, exit_from_efi_boot_services};

#[no_mangle]
fn efi_main(image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    println!("Booting WasabiOS...");
    println!("image_handle: {:#018X}", image_handle);
    println!("efi_system_table: {:#p}", efi_system_table);

    // メモリマップを取得
    let mut memory_map = MemoryMapHolder::new();
    exit_from_efi_boot_services(image_handle, efi_system_table, &mut memory_map);

    info!("Exited from UEFI Boot Services");

    // メモリマップを表示
    let mut total_pages = 0;
    for e in memory_map.iter() {
        if e.memory_type() == EfiMemoryType::CONVENTIONAL_MEMORY {
            total_pages += e.number_of_pages();
        }
    }

    let total_mb = total_pages * 4096 / 1024 / 1024;
    info!("Total memory: {} MB", total_mb);

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {:?}", info);
    loop {}
}
