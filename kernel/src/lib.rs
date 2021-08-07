#![allow(
    // dead_code,
    // unused,
    incomplete_features,
    rustdoc::private_intra_doc_links,
)]
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
    const_fn_trait_bound,
    allocator_api,
    inline_const
)]
#![no_std]
// Testing
#![cfg_attr(
    test,
    no_main,
    test_runner(crate::test_runner),
    feature(custom_test_frameworks),
    reexport_test_harness_main = "test_main"
)]

#[macro_use]
pub mod logger;
#[macro_use]
pub mod mm;

pub mod arch;
pub mod backtrace;
pub mod board;
pub mod drivers;
pub mod fs;
pub mod process;
pub mod sync;
pub mod syscall;
pub mod timer;
pub mod trap;

pub extern crate alloc;
pub extern crate core_io;

pub use algorithm::*;
pub use core_io as io;
pub use fs::*;
pub use mm::*;
pub use process::*;

use crate::{
    arch::{
        TaskContextImpl, __switch,
        cpu::{get_duration, shutdown},
        interface::{PageTable, Trap},
        TrapImpl,
    },
    board::{init_board, interface::Config, ConfigImpl},
    schedule::{get_sched_cx, SCHEDULE_THREAD},
    syscall::{RELEASE, SYSNAME},
};

pub fn kernel_init(hart_id: usize, dtb_pa: PA) {
    if hart_id == ConfigImpl::BOOT_CPU_ID {
        init_board(hart_id, dtb_pa);
        mm::clear_bss();
        println!("\n{} {}", SYSNAME, RELEASE);
        mm::init();
        fs::init();
    }
    TrapImpl::init();
    mm::KERNEL_PAGE_TABLE.activate();
}

pub fn switch_to_schedule(args: &[&[&str]]) -> ! {
    let sched_thread = Thread::new_kernel(schedule as usize, None);
    unsafe {
        SCHEDULE_THREAD = core::mem::transmute(sched_thread.as_ref());
        let mut cur_task_cx: *const TaskContextImpl = core::mem::transmute(1usize);
        llvm_asm!("" :: "{a2}" (args.as_ptr()), "{a3}" (args.len()): "memory": "volatile");
        __switch(&mut cur_task_cx, *get_sched_cx());
    }

    panic!("bug")
}

pub fn schedule(_a0: usize, _a1: usize, args: &[&[&str]]) {
    println!("schedule");
    let d1 = get_duration();
    for &arg in args {
        let thread = Thread::new_thread(arg[0], arg);
        SCHEDULER.critical_section(|s| s.add_thread(thread));
        info!("成功创建并添加线程");

        while let Some(next_thread) = SCHEDULER.critical_section(|v| v.get_next()) {
            let status = next_thread.inner.lock().status;
            match status {
                ThreadStatus::Ready => {
                    info!("thread {:?} is ready", next_thread.tid);
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
    info!("run all user threads take {:?}", get_duration() - d1);
    unsafe {
        shutdown();
    }
}

//--------------------------------------------------------------------------------------------------
// Testing
//--------------------------------------------------------------------------------------------------

/// The default runner for unit tests.
#[cfg(test)]
pub fn test_runner(tests: &[&test_types::UnitTest]) {
    // This line will be printed as the test header.
    info!("Running {} tests", tests.len());

    for (i, test) in tests.iter().enumerate() {
        println!();
        info!("{:0>3} {:.<58} 🚀", i + 1, test.name);
        // Run the actual test.
        (test.test_func)();
        // Failed tests call panic!(). Execution reaches here only if the test
        // has passed.
    }
}

/// The `kernel_init()` for unit tests.
#[cfg(test)]
#[no_mangle]
pub fn rust_main(hart_id: usize, dtb_pa: PA) -> ! {
    use crate::arch::cpu;

    kernel_init(hart_id, dtb_pa);

    // 调用编译器生成的 test_main 函数
    test_main();

    unsafe { cpu::shutdown() }
}
