pub use sdcard::SDCardWrapper;
pub use virtio_blk::VirtIOBlock;

mod sdcard;
mod virtio_blk;

use core::mem::transmute;

use crate::{
    io,
    io::{Read, Result, Seek, SeekFrom, Write},
    sync::SpinLock,
};

const BLOCK_SZ: usize = 512;
/// 块设备抽象
pub trait BlockDevice: Send + Sync {
    fn new() -> Self;
    fn read_block(&mut self, block_id: usize, buf: &mut [u8; BLOCK_SZ]);
    fn write_block(&mut self, block_id: usize, buf: &[u8; BLOCK_SZ]);
}

/// The [`BufBlkDeviceInner<Block>`] struct adds buffering to any BlockDevice.
///
/// TODO 设置 Buffer 数量
struct BufBlkDeviceInner<Block: BlockDevice> {
    device: Block,
    buf: [u8; BLOCK_SZ],
    pos: usize, // 在 buf 中的偏移量
    block_id: usize,
    modified: bool,
}

impl<Block: BlockDevice> BufBlkDeviceInner<Block> {
    pub fn new() -> Self {
        let mut block_device = Self {
            device: Block::new(),
            buf: [0; BLOCK_SZ],
            pos: 0,
            block_id: 0,
            modified: false,
        };
        block_device.fill_buf();

        block_device
    }

    #[inline]
    fn fill_buf(&mut self) {
        self.device.read_block(self.block_id, &mut self.buf);
    }
    #[inline]
    fn flush_buf(&mut self) {
        self.device.write_block(self.block_id, &self.buf);
    }
}

/// TODO 按需 fill_buf（初始化时可以不 fill_buf，读满后也可以不 fill_buf）
impl<Block: BlockDevice> Read for BufBlkDeviceInner<Block> {
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
                    #[allow(mutable_transmutes)]
                    self.device
                        .read_block(self.block_id, unsafe { transmute(&buf[n + i * BLOCK_SZ]) });
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
impl<Block: BlockDevice> Write for BufBlkDeviceInner<Block> {
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
                        .write_block(self.block_id, unsafe { transmute(&buf[n + i * BLOCK_SZ]) });
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
impl<Block: BlockDevice> Seek for BufBlkDeviceInner<Block> {
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
            SeekFrom::End(_) => {
                unimplemented!()
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

/// [`BufBlockDevice<Block>`] 是一个带锁的 [`BufBlkDeviceInner<Block>`]
pub struct BufBlockDevice<Block: BlockDevice>(SpinLock<BufBlkDeviceInner<Block>>);

impl<Block: BlockDevice> BufBlockDevice<Block> {
    #[inline]
    pub fn new() -> Self {
        Self(SpinLock::new(BufBlkDeviceInner::new()))
    }
}
impl<Block: BlockDevice> Read for BufBlockDevice<Block> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.critical_section(|v| v.read(buf))
    }
}
impl<Block: BlockDevice> Write for BufBlockDevice<Block> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0.critical_section(|v| v.write(buf))
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.0.critical_section(|v| v.flush())
    }
}
impl<Block: BlockDevice> Seek for BufBlockDevice<Block> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.0.critical_section(|v| v.seek(pos))
    }
}
