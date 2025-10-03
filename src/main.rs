#![no_std]
#![no_main]
#![feature(offset_of)]

extern crate alloc;

use alloc::vec::Vec;
use core::panic::PanicInfo;
use wasabi::{println, info, error};
use wasabi::allocator::ALLOCATOR;
use wasabi::graphics::{fill_rect, draw_test_pattern, Bitmap};
use wasabi::qemu::{exit_qemu, QemuExitCode};
use wasabi::uefi::{init_vram, EfiHandle, EfiSystemTable, MemoryMapHolder, exit_from_efi_boot_services};

#[no_mangle]
fn efi_main(image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    println!("Booting WasabiOS...");

    // グラフィックスを初期化
    let mut vram = init_vram(efi_system_table).expect("init_vram failed");
    let vw = vram.width();
    let vh = vram.height();
    info!("VRAM initialized: {}x{}", vw, vh);
    fill_rect(&mut vram, 0x000000, 0, 0, vw, vh).expect("fill_rect failed");
    draw_test_pattern(&mut vram);
    info!("Graphics initialized");

    // メモリマップを取得してブートサービスを終了
    let mut memory_map = MemoryMapHolder::new();
    exit_from_efi_boot_services(image_handle, efi_system_table, &mut memory_map);
    info!("Exited from UEFI boot services");

    // アロケータを初期化
    ALLOCATOR.init_with_mmap(&memory_map);
    info!("Memory allocator initialized");

    // ヒープメモリのテスト
    let mut vec = Vec::new();
    vec.push(1);
    vec.push(2);
    vec.push(3);
    info!("Vec test: {:?}", vec);

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info:?}");
    exit_qemu(QemuExitCode::Fail);
}
