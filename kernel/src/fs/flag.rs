/// https://man7.org/linux/man-pages/man2/fcntl.2.html
/// https://man7.org/linux/man-pages/man2/open.2.html
use bitflags::*;

/// see fcntl(2). `/usr/riscv64-linux-gnu/include/asm-generic/fcntl.h`
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum FcntlCmd {
    F_DUPFD = 0, /* dup */
    F_GETFD = 1, /* get close_on_exec */
    F_SETFD = 2, /* set/clear close_on_exec */
    F_GETFL = 3, /* get file->f_flags */
    F_SETFL = 4, /* set file->f_flags */
    F_GETLK = 5,
    F_SETLK = 6,
    F_SETLKW = 7,

    /* As for F_DUPFD, but additionally set the close-on-exec flag for the duplicate file descriptor. */
    F_DUPFD_CLOEXEC = 1024 + 6,
}

bitflags! {
    pub struct FcntlArg: usize {
        const   FD_CLOEXEC  =   1;
    }
}
