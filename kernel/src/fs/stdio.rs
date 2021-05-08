use crate::io::*;

use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref STDIN: Arc<Stdin> = Default::default();
    pub static ref STDOUT: Arc<Stdout> = Default::default();
}

#[derive(Default)]
pub struct Stdin;

#[derive(Default)]
pub struct Stdout;

impl Read for Stdin {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        todo!()
    }
}
impl Write for Stdin {
    fn write(&mut self, _buf: &[u8]) -> Result<usize> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }

    fn flush(&mut self) -> Result<()> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }
}
impl Seek for Stdin {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }
}

impl Read for Stdout {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }
}
impl Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        print!("{}", unsafe { core::str::from_utf8_unchecked(buf) });
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }
}
impl Seek for Stdout {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        Err(Error::from(ErrorKind::PermissionDenied))
    }
}
