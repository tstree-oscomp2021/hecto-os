//! 内存管理模块

pub mod address;
pub mod frame_allocator;
pub mod heap;
pub mod memory_set;
pub mod page_table;

pub use address::{VARange, VPNRange, PA, PPN, VA, VPN};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::{MapArea, MapType, MemorySet};
pub use page_table::{PTEFlags, PageTable, PageTableEntry, KERNEL_PAGE_TABLE};

/// 初始化内存相关的子模块
pub fn init() {
    heap::init();
    frame_allocator::init_frame_allocator();

    log::info!("mod memory initialized");
}

/// bss 段清零
pub fn clear_bss() {
    use crate::ffi::*;
    unsafe {
        core::ptr::write_bytes(
            sbss as *mut usize,
            0,
            (ebss as usize - sbss as usize) / core::mem::size_of::<usize>(),
        );
    }
}

/// 内核使用线性映射的偏移量
pub const KERNEL_MAP_OFFSET: usize = 0xFFFF_FFC0_0000_0000;

pub const USER_STACK_SIZE: usize = 2 << 13;
pub const KERNEL_STACK_SIZE: usize = 2 << 13;
// 将 GUARD_PAGE_SIZE 也设置为 KERNEL_STACK_SIZE 的值是为了让每个内核栈顶的后 n 位都是 0，方便根据 sp 得到栈顶
pub const GUARD_PAGE_SIZE: usize = KERNEL_STACK_SIZE;
pub const KERNEL_STACK_TOP: usize = usize::MAX - GUARD_PAGE_SIZE + 1;

pub const KERNEL_HEAP_SIZE: usize = 0x20_0000;

pub const MEMORY_START: usize = 0xFFFF_FFC0_8000_0000;
pub const MEMORY_SIZE: usize = 0x80_0000;
pub const MEMORY_END: usize = MEMORY_START + MEMORY_SIZE;

pub const PAGE_SIZE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 0x1000;
