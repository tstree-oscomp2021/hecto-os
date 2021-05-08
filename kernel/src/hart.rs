#![allow(unused)]

/// 保存 hart_id
pub unsafe fn set_hart_id(hart_id: usize) {
    llvm_asm!("mv tp, $0" : : "r"(hart_id));
}

/// 获取当前 hart id
pub fn get_hart_id() -> usize {
    let hart_id;
    unsafe {
        llvm_asm!("mv $0, tp" : "=r"(hart_id));
    }
    hart_id
}

/// 向某 hart 发送处理器间中断以唤醒它
pub fn send_ipi(hart_id: usize) {
    super::sbi::send_ipi(1 << hart_id);
}

/// 休眠，等待中断
pub fn halt() {
    unsafe { riscv::asm::wfi() }
}

/// 关机
pub unsafe fn shutdown() -> ! {
    super::sbi::shutdown()
}

/// 休眠，等待中断
pub fn wait_for_interrupt() {
    unsafe {
        let sie = riscv::register::sstatus::read().sie();
        // 打开当前处理器的全局中断使能
        riscv::register::sstatus::set_sie();
        // 等待中断
        riscv::asm::wfi();
        // 如果之前没开中断使能，就关闭中断使能
        if !sie {
            riscv::register::sstatus::clear_sie();
        }
    }
}
