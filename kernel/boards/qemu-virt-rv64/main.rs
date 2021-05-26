global_asm!(include_str!("entry.asm"));

pub mod config;

use crate::{
    arch::{
        TaskContextImpl, TrapImpl, __switch, cpu,
        interface::{PageTable, Trap},
    },
    board::{interface::Config, ConfigImpl},
    processor::*,
    *,
};

#[no_mangle]
pub fn rust_main(hart_id: usize, _dtb_pa: PA) -> ! {
    unsafe {
        // 保存 hart_id
        cpu::set_cpu_id(hart_id);
        // 允许内核读写用户态内存
        riscv::register::sstatus::set_sum();
    }

    if hart_id == ConfigImpl::BOOT_CPU_ID {
        mm::clear_bss();
        mm::init();
        fs::init();
    }

    // remap kernel
    mm::KERNEL_PAGE_TABLE.activate();

    // 初始化调度线程，然后切换至调度线程
    let sched_thread = Thread::init_sched_thread(schedule as usize);
    *get_sched_cx() = sched_thread.task_cx;
    unsafe {
        let mut cur_task_cx: *const TaskContextImpl = core::mem::transmute(1usize);
        __switch(&mut cur_task_cx, *get_sched_cx());
    }

    panic!("有 bug")
}

pub fn schedule() {
    info!("进入调度线程");

    // 添加用户线程
    SCHEDULER.lock(|s| {
        s.add_thread(Thread::new_thread("clone", None));
        s.add_thread(Thread::new_thread("execve", None));
        s.add_thread(Thread::new_thread("getppid", None));
        s.add_thread(Thread::new_thread("getpid", None));
        s.add_thread(Thread::new_thread("dup2", None));
        s.add_thread(Thread::new_thread("dup", None));
        s.add_thread(Thread::new_thread("chdir", None));
        s.add_thread(Thread::new_thread("mkdir_", None));
        s.add_thread(Thread::new_thread("getcwd", None));
        s.add_thread(Thread::new_thread("openat", None));
        s.add_thread(Thread::new_thread("open", None));
    });

    TrapImpl::init();

    info!("运行用户线程");
    loop {
        while let Some(next_thread) = SCHEDULER.lock(|v| v.get_next()) {
            let status = next_thread.inner.lock().status;
            match status {
                ThreadStatus::Ready => {
                    debug!("线程 {:?} 调度开始", next_thread.tid);
                    next_thread.activate();
                    // next_thread.inner.lock().status = ThreadStatus::Running;
                    unsafe {
                        __switch(get_sched_cx(), next_thread.task_cx);
                    }
                }
                _ => {}
            }
        }
        // TODO 没有可运行的线程了，休眠等待
    }
}

/// linker.ld 中的 symbols
pub mod symbol {
    #[allow(dead_code)]
    extern "C" {
        pub fn skernel();
        pub fn stext();
        pub fn etext();
        pub fn srodata();
        pub fn erodata();
        pub fn sdata();
        pub fn edata();
        pub fn sbss_with_stack();
        pub fn sbss();
        pub fn ebss();
        pub fn ekernel();
    }
}
