#![no_std]
#![feature(offset_of)]

extern crate alloc;

pub mod allocator;
pub mod graphics;
pub mod print;
pub mod qemu;
pub mod result;
pub mod serial;
pub mod uefi;
pub mod x86;
