global_asm!(include_str!("entry.asm"));

pub mod config;

use crate::{
    arch::{
        cpu,
        interface::{PageTable, Trap},
        TaskContextImpl, TrapImpl, __switch,
    },
    processor::current_processor,
    *,
};

const BOOT_HART_ID: usize = 0;

#[no_mangle]
pub fn rust_main(hart_id: usize, _dtb_pa: PA) -> ! {
    unsafe {
        // 保存 hart_id
        cpu::set_cpu_id(hart_id);
        // 允许内核读写用户态内存
        riscv::register::sstatus::set_sum();
    }

    if hart_id == BOOT_HART_ID {
        mm::clear_bss();
        mm::init();
    }

    // 初始化块设备驱动之前先激活新页表
    mm::KERNEL_PAGE_TABLE.activate();

    if hart_id == BOOT_HART_ID {
        fs::init();
        // fs::test_fat32();
        // 添加用户线程
        SCHEDULER.lock(|s| {
            s.add_thread(Thread::new_thread("getcwd", None));
            s.add_thread(Thread::new_thread("openat", None));
            s.add_thread(Thread::new_thread("open", None));
        });
    }
    TrapImpl::init();

    info!("运行用户线程");
    loop {
        if let Some(next_thread) = SCHEDULER.lock(|v| v.get_next()) {
            let next_task_cx = next_thread.task_cx;
            let cur_task_cx2: &&TaskContextImpl = current_processor().lock(|p| {
                next_thread
                    .process
                    .inner
                    .lock()
                    .memory_set
                    .page_table
                    .activate();
                p.current_thread = Some(next_thread);
                unsafe { core::mem::transmute(&p.idle_task_cx) }
            });

            unsafe {
                // hart::send_ipi(1); // 唤醒 hart1
                // 切换线程
                __switch(cur_task_cx2, next_task_cx);
            }
        }
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
