extern crate alloc;

use crate::result::Result;
use alloc::boxed::Box;
use core::arch::asm;
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

// 基本的なx86命令
pub fn hlt() {
    unsafe { asm!("hlt") }
}

pub fn busy_loop_hint() {
    unsafe { asm!("pause") }
}

pub fn read_io_port_u8(port: u16) -> u8 {
    let mut data: u8;
    unsafe {
        asm!("in al, dx",
            out("al") data,
            in("dx") port)
    }
    data
}

pub fn write_io_port_u8(port: u16, data: u8) {
    unsafe {
        asm!("out dx, al",
            in("al") data,
            in("dx") port)
    }
}

// ページテーブル関連の定数
pub const PAGE_SIZE: usize = 4096;
const ATTR_MASK: u64 = 0xFFF;
const ATTR_PRESENT: u64 = 1 << 0;
const ATTR_WRITABLE: u64 = 1 << 1;
const ATTR_WRITE_THROUGH: u64 = 1 << 3;
const ATTR_CACHE_DISABLE: u64 = 1 << 4;

// CR3レジスタの読み書き
pub fn read_cr3() -> *mut PML4 {
    let mut cr3: *mut PML4;
    unsafe {
        asm!("mov rax, cr3", out("rax") cr3)
    }
    cr3
}

pub unsafe fn write_cr3(table: *const PML4) {
    asm!("mov cr3, rax", in("rax") table)
}

pub fn flush_tlb() {
    unsafe {
        write_cr3(read_cr3());
    }
}

// ページ属性
#[derive(Debug, Copy, Clone)]
#[repr(u64)]
pub enum PageAttr {
    NotPresent = 0,
    ReadWriteKernel = ATTR_PRESENT | ATTR_WRITABLE,
    ReadWriteIo = ATTR_PRESENT | ATTR_WRITABLE | ATTR_WRITE_THROUGH | ATTR_CACHE_DISABLE,
}

// エントリ構造体（各レベル共通、ジェネリック）
#[repr(transparent)]
pub struct Entry<const LEVEL: usize, const SHIFT: usize, NEXT> {
    value: u64,
    next_type: PhantomData<NEXT>,
}

impl<const LEVEL: usize, const SHIFT: usize, NEXT> Entry<LEVEL, SHIFT, NEXT> {
    fn read_value(&self) -> u64 {
        self.value
    }

    fn is_present(&self) -> bool {
        (self.read_value() & (1 << 0)) != 0
    }

    fn is_writable(&self) -> bool {
        (self.read_value() & (1 << 1)) != 0
    }

    fn is_user(&self) -> bool {
        (self.read_value() & (1 << 2)) != 0
    }

    fn format(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "L{}Entry @ {:#p} {{ {:#018X} {}{}{} ",
            LEVEL,
            self,
            self.read_value(),
            if self.is_present() { "P" } else { "N" },
            if self.is_writable() { "W" } else { "R" },
            if self.is_user() { "U" } else { "S" }
        )?;
        write!(f, " }}")
    }

    fn table(&self) -> Result<&NEXT> {
        if self.is_present() {
            Ok(unsafe { &*((self.value & !ATTR_MASK) as *const NEXT) })
        } else {
            Err("Page Not Found")
        }
    }

    fn table_mut(&mut self) -> Result<&mut NEXT> {
        if self.is_present() {
            Ok(unsafe { &mut *((self.value & !ATTR_MASK) as *mut NEXT) })
        } else {
            Err("Page Not Found")
        }
    }

    fn set_page(&mut self, phys: u64, attr: PageAttr) -> Result<()> {
        if phys & ATTR_MASK != 0 {
            Err("Phys is not aligned")
        } else {
            self.value = phys | attr as u64;
            Ok(())
        }
    }

    fn populate(&mut self) -> Result<&mut Self> {
        if self.is_present() {
            Err("Page is already populated")
        } else {
            let next: Box<NEXT> = Box::new(unsafe { MaybeUninit::zeroed().assume_init() });
            self.value = Box::into_raw(next) as u64 | PageAttr::ReadWriteKernel as u64;
            Ok(self)
        }
    }

    fn ensure_populated(&mut self) -> Result<&mut Self> {
        if self.is_present() {
            Ok(self)
        } else {
            self.populate()
        }
    }
}

