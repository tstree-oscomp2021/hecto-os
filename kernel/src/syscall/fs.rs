use alloc::string::ToString;
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

    let cur_thread = current_processor().lock(|p| p.current_thread());
    let mut process_inner = cur_thread.process.inner.lock();

    let path = if dirfd == AT_FDCWD {
        // TODO 加上 CWD
        path.to_string()
    } else {
        let mut dir = process_inner.fd_table[dirfd as usize]
            .as_ref()
            .unwrap()
            .vnode
            .full_path
            .clone();
        dir.push('/');
        dir.push_str(path);
        dir
    };
    debug!("open {}", path);

    // TODO 找到空的 fd
    match file_open(path, unsafe {
        OpenFlags::from_bits_unchecked(flags as usize)
    }) {
        Ok(fd) => {
            process_inner.fd_table.push(Some(fd));
            return process_inner.fd_table.len() as isize - 1;
        }
        Err(errno) => -(errno as isize),
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

pub(super) fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    let buffer = unsafe { from_raw_parts_mut(buf, size) };
    let cur_thread = current_processor().lock(|p| p.current_thread());
    let process_inner = cur_thread.process.inner.lock();
    let cwd = process_inner.cwd.as_bytes();
    // TODO 判断缓冲区大小不够的情况（目前会直接 panic）
    buffer[..cwd.len()].copy_from_slice(cwd);
    buffer[cwd.len()] = b'\0';

    buf as isize
}
