//! Slab Allocator. Rewritten from [RT-Thread](https://github.com/RT-Thread/rt-thread/blob/master/src/slab.c).

pub(super) mod linked_list;
pub(super) mod page;
mod slab;

pub use slab::SlabAllocator;
