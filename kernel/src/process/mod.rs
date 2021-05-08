pub mod process;
pub mod processor;
pub mod thread;

pub use process::{Pid, Process, KERNEL_PROCESS};
pub use processor::{PROCESSORS, SCHEDULER};
pub use thread::{get_kernel_stack_range, TaskContext, Thread, ThreadStatus};

global_asm!(include_str!("switch.S"));
extern "C" {
    /// __switch 调用结束后，*current_task_cx_ptr2 会指向保存在内核栈中的 TaskContext
    /// next_task_cx_ptr 指向的 TaskContext 都会被 load
    pub fn __switch(current_task_cx_ptr2: &&TaskContext, next_task_cx_ptr: &TaskContext);
}
