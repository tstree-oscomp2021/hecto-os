mod virtio_blk;

use crate::{io::*, spinlock::SpinLock};

pub struct BlockDeviceImpl(SpinLock<virtio_blk::VirtIOBlock>);

impl BlockDeviceImpl {
    #[inline]
    pub fn new() -> Self {
        Self(SpinLock::new(virtio_blk::VirtIOBlock::new()))
    }
}
impl Read for BlockDeviceImpl {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.lock().read(buf)
    }
}
impl Write for BlockDeviceImpl {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0.lock().write(buf)
    }
    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.0.lock().flush()
    }
}
impl Seek for BlockDeviceImpl {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.0.lock().seek(pos)
    }
}
