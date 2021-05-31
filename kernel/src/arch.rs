pub mod interface {
    pub use crate::{
        logger::interface::Console,
        mm::interface::{PageTable, PTE},
        process::interface::TaskContext,
        syscall::interface::Syscall,
        trap::interface::{Trap, TrapFrame},
    };

    pub trait Register {
        fn sp() -> usize;
        fn fp() -> usize;
        fn ra() -> usize;
    }
}

#[cfg(target_arch = "riscv64")]
#[path = "../arch/riscv/mod.rs"]
mod arch_impl;

pub use arch_impl::{
    console::ConsoleImpl,
    cpu,
    page_table::{PTEImpl, PageTableImpl},
    register::RegisterImpl,
    switch::__switch,
    syscall::SyscallImpl,
    task_context::TaskContextImpl,
    trap::{TrapImpl, __restore, __trap, ret_to_restore},
    trap_context::TrapFrameImpl,
};
