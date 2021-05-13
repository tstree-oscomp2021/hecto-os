#![no_std]
#![no_main]
#![feature(global_asm, format_args_nl)]

use kernel::*;

global_asm!(include_str!("entry.asm"));

const BOOT_HART_ID: usize = 0;

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
        fs::init();
        // fs::test_fat32();
        SCHEDULER.lock(|s| s.add_thread(Thread::new_thread("open", None)));
    }
    interrupt::init();

    info!("运行用户线程");
    loop {
        if let Some(next_thread) = SCHEDULER.lock(|v| v.get_next()) {
            let next_task_cx = next_thread.task_cx;
            let cur_task_cx2: &&TaskContext = PROCESSORS[hart::get_hart_id()].lock(|p| {
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
