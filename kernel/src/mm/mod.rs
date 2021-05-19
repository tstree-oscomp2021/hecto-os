//! 内存管理模块

pub mod address;
pub mod frame_allocator;
pub mod heap;
pub mod memory_set;
pub mod page_table;

pub use address::{VARange, VPNRange, PA, PPN, VA, VPN};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::{MapArea, MapType, MemorySet};
pub use page_table::KERNEL_PAGE_TABLE;

/// 初始化内存相关的子模块
pub fn init() {
    heap::init();
    frame_allocator::init_frame_allocator();

    info!("mod memory initialized");
}

/// bss 段清零
pub fn clear_bss() {
    use crate::board::*;
    unsafe {
        core::slice::from_raw_parts_mut(sbss as *mut usize, ebss as usize - sbss as usize).fill(0);
    }
}

pub mod interface {
    pub use super::page_table::{PageTable, PTE};

    pub trait Config<const MMIO_N: usize> {
        /// 内核使用线性映射的偏移量
        const KERNEL_MAP_OFFSET: usize;
        /// 用户栈大小
        const USER_STACK_SIZE: usize;
        /// 内核栈大小
        const KERNEL_STACK_SIZE: usize;
        /// 内核堆大小
        const KERNEL_HEAP_SIZE: usize;
        /// 内存起始地址
        const MEMORY_START: usize;
        /// 内存大小
        const MEMORY_SIZE: usize;

        const PAGE_SIZE_BITS: usize;
        const PAGE_SIZE: usize;

        // 将 GUARD_PAGE_SIZE 也设置为 KERNEL_STACK_SIZE 的值是为了让每个内核栈顶的后 n
        // 位都是 0，方便根据 sp 得到栈顶
        const GUARD_PAGE_SIZE: usize = Self::KERNEL_STACK_SIZE;
        const KERNEL_STACK_TOP: usize = usize::MAX - Self::GUARD_PAGE_SIZE + 1;
        const MEMORY_END: usize = Self::MEMORY_START + Self::MEMORY_SIZE;

        const MMIO: [(usize, usize); MMIO_N];
        const CLOCK_FREQ: usize;
    }
}
