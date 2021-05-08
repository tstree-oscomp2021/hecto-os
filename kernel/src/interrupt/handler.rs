use super::timer;
use crate::syscall::syscall_handler;
use riscv::register::{
    scause::{Exception, Interrupt, Scause, Trap},
    sepc, stvec,
};

global_asm!(include_str!("./interrupt.asm"));

/// 初始化中断处理
///
/// 把中断入口 `__interrupt` 写入 `stvec` 中，开启一些中断以接收按键信息
pub fn init() {
    unsafe {
        extern "C" {
            /// `interrupt.asm` 中的中断入口
            fn __interrupt();
        }
        // 使用 Direct 模式，将中断入口设置为 `__interrupt`
        stvec::write(__interrupt as usize, stvec::TrapMode::Direct);

        // // 开启 S 态外部中断
        // sie::set_sext();
        // // 开启 S 态软件中断
        // sie::set_ssoft();
    }
}

/// 中断处理入口
#[no_mangle]
pub fn handle_interrupt(scause: Scause, stval: usize) {
    // log::info!(
    //     "handle_interrupt. sp:{:x} kernel_stack_top: {:x} {:?}",
    //     sp(),
    //     sp() & !(KERNEL_STACK_SIZE - 1),
    //     scause.cause()
    // );

    match scause.cause() {
        // 来自用户态的系统调用
        Trap::Exception(Exception::UserEnvCall) => syscall_handler(),
        // 时钟中断
        Trap::Interrupt(Interrupt::SupervisorTimer) => supervisor_timer(),
        // 外部中断
        Trap::Interrupt(Interrupt::SupervisorExternal) => unimplemented!(),
        // 缺页异常
        // Trap::Exception(Exception::LoadPageFault)
        // | Trap::Exception(Exception::StorePageFault)
        // | Trap::Exception(Exception::InstructionPageFault)
        // | Trap::Exception(Exception::LoadFault)
        // | Trap::Exception(Exception::StoreFault)
        // | Trap::Exception(Exception::InstructionFault) => unimplemented!(),
        // 其他情况，无法处理
        _ => {
            panic!(
                "cause: {:?}, stval: {:x}, sepc: {:x}",
                scause.cause(),
                stval,
                sepc::read()
            );
        }
    }
    // log::info!("handle_interrupt end");
}

/// 处理时钟中断
fn supervisor_timer() {
    timer::tick();
}
