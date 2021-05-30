#![no_std]
#![no_main]
#![allow(incomplete_features)]
#![feature(
    asm,
    global_asm,
    llvm_asm,
    panic_info_message,
    alloc_error_handler,
    format_args_nl,
    step_trait_ext,
    step_trait,
    get_mut_unchecked,
    const_generics,
    drain_filter,
    map_first_last,
    const_btree_new,
    const_fn_trait_bound
)]
// #![allow(dead_code)]
// #![allow(unused)]

#[macro_use]
pub mod logger;
pub mod backtrace;
pub mod drivers;
pub mod fs;
#[macro_use]
pub mod mm;
pub mod process;
pub mod sync;
pub mod syscall;
pub mod timer;
pub mod trap;

pub mod arch;
pub mod board;

use core_io as io;

extern crate alloc;
extern crate core_io;

pub use algorithm::*;
pub use fs::*;
pub use mm::*;
pub use process::*;

use crate::{
    arch::{
        TaskContextImpl, TrapImpl, __switch,
        cpu::shutdown,
        interface::{PageTable, Trap},
    },
    board::{init_board, interface::Config, ConfigImpl},
    schedule::get_sched_cx,
    syscall::{RELEASE, SYSNAME},
};

#[no_mangle]
pub fn rust_main(hart_id: usize, dtb_pa: PA) -> ! {
    if hart_id == ConfigImpl::BOOT_CPU_ID {
        init_board(hart_id, dtb_pa);
        println!("\n{} {}", SYSNAME, RELEASE);
        mm::clear_bss();
        mm::init();
        fs::init();
    }
    TrapImpl::init();

    mm::KERNEL_PAGE_TABLE.activate();
    // 初始化调度线程
    let sched_thread = Thread::init_sched_thread(schedule as usize);
    *get_sched_cx() = sched_thread.task_cx;
    unsafe {
        let mut cur_task_cx: *const TaskContextImpl = core::mem::transmute(1usize);
        __switch(&mut cur_task_cx, *get_sched_cx());
    }

    panic!("bug")
}

pub fn schedule() {
    println!("schedule");

    #[rustfmt::skip]
    let file_name = [
        "yield",
        "write",
        "waitpid",
        "wait",
        "unlink",
        "uname",
        "umount",
        "times",
        "read",
        "pipe",
        "openat",
        "open",
        "munmap",
        "mount",
        "mmap",
        "mkdir_",
        "gettimeofday",
        "getppid",
        "getpid",
        "getdents",
        "getcwd",
        "fstat",
        "fork",
        "exit",
        "execve",
        "dup2",
        "dup",
        "close",
        "clone",
        "chdir",
        "brk",
        "sleep"
    ];

    let mut testsuits = alloc::collections::VecDeque::new();
    for file in file_name {
        testsuits.push_back(Thread::new_thread(file, None));
    }

    println!("run user thread");
    loop {
        if let Some(test) = testsuits.pop_front() {
            SCHEDULER.critical_section(|s| s.add_thread(test));
        } else {
            unsafe {
                shutdown();
            }
        }
        while let Some(next_thread) = SCHEDULER.critical_section(|v| v.get_next()) {
            let status = next_thread.inner.lock().status;
            match status {
                ThreadStatus::Ready => {
                    debug!("thread {:?} is ready", next_thread.tid);
                    next_thread.activate();
                    // next_thread.inner.lock().status = ThreadStatus::Running;
                    unsafe {
                        __switch(get_sched_cx(), next_thread.task_cx);
                    }
                }
                _ => {}
            }
        }
    }
}
