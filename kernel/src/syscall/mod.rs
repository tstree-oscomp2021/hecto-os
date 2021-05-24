//! 各种系统调用
mod fs;
mod process;

use alloc::sync::Arc;

use fs::*;
use interface::Syscall;
use process::*;

use crate::{arch::SyscallImpl, process::*};

/// 系统调用的总入口
pub fn syscall_handler() {
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
        SyscallImpl::pipe2 => todo!(),
        SyscallImpl::dup => sys_dup(args[0]),
        SyscallImpl::dup3 => sys_dup3(args[0], args[1], args[2]),
        SyscallImpl::chdir => sys_chdir(args[0] as *const u8),
        SyscallImpl::openat => sys_openat(args[0], args[1] as *const u8, args[2] as isize, args[3]),
        SyscallImpl::close => sys_close(args[0]),
        SyscallImpl::getdents64 => unimplemented!(),
        SyscallImpl::read => sys_read(args[0], args[1] as *mut u8, args[2]),
        SyscallImpl::write => sys_write(args[0], args[1] as *const u8, args[2]),
        SyscallImpl::linkat => unimplemented!(),
        SyscallImpl::unlinkat => unimplemented!(),
        SyscallImpl::mkdirat => sys_mkdirat(args[0], args[1] as *const u8, args[2]),
        SyscallImpl::umount2 => unimplemented!(),
        SyscallImpl::mount => unimplemented!(),
        SyscallImpl::fstat => unimplemented!(),
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
            args[1] as *mut isize,
            args[2] as isize,
            args[3] as *mut (),
        ),
        SyscallImpl::exit => sys_exit(args[0] as isize),
        SyscallImpl::getppid => sys_getppid(),
        SyscallImpl::getpid => sys_getpid(),
        // 内存管理相关 8 个
        SyscallImpl::brk => unimplemented!(),
        SyscallImpl::munmap => unimplemented!(),
        SyscallImpl::mmap => unimplemented!(),
        SyscallImpl::times => unimplemented!(),
        SyscallImpl::uname => unimplemented!(),
        SyscallImpl::sched_yield => unimplemented!(),
        SyscallImpl::gettimeofday => unimplemented!(),
        SyscallImpl::nanosleep => unimplemented!(),
        // 特定于架构的
        _ => syscall_id.arch_specific_syscall_handler(),
    } as usize;
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
