#[cfg(feature = "riscv64")]
#[path = "../arch/riscv/mod.rs"]
mod arch_impl;

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

pub type PageTableImpl = arch_impl::page_table::PageTableImpl;
pub type PTEImpl = arch_impl::page_table::PTEImpl;

pub type TrapImpl = arch_impl::trap::TrapImpl;
pub type TrapFrameImpl = arch_impl::trap_context::TrapFrameImpl;
pub type TaskContextImpl = arch_impl::task_context::TaskContextImpl;

pub type SyscallImpl = arch_impl::syscall::SyscallImpl;

pub type ConsoleImpl = arch_impl::console::ConsoleImpl;
pub type RegisterImpl = arch_impl::register::RegisterImpl;

pub use arch_impl::{
    cpu,
    switch::__switch,
    trap::{__restore, __trap},
};
