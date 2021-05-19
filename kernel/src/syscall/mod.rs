//! 各种系统调用
mod fs;
mod process;

use alloc::sync::Arc;
use core::mem::size_of;

use cstr_core::*;
use fs::*;
use interface::Syscall;
use process::*;

use crate::{
    arch::{interface::Register, RegisterImpl, SyscallImpl, TrapFrameImpl},
    board::{interface::Config, ConfigImpl},
    mm::*,
};

/// 系统调用的总入口
pub fn syscall_handler() {
    // 对 sp 向上取整
    // XXX 可能的问题：sp 刚好在栈底
    let kernel_stack_top = VA(RegisterImpl::sp() - 1 + ConfigImpl::KERNEL_STACK_SIZE
        & !(ConfigImpl::KERNEL_STACK_SIZE - 1));
    let context: &mut TrapFrameImpl = (kernel_stack_top - size_of::<TrapFrameImpl>()).get_mut();

    // 无论如何处理，一定会跳过当前的 ecall 指令
    context.sepc += 4;
    // UNSAFE! 如果为非法的 SyscallImpl，请勿试图直接 printf syscall_id
    let syscall_id: SyscallImpl = unsafe { core::mem::transmute(context.x[17]) };
    // 参数，a0 ~ a5
    let args: &[usize] = &context.x[10..16];

    context.x[10] = match syscall_id {
        // 文件系统相关 16 个
        SyscallImpl::getcwd => unimplemented!(),
        SyscallImpl::pipe2 => unimplemented!(),
        SyscallImpl::dup => unimplemented!(),
        SyscallImpl::dup3 => unimplemented!(),
        SyscallImpl::chdir => unimplemented!(),
        SyscallImpl::openat => sys_openat(
            args[0] as isize,
            args[1] as *const c_char,
            args[2] as isize,
            args[3],
        ),
        SyscallImpl::close => sys_close(args[0]),
        SyscallImpl::getdents64 => unimplemented!(),
        SyscallImpl::read => sys_read(args[0], args[1] as *mut u8, args[2]),
        SyscallImpl::write => sys_write(args[0], args[1] as *const u8, args[2]),
        SyscallImpl::linkat => unimplemented!(),
        SyscallImpl::unlinkat => unimplemented!(),
        SyscallImpl::mkdirat => unimplemented!(),
        SyscallImpl::umount2 => unimplemented!(),
        SyscallImpl::mount => unimplemented!(),
        SyscallImpl::fstat => unimplemented!(),
        // 进程管理相关 6 个
        SyscallImpl::clone => unimplemented!(),
        SyscallImpl::execve => unimplemented!(),
        SyscallImpl::wait4 => unimplemented!(),
        SyscallImpl::exit => sys_exit(args[0] as isize),
        SyscallImpl::getppid => unimplemented!(),
        SyscallImpl::getpid => unimplemented!(),
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
