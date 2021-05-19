pub mod process;
pub mod processor;
pub mod thread;

pub use process::{Pid, Process, KERNEL_PROCESS};
pub use processor::{PROCESSORS, SCHEDULER};
pub use thread::{
    get_cur_kernel_stack_top, get_current_thread, get_current_trapframe, get_kernel_stack_range,
    Thread, ThreadStatus,
};

pub mod interface {
    /// 函数调用上下文：在控制流转移前后需要保持不变的寄存器
    /// 一部分由调用者保存，一部分由被调用者保存
    /// __switch 就是一个函数调用，会保存由被调用者保存的寄存器，
    /// 然后切换到另一个线程
    pub trait TaskContext {
        fn set_ra(&mut self, value: usize) -> &mut Self;
    }
}
