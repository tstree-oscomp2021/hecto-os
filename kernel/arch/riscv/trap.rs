use riscv::register::{
    scause::{Exception, Interrupt, Scause, Trap},
    sepc, stvec,
};

use super::timer;
use crate::{syscall::syscall_handler, trap::handle_pagefault};

global_asm!(include_str!("./trap.asm"));
extern "C" {
    pub fn __trap();
    pub fn __restore();
}

pub struct TrapImpl;

impl crate::arch::interface::Trap for TrapImpl {
    fn init() {
        unsafe {
            // 使用 Direct 模式，将中断入口设置为 `__interrupt`
            stvec::write(__trap as usize, stvec::TrapMode::Direct);

            // // 开启 S 态外部中断
            // sie::set_sext();
            // // 开启 S 态软件中断
            // sie::set_ssoft();
        }

        timer::init();

        info!("mod trap initialized");
    }
}

/// 中断处理入口
#[no_mangle]
pub fn handle_trap(scause: Scause, stval: usize) {
    match scause.cause() {
        // 来自用户态的系统调用
        Trap::Exception(Exception::UserEnvCall) => {
            unsafe {
                riscv::register::sstatus::set_sie();
            }
            syscall_handler()
        }
        // 时钟中断
        Trap::Interrupt(Interrupt::SupervisorTimer) => supervisor_timer(),
        // 外部中断
        Trap::Interrupt(Interrupt::SupervisorExternal) => unimplemented!(),
        // 缺页异常
        Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::InstructionFault) => {
            debug!(
                "cause: {:?}, stval: {:x}, sepc: {:x}",
                scause.cause(),
                stval,
                sepc::read()
            );
            handle_pagefault(stval);
        }
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
    // info!("handle_interrupt end");
}

/// 处理时钟中断
fn supervisor_timer() {
    timer::tick();
}
