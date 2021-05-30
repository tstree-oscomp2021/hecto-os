//! 各种系统调用
mod fs;
mod misc;
mod mm;
mod process;
use alloc::sync::Arc;
use core::{sync::atomic::Ordering, time::Duration};

use fatfs::{LinuxDirent64, Stat};
use fs::*;
use interface::Syscall;
use misc::*;
use mm::*;
use process::*;

use crate::{
    arch::{cpu, SyscallImpl},
    process::*,
};

/// 系统调用的总入口
pub fn syscall_handler() {
    let cur_thread = get_current_thread();
    cur_thread.inner.critical_section(|inner| {
        let cur_cycles = cpu::get_cycles();
        cur_thread
            .process
            .times
            .tms_utime
            .fetch_add(cur_cycles - inner.cycles, Ordering::SeqCst);
        // 线程进入内核控制路径时的时刻
        inner.cycles = cur_cycles;
    });

    let context = get_current_trapframe();

    // 无论如何处理，一定会跳过当前的 ecall 指令
    context.sepc += 4;
    // UNSAFE! 如果为非法的 SyscallImpl，请勿试图直接 printf syscall_id
    let syscall_id: SyscallImpl = unsafe { core::mem::transmute(context.x[17]) };
    // 参数，a0 ~ a5
    let args: &[usize] = &context.x[10..16];

    context.x[10] = match syscall_id {
        // 文件系统相关 16 个
        SyscallImpl::getcwd => sys_getcwd(args[0] as *mut u8, args[1]),
        SyscallImpl::pipe2 => sys_pipe2(args[0] as *mut i32, args[1] as i32),
        SyscallImpl::dup => sys_dup(args[0]),
        SyscallImpl::dup3 => sys_dup3(args[0], args[1], args[2]),
        SyscallImpl::chdir => sys_chdir(args[0] as *const u8),
        SyscallImpl::openat => sys_openat(args[0], args[1] as *const u8, args[2] as isize, args[3]),
        SyscallImpl::close => sys_close(args[0]),
        SyscallImpl::getdents64 => sys_getdents64(args[0], args[1] as *mut LinuxDirent64, args[2]),
        SyscallImpl::read => sys_read(args[0], args[1] as *mut u8, args[2]),
        SyscallImpl::write => sys_write(args[0], args[1] as *const u8, args[2]),
        SyscallImpl::linkat => unimplemented!(),
        SyscallImpl::unlinkat => sys_unlinkat(args[0], args[1] as *const u8, args[2] as i32),
        SyscallImpl::mkdirat => sys_mkdirat(args[0], args[1] as *const u8, args[2]),
        SyscallImpl::umount2 => sys_umount2(args[0] as *const u8, args[1] as i32),
        SyscallImpl::mount => sys_mount(
            args[0] as *const u8,
            args[1] as *const u8,
            args[2] as *const u8,
            args[3],
            args[4] as *const u8,
        ),
        SyscallImpl::fstat => sys_fstat(args[0], args[1] as *mut Stat),
        // 进程管理相关 6 个
        SyscallImpl::clone => sys_clone(
            args[0] as u64,
            args[1] as *mut usize,
            args[2] as *mut usize,
            args[3],
            args[4] as *mut usize,
        ),
        SyscallImpl::execve => sys_execve(
            args[0] as *const u8,
            args[1] as *const *const u8,
            args[2] as *const *const u8,
        ),
        SyscallImpl::wait4 => sys_wait4(
            args[0] as isize,
            args[1] as *mut i32,
            args[2] as isize,
            args[3] as *mut (),
        ),
        SyscallImpl::exit => sys_exit(args[0] as i32),
        SyscallImpl::getppid => sys_getppid(),
        SyscallImpl::getpid => sys_getpid(),
        SyscallImpl::sched_yield => sys_sched_yield(),
        // 内存管理相关 8 个
        SyscallImpl::brk => sys_brk(args[0].into()),
        SyscallImpl::munmap => sys_munmap(args[0].into(), args[1]),
        SyscallImpl::mmap => sys_mmap(
            args[0].into(),
            args[1],
            unsafe { mm::PROT::from_bits_unchecked(args[2]) },
            args[3],
            args[4],
            args[5],
        ),
        // 其他
        SyscallImpl::times => sys_times(args[0] as *mut usize),
        SyscallImpl::uname => sys_uname(args[0] as *mut UTSName),
        SyscallImpl::gettimeofday => {
            sys_gettimeofday(args[0] as *mut TimeVal, args[1] as *mut TimeZone)
        }
        SyscallImpl::nanosleep => {
            sys_nanosleep(args[0] as *const Duration, args[1] as *mut Duration)
        }
        // 特定于架构的
        _ => syscall_id.arch_specific_syscall_handler(),
    } as usize;

    cur_thread.inner.critical_section(|inner| {
        let cur_cycles = cpu::get_cycles();
        cur_thread
            .process
            .times
            .tms_stime
            .fetch_add(cur_cycles - inner.cycles, Ordering::SeqCst);
        // 线程从内核控制路径离开时的时刻
        inner.cycles = cur_cycles;
    });
}

pub mod interface {
    pub trait Syscall {
        /// XXX 需要注意 UB
        fn arch_specific_syscall_handler(self) -> isize;
    }
}

pub type Result<T, E = Errno> = core::result::Result<T, E>;

/// `/usr/include/errno.h`
#[repr(isize)]
pub enum Errno {
    EPERM = 1,         /* Operation not permitted */
    ENOENT = 2,        /* No such file or directory */
    ESRCH = 3,         /* No such process */
    EINTR = 4,         /* Interrupted system call */
    EIO = 5,           /* I/O error */
    ENXIO = 6,         /* No such device or address */
    E2BIG = 7,         /* Argument list too long */
    ENOEXEC = 8,       /* Exec format error */
    EBADF = 9,         /* Bad file number */
    ECHILD = 10,       /* No child processes */
    EAGAIN = 11,       /* Try again */
    ENOMEM = 12,       /* Out of memory */
    EACCES = 13,       /* Permission denied */
    EFAULT = 14,       /* Bad address */
    ENOTBLK = 15,      /* Block device required */
    EBUSY = 16,        /* Device or resource busy */
    EEXIST = 17,       /* File exists */
    EXDEV = 18,        /* Cross-device link */
    ENODEV = 19,       /* No such device */
    ENOTDIR = 20,      /* Not a directory */
    EISDIR = 21,       /* Is a directory */
    EINVAL = 22,       /* Invalid argument */
    ENFILE = 23,       /* File table overflow */
    EMFILE = 24,       /* Too many open files */
    ENOTTY = 25,       /* Not a typewriter */
    ETXTBSY = 26,      /* Text file busy */
    EFBIG = 27,        /* File too large */
    ENOSPC = 28,       /* No space left on device */
    ESPIPE = 29,       /* Illegal seek */
    EROFS = 30,        /* Read-only file system */
    EMLINK = 31,       /* Too many links */
    EPIPE = 32,        /* Broken pipe */
    EDOM = 33,         /* Math argument out of domain of func */
    ERANGE = 34,       /* Math result not representable */
    EDEADLK = 35,      /* Resource deadlock would occur */
    ENAMETOOLONG = 36, /* File name too long */
    ENOLCK = 37,       /* No record locks available */
}
