//! 各种系统调用

mod fs;
mod process;
mod syscall_id;

use crate::{interrupt::*, mm::*, register::*};
use alloc::sync::Arc;
use core::mem::size_of;
use cstr_core::*;
use fs::*;
use process::*;
use syscall_id::SyscallId;

/// 系统调用的总入口
pub fn syscall_handler() {
    unsafe {
        riscv::register::sstatus::set_sie();
    }

    // 对 sp 向上取整
    // XXX 可能的问题：sp 刚好在栈底
    let kernel_stack_top = VA(sp() - 1 + KERNEL_STACK_SIZE & !(KERNEL_STACK_SIZE - 1));
    let context: &mut Context = (kernel_stack_top - size_of::<Context>()).get_mut();

    // 无论如何处理，一定会跳过当前的 ecall 指令
    context.sepc += 4;
    // UNSAFE! 如果为非法的 SyscallId，请勿试图直接 printf syscall_id
    let syscall_id: SyscallId = unsafe { core::mem::transmute(context.x[17]) };
    // 参数，a0 ~ a5
    let args: &[usize] = &context.x[10..16];

    context.x[10] = match syscall_id {
        // 文件系统相关 16 个
        SyscallId::getcwd => unimplemented!(),
        SyscallId::pipe2 => unimplemented!(),
        SyscallId::dup => unimplemented!(),
        SyscallId::dup3 => unimplemented!(),
        SyscallId::chdir => unimplemented!(),
        SyscallId::openat => sys_openat(
            args[0] as isize,
            args[1] as *const c_char,
            args[2] as isize,
            args[3],
        ),
        SyscallId::close => sys_close(args[0]),
        SyscallId::getdents64 => unimplemented!(),
        SyscallId::read => sys_read(args[0], args[1] as *mut u8, args[2]), //
        SyscallId::write => sys_write(args[0], args[1] as *const u8, args[2]), //
        SyscallId::linkat => unimplemented!(),
        SyscallId::unlinkat => unimplemented!(),
        SyscallId::mkdirat => unimplemented!(),
        SyscallId::umount2 => unimplemented!(),
        SyscallId::mount => unimplemented!(),
        SyscallId::fstat => unimplemented!(),
        // 进程管理相关 6 个
        SyscallId::clone => unimplemented!(),  //
        SyscallId::execve => unimplemented!(), //
        SyscallId::wait4 => unimplemented!(),  //
        SyscallId::exit => sys_exit(args[0] as isize),
        SyscallId::getppid => unimplemented!(),
        SyscallId::getpid => unimplemented!(),
        // 内存管理相关 8 个
        SyscallId::brk => unimplemented!(),
        SyscallId::munmap => unimplemented!(),
        SyscallId::mmap => unimplemented!(),
        SyscallId::times => unimplemented!(),
        SyscallId::uname => unimplemented!(),
        SyscallId::sched_yield => unimplemented!(), //
        SyscallId::gettimeofday => unimplemented!(),
        SyscallId::nanosleep => unimplemented!(),
        _ => {
            println!("unimplemented syscall: {}", syscall_id as usize);
            -1
        }
    } as usize;
}
