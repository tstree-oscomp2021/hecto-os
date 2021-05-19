use alloc::string::{String, ToString};
use core::slice::{from_raw_parts, from_raw_parts_mut};

use cstr_core::*;

use super::*;
use crate::{
    fs::*,
    io::{Read, Write},
    processor::current_processor,
};

const AT_FDCWD: usize = -100isize as usize;

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

pub(super) fn sys_openat(
    dirfd: usize,
    pathname: *const c_char,
    flags: isize,
    _mode: usize,
) -> isize {
    let full_path = normalize_path(dirfd, pathname);

    // TODO 找到空的 fd
    match file_open(full_path, unsafe {
        OpenFlags::from_bits_unchecked(flags as usize)
    }) {
        Ok(fd) => {
            let cur_thread = current_processor().lock(|p| p.current_thread());
            let mut process_inner = cur_thread.process.inner.lock();
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

/// 成功执行，返回 0。失败，返回-1。
pub(super) fn sys_mkdirat(dirfd: usize, pathname: *const u8, mode: usize) -> isize {
    let full_path = normalize_path(dirfd, pathname);
    mkdir(full_path, unsafe {
        StatMode::from_bits_unchecked(mode as u32)
    })
}

/// TODO 去掉中间重复的 `/` 和 `.`
fn normalize_path(dirfd: usize, pathname: *const u8) -> String {
    let mut path = unsafe { core::str::from_utf8_unchecked(CStr::from_ptr(pathname).to_bytes()) };
    // 如果是以 / 开头，说明是绝对路径，直接返回
    if path.starts_with('/') {
        return path.to_string();
    }
    // 去掉开头的 `./`
    path = path.trim_start_matches("./");

    let cur_thread = current_processor().lock(|p| p.current_thread());
    let process_inner = cur_thread.process.inner.lock();

    // 目录路径
    let mut dir_path: &str = if dirfd == AT_FDCWD {
        &process_inner.cwd
    } else {
        &process_inner.fd_table[dirfd as usize]
            .as_ref()
            .unwrap()
            .vnode
            .full_path
    };
    dir_path = dir_path.trim_end_matches('/');

    let mut full_path = String::with_capacity(dir_path.len() + 1 + path.len());
    full_path.push_str(dir_path);
    full_path.push('/');
    full_path.push_str(path);
    full_path
}
