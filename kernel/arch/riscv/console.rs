use super::sbi::console_putchar;
use crate::{
    arch::interface::Console,
    io::{Read, Seek, Write},
    spinlock::SpinLock,
};

pub struct ConsoleImpl;

static CONSOLE_LOCK: SpinLock<()> = SpinLock::new(());

impl core::fmt::Write for ConsoleImpl {
    /// 打印一个字符串
    ///
    /// [`console_putchar`] sbi 调用每次接受一个 `usize`，但实际上会把它作为
    /// `u8` 来打印字符。 因此，如果字符串中存在非 ASCII 字符，需要在 utf-8
    /// 编码下，对于每一个 `u8` 调用一次 [`console_putchar`]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        CONSOLE_LOCK.lock(|_| s.bytes().for_each(|c| console_putchar(c as usize)));
        Ok(())
    }
}

impl Read for ConsoleImpl {
    fn read(&mut self, _buf: &mut [u8]) -> core_io::Result<usize> {
        todo!()
    }
}
impl Write for ConsoleImpl {
    fn write(&mut self, buf: &[u8]) -> core_io::Result<usize> {
        core::fmt::Write::write_str(self, unsafe { core::str::from_utf8_unchecked(buf) }).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> core_io::Result<()> {
        todo!()
    }
}
impl Seek for ConsoleImpl {
    fn seek(&mut self, _pos: core_io::SeekFrom) -> core_io::Result<u64> {
        Ok(0)
    }
}

impl Console for ConsoleImpl {
    const CONSOLE_INSTANCE: Self = ConsoleImpl;
}
