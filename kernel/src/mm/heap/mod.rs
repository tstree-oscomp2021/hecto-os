pub use slab_allocator::linked_list;

mod slab_allocator;

use core::alloc::{GlobalAlloc, Layout};

use crate::{
    board::{interface::Config, ConfigImpl},
    sync::SpinLock,
};

#[repr(align(4096))]
pub struct HeapSpace(pub [u8; ConfigImpl::KERNEL_HEAP_SIZE]);
pub static mut HEAP_SPACE: HeapSpace = HeapSpace([0; ConfigImpl::KERNEL_HEAP_SIZE]);

pub struct LockedHeap(SpinLock<slab_allocator::SlabAllocator>);

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.critical_section(|slab| slab.alloc(layout))
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.critical_section(|slab| slab.dealloc(ptr, layout))
    }
}

#[global_allocator]
pub static mut HEAP: LockedHeap = LockedHeap(SpinLock::new(slab_allocator::SlabAllocator));

pub fn init() {
    unsafe {
        HEAP.0.critical_section(|slab| {
            slab.init(
                HEAP_SPACE.0.as_ptr() as usize,
                HEAP_SPACE.0.as_ptr() as usize + ConfigImpl::KERNEL_HEAP_SIZE,
            )
        })
    }
    println!("heap initialized");
}

/// 空间分配错误的回调
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}
