use crate::{io, io::*, mm::*, spinlock::*};
use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::{VirtIOBlk, VirtIOHeader};

#[allow(unused)]
const VIRTIO0: usize = crate::config::MMIO[0].0 + KERNEL_MAP_OFFSET;

const BLOCK_SZ: usize = 512;

/// 一个带 buffer 的块设备
/// TODO 给多个数据块都加上 buffer
pub struct VirtIOBlock {
    device: VirtIOBlk<'static>,
    buf: [u8; BLOCK_SZ],
    pos: usize, // 在 buf 中的偏移量
    block_id: usize,
    modified: bool,
}

unsafe impl Send for VirtIOBlock {}
unsafe impl Sync for VirtIOBlock {}

impl VirtIOBlock {
    pub fn new() -> Self {
        let mut virtio_block = Self {
            device: VirtIOBlk::new(unsafe { &mut *(VIRTIO0 as *mut VirtIOHeader) }).unwrap(),
            buf: [0; BLOCK_SZ],
            pos: 0,
            block_id: 0,
            modified: false,
        };
        virtio_block.fill_buf();

        virtio_block
    }

    #[inline]
    fn fill_buf(&mut self) {
        self.device
            .read_block(self.block_id, &mut self.buf)
            .expect("Error when reading VirtIOBlk");
    }
    #[inline]
    fn flush_buf(&mut self) {
        self.device
            .write_block(self.block_id, &self.buf)
            .expect("Error when writing VirtIOBlk");
    }
}

/// TODO 按需 fill_buf（初始化时可以不 fill_buf，读满后也可以不 fill_buf）
/// TODO 写一些单元测试
impl Read for VirtIOBlock {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // 将 self.buf 中剩余的读进去
        let n = (&self.buf[self.pos..]).read(buf)?;
        self.pos += n;
        // 如果 self.buf 中的读完了，继续
        if self.pos == BLOCK_SZ - 1 {
            self.flush()?;
            // 看看是否能够不通过 self.buf，直接传输数据
            let size = (buf.len() - n) / BLOCK_SZ;
            if size > 0 {
                for i in 0..size {
                    self.block_id += 1;
                    self.device
                        .read_block(
                            self.block_id,
                            &mut buf[n + i * BLOCK_SZ..n + (i + 1) * BLOCK_SZ],
                        )
                        .expect("Error when reading VirtIOBlk");
                }
            }
            self.block_id += 1;
            self.fill_buf();
            // 将刚读入的读进去
            self.pos = (&self.buf[..]).read(&mut buf[n + size * BLOCK_SZ..])?;
        }

        Ok(buf.len())
    }
}
impl Write for VirtIOBlock {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // 将 self.buf 中剩余的写进去
        let n = (&mut self.buf[self.pos..]).write(buf)?;
        self.pos += n;
        if n > 0 {
            self.modified = true;
        }
        // 如果 self.buf 中的写满了，继续
        if self.pos == BLOCK_SZ - 1 {
            self.flush()?;
            // 看看是否能够不通过 self.buf，直接传输数据
            let size = buf.len() - n / BLOCK_SZ;
            if size > 0 {
                for i in 0..size {
                    self.block_id += 1;
                    self.device
                        .write_block(
                            self.block_id,
                            &buf[n + i * BLOCK_SZ..n + (i + 1) * BLOCK_SZ],
                        )
                        .expect("Error when reading VirtIOBlk");
                }
            }
            self.block_id += 1;
            self.fill_buf();
            // 将刚读入的写进去
            self.pos = (&mut self.buf[..]).write(&buf[n + size * BLOCK_SZ..])?;
            if self.pos > 0 {
                self.modified = true;
            }
        }

        Ok(buf.len())
    }

    /// 切换到一个 block 前需要 flush 一下
    fn flush(&mut self) -> io::Result<()> {
        if self.modified {
            self.flush_buf();
            self.modified = false;
        }
        Ok(())
    }
}
impl Seek for VirtIOBlock {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(i) => {
                self.pos = i as usize % BLOCK_SZ;
                let new_block_id = i as usize / BLOCK_SZ;
                if self.block_id != new_block_id {
                    self.flush()?;
                    self.block_id = new_block_id;
                    self.fill_buf();
                }

                Ok(i)
            }
            SeekFrom::End(i) => {
                let max_block_id = self.device.capacity - 1;
                let i = ((max_block_id * BLOCK_SZ - 1) as i64 + i) as u64;

                self.pos = i as usize % BLOCK_SZ;
                let new_block_id = i as usize / BLOCK_SZ;
                if self.block_id != new_block_id {
                    self.flush()?;
                    self.block_id = new_block_id;
                    self.fill_buf();
                }

                Ok(i as u64)
            }
            SeekFrom::Current(i) => {
                let i = ((self.block_id * BLOCK_SZ + self.pos) as i64 + i) as u64;

                self.pos = i as usize % BLOCK_SZ;
                let new_block_id = i as usize / BLOCK_SZ;
                if self.block_id != new_block_id {
                    self.flush()?;
                    self.block_id = new_block_id;
                    self.fill_buf();
                }

                Ok(i as u64)
            }
        }
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
        QUEUE_FRAMES.lock().push(frame);
    }
    ppn_base.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PA, pages: usize) -> i32 {
    let mut ppn_base: PPN = pa.into();
    for _ in 0..pages {
        core::mem::drop(FrameTracker { ppn: ppn_base });
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
