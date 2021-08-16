use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::slice::{from_raw_parts, from_raw_parts_mut};

use cstr_core::*;
use fatfs::{LinuxDirent64, Stat, StatMode};

use super::*;
use crate::{
    fs::*,
    io::{Read, Write},
    process::process::ProcessInner,
};

pub const AT_FDCWD: usize = -100isize as usize;

pub(super) fn sys_write(fd: usize, buf: *const u8, count: usize) -> isize {
    debug!("sys_write: fd = {:#x}", fd);
    let mut process_inner = get_current_thread().process.inner.lock();
    if let Some(fd) = process_inner.fd_table.get_mut(fd).unwrap() {
        let buffer = unsafe { from_raw_parts(buf, count) };
        if let Ok(n) = unsafe { Arc::get_mut_unchecked(fd) }.write(buffer) {
            return n as isize;
        }
    }
    -1
}

pub(super) fn sys_writev(fd: usize, iovecs: &[&str]) -> isize {
    let mut process_inner = get_current_thread().process.inner.lock();
    if let Some(fd) = process_inner.fd_table.get_mut(fd).unwrap() {
        let mut buffer = Vec::with_capacity(4);
        for &iovec in iovecs {
            buffer.write(iovec.as_bytes()).unwrap();
        }
        if let Ok(n) = unsafe { Arc::get_mut_unchecked(fd) }.write(&buffer) {
            return n as isize;
        }
    }
    -1
}

pub(super) fn sys_read(fd: usize, buf: *mut u8, count: usize) -> isize {
    debug!("sys_read: fd = {:#x}", fd);
    let mut process_inner = get_current_thread().process.inner.lock();
    if let Some(fd) = process_inner.fd_table.get_mut(fd).unwrap() {
        let buffer = unsafe { from_raw_parts_mut(buf, count) };
        if let Ok(n) = unsafe { Arc::get_mut_unchecked(fd) }.read(buffer) {
            return n as isize;
        }
    }
    -1
}

// ssize_t sendfile(int out_fd, int in_fd, off_t *offset, size_t count)
pub(super) fn sys_sendfile(out_fd: usize, in_fd: usize, offset: usize, count: usize) -> isize {
    debug!(
        "sys_sendfile(out_fd={:#x}, in_fd={:#x}, offset={:#x}, count={});",
        out_fd, in_fd, offset, count
    );
    assert_eq!(offset, 0);
    let process_inner = get_current_thread().process.inner.lock();
    if let Some(out_fd) = process_inner.get_fd(out_fd).unwrap() {
        if let Some(in_fd) = process_inner.get_fd(in_fd).unwrap() {
            debug!("out_fd.vnode.full_path {}", out_fd.vnode.full_path);
            debug!("in_fd.vnode.full_path {}", in_fd.vnode.full_path);

            let app = FILE_SYSTEM_TABLE[0]
                .1
                .as_ref()
                .unwrap()
                .open_file(&in_fd.vnode.full_path)
                .unwrap();
            trace!(
                "文件 {} 大小为 {}",
                in_fd.vnode.full_path,
                app.size().unwrap()
            );

            const BLK_SIZE: usize = 512;
            let mut buffer = [0u8; BLK_SIZE];
            let mut remain = count;
            while remain != 0 {
                let read_count = if remain >= BLK_SIZE { BLK_SIZE } else { remain };
                if let Ok(m) =
                    unsafe { Arc::get_mut_unchecked(in_fd) }.read(&mut buffer[..read_count])
                {
                    if let Ok(n) = unsafe { Arc::get_mut_unchecked(out_fd) }.write(&buffer[..m]) {
                        remain -= n;
                        println!("m = {}, n = {}", m, n);
                        // println!("remain = {}", remain);
                        if m != BLK_SIZE && (m != read_count || m != n) {
                            break;
                        }
                    }
                }
            }
            return (count - remain) as isize;
        }
    }

    -1
}

