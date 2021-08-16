//! Pipe，视为 Inode

use alloc::{
    boxed::Box,
    collections::VecDeque,
    string::String,
    sync::{Arc, Weak},
};

use fatfs::Inode;

use super::OpenFlags;
use crate::{
    get_current_thread,
    io::*,
    sync::{Condvar, SpinLock},
    FileDescriptor, Vnode,
};

pub fn create_pipe_pair() -> [Arc<FileDescriptor>; 2] {
    let condvar = Arc::new(Condvar::default());
    let data = Arc::new(SpinLock::new(VecDeque::new()));

    [
        Arc::new(FileDescriptor {
            flags: OpenFlags::RDONLY,
            pos: 0,
            vnode: Arc::new(Vnode {
                fs: &(None, None),
                full_path: String::from("pipe read end"),
                inode: Box::new(PipeRead {
                    data: data.clone(),
                    condvar: condvar.clone(),
                }),
            }),
        }),
        Arc::new(FileDescriptor {
            flags: OpenFlags::WRONLY,
            pos: 0,
            vnode: Arc::new(Vnode {
                fs: &(None, None),
                full_path: String::from("pipe write end"),
                inode: Box::new(PipeWrite {
                    data: Arc::downgrade(&data),
                    condvar,
                }),
            }),
        }),
    ]
}

pub struct PipeRead {
    data: Arc<SpinLock<VecDeque<u8>>>,
    condvar: Arc<Condvar>,
}

pub struct PipeWrite {
    data: Weak<SpinLock<VecDeque<u8>>>,
    condvar: Arc<Condvar>,
}

impl Read for PipeRead {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        loop {
            let count = self.data.critical_section(|data| {
                let mut i = 0;
                while i < buf.len() {
                    if let Some(b) = data.pop_front() {
                        buf[i] = b;
                        i += 1;
                    } else {
                        break;
                    }
                }
                if i == 0 {
                    self.condvar.wait();
                }
                i
            });

            // TODO 如果 flags 为 O_NONBLOCK，则返回 Err(EAGAIN)
            if count > 0 {
                return Ok(count);
            } else if Arc::weak_count(&self.data) == 0 {
                // 如果没有了写端，则返回 0 表示读到了末尾
                return Ok(0);
            } else {
                get_current_thread().yield_to_sched();
            }
        }
    }
}
impl Write for PipeRead {
    fn write(&mut self, _buf: &[u8]) -> Result<usize> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }

    fn flush(&mut self) -> Result<()> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }
}
impl Seek for PipeRead {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        Ok(0)
    }
}

impl Read for PipeWrite {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }
}
impl Write for PipeWrite {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        // 如果存在读端
        if let Some(data) = self.data.upgrade() {
            data.critical_section(|data| {
                for &byte in buf {
                    data.push_back(byte);
                }
            });

            self.condvar.notify_all();
            Ok(buf.len())
        } else {
            Err(Error::from(ErrorKind::BrokenPipe))
        }
    }

    fn flush(&mut self) -> Result<()> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }
}
impl Seek for PipeWrite {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        Ok(0)
    }
}

impl Inode for PipeRead {
    fn get_fstat(&self) -> fatfs::Stat {
        todo!()
    }

    fn get_dents64(&self) -> fatfs::LinuxDirent64 {
        todo!()
    }
}
impl Inode for PipeWrite {
    fn get_fstat(&self) -> fatfs::Stat {
        todo!()
    }

    fn get_dents64(&self) -> fatfs::LinuxDirent64 {
        todo!()
    }
}
