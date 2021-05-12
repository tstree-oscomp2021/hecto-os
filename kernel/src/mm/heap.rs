//! 实现操作系统动态内存分配所用的堆

use super::*;
use buddy_system_allocator::LockedHeap;

/// 进行动态内存分配所用的堆空间
///
/// 大小为 [`KERNEL_HEAP_SIZE`]
/// 这段空间编译后会被放在操作系统执行程序的 bss 段
#[repr(align(4096))]
pub struct HeapSpace(pub [u8; KERNEL_HEAP_SIZE]);
pub static mut HEAP_SPACE: HeapSpace = HeapSpace([0; KERNEL_HEAP_SIZE]);

/// 堆，动态内存分配器
///
/// ### `#[global_allocator]`
/// [`LockedHeap`] 实现了 [`alloc::alloc::GlobalAlloc`] trait，
/// 可以为全局需要用到堆的地方分配空间。例如 `Box` `Arc` 等
#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();
/// 0x80_0000 24
// const ORDER: usize = 0usize.leading_zeros() as usize - KERNEL_HEAP_SIZE.leading_zeros() as usize;

/// 初始化操作系统运行时堆空间
pub fn init() {
    // 告诉分配器使用这一段预留的空间作为堆
    unsafe {
        HEAP.lock()
            .init(HEAP_SPACE.0.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
    info!("heap initialized");
}

/// 空间分配错误的回调
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[allow(unused)]
pub fn heap_test() {
    use crate::ffi::*;
    use alloc::{boxed::Box, vec::Vec};

    let bss_range = sbss as usize..ebss as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for i in 0..500 {
        assert_eq!(v[i], i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    info!("heap_test passed!");
}
