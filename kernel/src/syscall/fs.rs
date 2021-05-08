use super::*;
use crate::{fs::*, hart::*, process::*};
use bitflags::*;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use cstr_core::*;

bitflags! {
    struct OpenFlags: usize {
        /// read only
        const RDONLY = 0;
        /// write only
        const WRONLY = 1;
        /// read write
        const RDWR = 2;
        /// create file if it does not exist
        const CREATE = 1 << 6;
        /// error if CREATE and the file exists
        const EXCLUSIVE = 1 << 7;
        /// truncate file upon open
        const TRUNCATE = 1 << 9;
        /// append on each write
        const APPEND = 1 << 10;
        /// close on exec
        const CLOEXEC = 1 << 19;
    }
}
impl OpenFlags {
    fn readable(&self) -> bool {
        let b = self.bits() & 0b11;
        b == OpenFlags::RDONLY.bits() || b == OpenFlags::RDWR.bits()
    }
    fn writable(&self) -> bool {
        let b = self.bits() & 0b11;
        b == OpenFlags::WRONLY.bits() || b == OpenFlags::RDWR.bits()
    }
}

pub(super) fn sys_write(fd: usize, buf: *const u8, count: usize) -> isize {
    let process = PROCESSORS[get_hart_id()]
        .lock()
        .current_thread()
        .process
        .clone();
    if let Some(inode) = process.inner.lock().fd_table.get_mut(fd).unwrap() {
        let buffer = unsafe { from_raw_parts(buf, count) };
        if let Ok(n) = unsafe { Arc::get_mut_unchecked(inode) }.write(buffer) {
            return n as isize;
        }
    }
    -1
}

pub(super) fn sys_read(fd: usize, buf: *mut u8, count: usize) -> isize {
    let process = PROCESSORS[get_hart_id()]
        .lock()
        .current_thread()
        .process
        .clone();
    if let Some(inode) = process.inner.lock().fd_table.get_mut(fd).unwrap() {
        let buffer = unsafe { from_raw_parts_mut(buf, count) };
        if let Ok(n) = unsafe { Arc::get_mut_unchecked(inode) }.read(buffer) {
            return n as isize;
        }
    }
    -1
}

const AT_FDCWD: isize = -100;

pub(super) fn sys_openat(
    dirfd: isize,
    pathname: *const c_char,
    flags: isize,
    mode: usize,
) -> isize {
    let path = unsafe { core::str::from_utf8_unchecked(CStr::from_ptr(pathname).to_bytes()) };
    if dirfd == AT_FDCWD {
        let cur_thread = PROCESSORS[get_hart_id()].lock().current_thread();
        let mut process_inner = cur_thread.process.inner.lock();
        process_inner
            .fd_table
            .push(Some(Arc::new(ROOT_DIR.open_file(path).unwrap())));
        process_inner.fd_table.len() as isize - 1
    } else {
        -1
    }
}

pub(super) fn sys_close(fd: usize) -> isize {
    PROCESSORS[get_hart_id()]
        .lock()
        .current_thread()
        .process
        .inner
        .lock()
        .fd_table
        .get_mut(fd)
        .take();
    0
}
