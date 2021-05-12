#![no_std]
#![no_main]
#![feature(
    global_asm,
    llvm_asm,
    panic_info_message,
    alloc_error_handler,
    drain_filter,
    linked_list_remove,
    format_args_nl,
    step_trait_ext,
    step_trait,
    rustc_attrs,
    map_first_last,
    get_mut_unchecked
)]
// #![allow(dead_code)]
// #![allow(unused)]

#[macro_use]
mod logger;
mod backtrace;
mod config;
mod drivers;
mod ffi;
mod fs;
mod hart;
mod interrupt;
mod mm;
mod process;
mod register;
mod sbi;
mod spinlock;
mod syscall;

extern crate alloc;
extern crate core_io;
use core_io as io;

use algorithm::*;
use fs::*;
use mm::*;
use process::*;

const BOOT_HART_ID: usize = 0;

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main(hart_id: usize, _dtb_pa: PA) -> ! {
    unsafe {
        // 保存 hart_id
        hart::set_hart_id(hart_id);
        // 允许内核读写用户态内存
        riscv::register::sstatus::set_sum();
    }

    if hart_id == BOOT_HART_ID {
        mm::clear_bss();
        mm::init();
    }

    // 初始化块设备驱动之前先激活新页表
    mm::KERNEL_PAGE_TABLE.activate();

    // 添加线程，至少一个
    if hart_id == BOOT_HART_ID {
        // 初始化块设备驱动和文件系统
        lazy_static::initialize(&FILE_SYSTEM);
        // fs::test_fat32();
        SCHEDULER
            .lock()
            .add_thread(Thread::new_thread("open", None));
    }
    interrupt::init();

    info!("运行用户线程");
    loop {
        // 此处用闭包的目的是为了让 SpinLockGuard 释放，防止死锁
        if let Some(next_thread) = { || SCHEDULER.lock().get_next() }() {
            let next_task_cx = next_thread.task_cx;
            let mut processor = PROCESSORS[hart::get_hart_id()].lock();
            processor.current_thread = Some(next_thread);
            let cur_task_cx2: &&TaskContext =
                unsafe { core::mem::transmute(&processor.idle_task_cx) };
            core::mem::drop(processor);
            unsafe {
                // hart::send_ipi(1); // 唤醒 hart1
                // 切换线程
                __switch(cur_task_cx2, next_task_cx);
            }
        }
    }
}
