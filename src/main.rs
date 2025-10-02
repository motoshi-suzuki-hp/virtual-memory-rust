#![no_std]
#![no_main]
#![feature(offset_of)]

use core::panic::PanicInfo;
use wasabi::{println, info, error};
use wasabi::graphics::{fill_rect, draw_test_pattern, Bitmap};
use wasabi::uefi::{init_vram, EfiHandle, EfiSystemTable, MemoryMapHolder, exit_from_efi_boot_services};
use wasabi::qemu::{exit_qemu, QemuExitCode};

#[no_mangle]
fn efi_main(image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    println!("Booting WasabiOS...");

    // グラフィックスを初期化
    let mut vram = init_vram(efi_system_table).expect("Failed to init vram");
    let vw = vram.width();
    let vh = vram.height();
    info!("VRAM initialized: {}x{}", vw, vh);

    // 画面を黒でクリア
    let _ = fill_rect(&mut vram, 0x000000, 0, 0, vw, vh);

    // テストパターンを描画
    draw_test_pattern(&mut vram);

    info!("Graphics initialized");

    // メモリマップ取得とブートサービス終了
    let mut memory_map = MemoryMapHolder::new();
    exit_from_efi_boot_services(image_handle, efi_system_table, &mut memory_map);

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info:?}");
    exit_qemu(QemuExitCode::Fail);
}
