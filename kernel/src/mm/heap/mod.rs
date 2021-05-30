mod slab_allocator;

use self::slab_allocator::LockedHeap;
use crate::board::{interface::Config, ConfigImpl};

#[repr(align(4096))]
pub struct HeapSpace(pub [u8; ConfigImpl::KERNEL_HEAP_SIZE]);
pub static mut HEAP_SPACE: HeapSpace = HeapSpace([0; ConfigImpl::KERNEL_HEAP_SIZE]);

#[global_allocator]
static mut HEAP: LockedHeap = LockedHeap::empty();

pub fn init() {
    unsafe {
        HEAP.init(HEAP_SPACE.0.as_ptr() as usize, ConfigImpl::KERNEL_HEAP_SIZE);
    }
    println!("heap initialized");
}

/// 空间分配错误的回调
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}
