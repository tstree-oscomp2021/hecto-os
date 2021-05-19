use core::slice::{from_raw_parts, from_raw_parts_mut};

use cstr_core::*;

use super::*;
use crate::{
    fs::*,
    io::{Read, Write},
    processor::current_processor,
};

pub(super) fn sys_write(fd: usize, buf: *const u8, count: usize) -> isize {
    let process = current_processor().lock(|p| p.current_thread().process.clone());
    if let Some(fd) = process.inner.lock().fd_table.get_mut(fd).unwrap() {
        let buffer = unsafe { from_raw_parts(buf, count) };
        if let Ok(n) = unsafe { Arc::get_mut_unchecked(fd) }.write(buffer) {
            return n as isize;
        }
    }
    -1
}

pub(super) fn sys_read(fd: usize, buf: *mut u8, count: usize) -> isize {
    let process = current_processor().lock(|p| p.current_thread().process.clone());
    if let Some(fd) = process.inner.lock().fd_table.get_mut(fd).unwrap() {
        let buffer = unsafe { from_raw_parts_mut(buf, count) };
        if let Ok(n) = unsafe { Arc::get_mut_unchecked(fd) }.read(buffer) {
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
    _mode: usize,
) -> isize {
    let path = unsafe {
        core::str::from_utf8_unchecked(CStr::from_ptr(pathname).to_bytes()).trim_start_matches("./")
    };
    if dirfd == AT_FDCWD {
        let cur_thread = current_processor().lock(|p| p.current_thread());
        let mut process_inner = cur_thread.process.inner.lock();
        // TODO 找到空的 fd
        process_inner.fd_table.push(file_open(path, unsafe {
            OpenFlags::from_bits_unchecked(flags as usize)
        }));
        process_inner.fd_table.len() as isize - 1
    } else {
        -1
    }
}

pub(super) fn sys_close(fd: usize) -> isize {
    current_processor()
        .lock(|p| p.current_thread())
        .process
        .inner
        .lock()
        .fd_table
        .get_mut(fd)
        .take();
    0
}