pub(super) fn sys_openat(dirfd: usize, pathname: *const u8, flags: isize, _mode: usize) -> isize {
    let full_path = normalize_path(dirfd, pathname);

    match file::file_open(full_path, unsafe {
        OpenFlags::from_bits_unchecked(flags as usize)
    }) {
        Ok(fd) => {
            let mut process_inner = get_current_thread().process.inner.lock();
            let newfd = process_inner.fd_alloc();
            if newfd >= 0 {
                process_inner.fd_table[newfd as usize] = Some(fd);
                newfd
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

pub(super) fn sys_close(fd: usize) -> isize {
    debug!("sys_close(fd={:#x});", fd);
    *get_current_thread()
        .process
        .inner
        .lock()
        .fd_table
        .get_mut(fd)
        .unwrap() = None;
    0
}

pub(super) fn sys_unlinkat(dirfd: usize, pathname: *const u8, _flags: i32) -> isize {
    let full_path = normalize_path(dirfd, pathname);
    if let Ok(_) = file::file_unlink(full_path) {
        0
    } else {
        -1
    }
}

pub(super) fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    let buffer = unsafe { from_raw_parts_mut(buf, size) };
    let process_inner = get_current_thread().process.inner.lock();
    let cwd = process_inner.cwd.as_bytes();
    // TODO 判断缓冲区大小不够的情况（目前会直接 panic）
    buffer[..cwd.len()].copy_from_slice(cwd);
    buffer[cwd.len()] = b'\0';

    buf as isize
}

/// 成功执行，返回 0。失败，返回-1。
pub(super) fn sys_mkdirat(dirfd: usize, pathname: *const u8, mode: usize) -> isize {
    let full_path = normalize_path(dirfd, pathname);
    file::mkdir(full_path, unsafe {
        StatMode::from_bits_unchecked(mode as u32)
    })
}

pub(super) fn sys_chdir(path: *const u8) -> isize {
    let full_path = normalize_path(AT_FDCWD, path);
    get_current_thread().process.inner.lock().cwd = full_path;

    0
}

pub(super) fn sys_dup(oldfd: usize) -> isize {
    debug!("sys_dup(oldfd={:#x});", oldfd);
    let mut process_inner = get_current_thread().process.inner.lock();
    if let Some(oldfd) = process_inner.fd_table[oldfd].as_ref() {
        let newf = Some(oldfd.clone());
        let newfd = process_inner.fd_alloc();
        if newfd >= 0 {
            process_inner.fd_table[newfd as usize] = newf;
            return newfd;
        }
    }
    -1
}

pub(super) fn sys_dup3(oldfd: usize, newfd: usize, _flags: usize) -> isize {
    debug!(
        "sys_dup3(fd={:#x}, newfd={:#x}, flags={:?});",
        oldfd, newfd, _flags
    );
    let mut process_inner = get_current_thread().process.inner.lock();
    if let Some(oldf) = process_inner.fd_table[oldfd].as_ref() {
        println!("oldf.vnode.full_path = {}", oldf.vnode.full_path);

        let newf = Some(oldf.clone());
        if newfd < ProcessInner::MAX_FD {
            if newfd >= process_inner.fd_table.len() {
                process_inner.fd_table.resize(newfd + 1, None);
            }
            // XXX newfd 可能已经存在，最好还是检查一下
            process_inner.fd_table[newfd] = newf;
            return newfd as isize;
        }
    }
    -1
}

pub(super) fn sys_pipe2(pipefd: *mut i32, _flags: i32) -> isize {
    let mut process_inner = get_current_thread().process.inner.lock();
    let fd_pair = pipe::create_pipe_pair();

    let read_fd = process_inner.fd_alloc() as i32;
    if read_fd >= 0 {
        process_inner.fd_table[read_fd as usize] = Some(fd_pair[0].clone());
    } else {
        return -1;
    }
    let write_fd = process_inner.fd_alloc() as i32;
    if write_fd >= 0 {
        process_inner.fd_table[write_fd as usize] = Some(fd_pair[1].clone());
    } else {
        process_inner.fd_table[read_fd as usize] = None;
        return -1;
    }

    drop(process_inner);
    // 读写用户区内存之前先 drop 掉锁
    unsafe {
        *pipefd.offset(0) = read_fd;
        *pipefd.offset(1) = write_fd;
    }
    debug!(
        "sys_pipe2 result: read_fd={}, write_fd={}",
        read_fd, write_fd
    );

    0
}

/// XXX source 为 sd 卡
pub(super) fn sys_mount(
    _source: *const u8,
    target: *const u8,
    _filesystemtype: *const u8,
    _mountflags: usize,
    _data: *const u8,
) -> isize {
    let target_path = normalize_path(AT_FDCWD, target);
    file::mount(target_path);

    0
}

pub(super) fn sys_umount2(target: *const u8, _flags: i32) -> isize {
    let target_path = normalize_path(AT_FDCWD, target);
    file::umount(target_path);

    0
}

pub(super) fn sys_getdents64(fd: usize, dirp: *mut LinuxDirent64, _count: usize) -> isize {
    let process_inner = get_current_thread().process.inner.lock();
    if let Some(fd) = process_inner.fd_table.get(fd).unwrap() {
        unsafe {
            *dirp = fd.vnode.inode.get_dents64();
        }

        core::mem::size_of::<LinuxDirent64>() as isize
    } else {
        -1
    }
}

pub(super) fn sys_fstat(fd: usize, statbuf: *mut Stat) -> isize {
    let process_inner = get_current_thread().process.inner.lock();
    if let Some(fd) = process_inner.fd_table.get(fd).unwrap() {
        unsafe {
            *statbuf = fd.vnode.inode.get_fstat();
        }

        0
    } else {
        -1
    }
}

pub(super) fn sys_fstatat(
    dirfd: usize,
    pathname: *const u8,
    statbuf: *mut Stat,
    _flags: usize,
) -> isize {
    let full_path = normalize_path(dirfd, pathname);

    if let Ok(fd) = file::file_open(full_path, unsafe { OpenFlags::from_bits_unchecked(0) }) {
        unsafe { *statbuf = fd.vnode.inode.get_fstat() };
        0
    } else {
        -1
    }
}

/// fcntl - manipulate file descriptor
pub(super) fn sys_fcntl(fd: usize, cmd: FcntlCmd, arg: FcntlArg) -> isize {
    debug!("sys_fcntl(fd={:#x}, cmd={:?}, arg={:?});", fd, cmd, arg);

    let mut process_inner = get_current_thread().process.inner.lock();
    if let Some(oldfd) = process_inner.fd_table.get_mut(fd).unwrap() {
        match cmd {
            // Duplicate the file descriptor fd using the lowest-numbered available file descriptor
            // greater than or equal to arg.
            FcntlCmd::F_DUPFD => {
                let newf = Some(oldfd.clone());
                let newfd = process_inner.fd_alloc_from(arg.bits());
                if newfd >= 0 {
                    process_inner.fd_table[newfd as usize] = newf;
                    return newfd;
                }
            }
            // As for F_DUPFD, but additionally set the close-on-exec flag for the duplicate file
            // descriptor.
            FcntlCmd::F_DUPFD_CLOEXEC => {
                let mut newf = oldfd.clone();
                let newfd = process_inner.fd_alloc_from(arg.bits());
                if newfd >= 0 {
                    unsafe { Arc::get_mut_unchecked(&mut newf) }
                        .flags
                        .insert(OpenFlags::CLOEXEC);
                    process_inner.fd_table[newfd as usize] = Some(newf);
                    return newfd;
                }
            }
            // Return (as the function result) the file descriptor flags; arg is ignored.
            FcntlCmd::F_GETFD => return oldfd.flags.bits() as isize,
            // Set the file descriptor flags to the value specified by arg.
            FcntlCmd::F_SETFD => {
                if arg.contains(FcntlArg::FD_CLOEXEC) {
                    unsafe { Arc::get_mut_unchecked(oldfd) }
                        .flags
                        .insert(OpenFlags::CLOEXEC);
                }
            }
            // unimplemented
            _ => {
                error!("fcntl cmd {:?} is not supported yet.", cmd);
            }
        }
    }
    -1
}

/// TODO 去掉中间重复的 `/` 和 `.`
pub fn normalize_path(dirfd: usize, pathname: *const u8) -> String {
    let mut path = unsafe {
        core::str::from_utf8_unchecked(CStr::from_ptr(pathname as *const c_char).to_bytes())
    };
    // 如果是以 / 开头，说明是绝对路径，直接返回
    if path.starts_with('/') {
        return path.to_string();
    }
    // 去掉开头的 `./` 和末尾的 `.`
    path = path.trim_start_matches("./");
    path = path.trim_end_matches(".");

    let process_inner = get_current_thread().process.inner.lock();

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