impl<const LEVEL: usize, const SHIFT: usize, NEXT> fmt::Display for Entry<LEVEL, SHIFT, NEXT> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f)
    }
}

impl<const LEVEL: usize, const SHIFT: usize, NEXT> fmt::Debug for Entry<LEVEL, SHIFT, NEXT> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f)
    }
}

// テーブル構造体（各レベル共通、ジェネリック）
#[repr(align(4096))]
pub struct Table<const LEVEL: usize, const SHIFT: usize, NEXT> {
    entry: [Entry<LEVEL, SHIFT, NEXT>; 512],
}

impl<const LEVEL: usize, const SHIFT: usize, NEXT: core::fmt::Debug> Table<LEVEL, SHIFT, NEXT> {
    fn format(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "L{}Table @ {:#p} {{", LEVEL, self)?;
        for i in 0..512 {
            let e = &self.entry[i];
            if !e.is_present() {
                continue;
            }
            writeln!(f, "  entry[{:3}] = {:?}", i, e)?;
        }
        writeln!(f, "}}")
    }

    pub fn next_level(&self, index: usize) -> Option<&NEXT> {
        self.entry.get(index).and_then(|e| e.table().ok())
    }

    fn calc_index(&self, addr: u64) -> usize {
        ((addr >> SHIFT) & 0b1_1111_1111) as usize
    }
}

impl<const LEVEL: usize, const SHIFT: usize, NEXT: fmt::Debug> fmt::Debug
    for Table<LEVEL, SHIFT, NEXT>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f)
    }
}

// 型エイリアス（4レベルのページテーブル階層）
pub type PT   = Table<1, 12, [u8; PAGE_SIZE]>;  // Page Table
pub type PD   = Table<2, 21, PT>;                // Page Directory
pub type PDPT = Table<3, 30, PD>;                // Page Directory Pointer Table
pub type PML4 = Table<4, 39, PDPT>;              // Page Map Level 4

impl PML4 {
    pub fn new() -> Box<Self> {
        Box::new(Self::default())
    }

    fn default() -> Self {
        // 0埋めしたエントリは全てNotPresentなので安全
        unsafe { MaybeUninit::zeroed().assume_init() }
    }

    pub fn create_mapping(
        &mut self,
        virt_start: u64,
        virt_end: u64,
        phys: u64,
        attr: PageAttr,
    ) -> Result<()> {
        // 4重ループで各レベルのテーブルを辿りながらマッピングを作成
        let table = self;
        let mut addr = virt_start;
        loop {
            let index = table.calc_index(addr);
            let table = table.entry[index].ensure_populated()?.table_mut()?;
            loop {
                let index = table.calc_index(addr);
                let table = table.entry[index].ensure_populated()?.table_mut()?;
                loop {
                    let index = table.calc_index(addr);
                    let table = table.entry[index].ensure_populated()?.table_mut()?;
                    loop {
                        let index = table.calc_index(addr);
                        let pte = &mut table.entry[index];
                        let phys_addr = phys + addr - virt_start;
                        pte.set_page(phys_addr, attr)?;
                        addr = addr.wrapping_add(PAGE_SIZE as u64);
                        if index + 1 >= (1 << 9) || addr >= virt_end {
                            break;
                        }
                    }
                    if index + 1 >= (1 << 9) || addr >= virt_end {
                        break;
                    }
                }
                if index + 1 >= (1 << 9) || addr >= virt_end {
                    break;
                }
            }
            if index + 1 >= (1 << 9) || addr >= virt_end {
                break;
            }
        }
        Ok(())
    }
}
