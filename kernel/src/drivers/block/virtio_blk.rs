use alloc::vec::Vec;

use lazy_static::lazy_static;
use virtio_drivers::{VirtIOBlk, VirtIOHeader};

use super::{BlockDevice, BLOCK_SZ};
use crate::{
    arch::interface::PageTable,
    board::{interface::Config, ConfigImpl},
    frame_alloc,
    mm::{KERNEL_PAGE_TABLE, VA},
    sync::SpinLock,
    Frame, FrameTracker, PA, PPN,
};

#[allow(unused)]
const VIRTIO0: usize = ConfigImpl::MMIO[0].0;

pub struct VirtIOBlock(VirtIOBlk<'static>);
impl BlockDevice for VirtIOBlock {
    fn new() -> Self {
        Self(VirtIOBlk::new(unsafe { &mut *(VIRTIO0 as *mut VirtIOHeader) }).unwrap())
    }
    fn read_block(&mut self, block_id: usize, buf: &mut [u8; BLOCK_SZ]) {
        self.0
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&mut self, block_id: usize, buf: &[u8; BLOCK_SZ]) {
        self.0
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

lazy_static! {
    /// CPU can submit request to VirtIO device through this circular queue, or obtain the result
    /// of the request from the queue.
    static ref QUEUE_FRAMES: SpinLock<Vec<FrameTracker>> = SpinLock::new(Vec::new());
}

#[no_mangle]
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PA {
    let mut ppn_base = PPN(0);
    for i in 0..pages {
        let frame = frame_alloc().unwrap();
        if i == 0 {
            ppn_base = frame.ppn;
        }
        assert_eq!(frame.ppn.0, ppn_base.0 + i);
        QUEUE_FRAMES.critical_section(|f| f.push(frame));
    }
    ppn_base.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PA, pages: usize) -> i32 {
    let mut ppn_base: PPN = pa.floor();
    for _ in 0..pages {
        core::mem::drop(Frame { ppn: ppn_base });
        ppn_base += 1;
    }
    0
}

#[no_mangle]
pub extern "C" fn virtio_phys_to_virt(paddr: PA) -> VA {
    VA::from(paddr)
}

#[no_mangle]
/// 将虚拟地址转为物理地址（为 [`virtio_drivers`] 库提供）
///
/// 实现这个函数的目的是告诉 DMA 具体的请求
pub extern "C" fn virtio_virt_to_phys(vaddr: VA) -> PA {
    (*KERNEL_PAGE_TABLE).translate_va(vaddr).unwrap()
}
