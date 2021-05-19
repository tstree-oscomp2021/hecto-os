use super::task_context::TaskContextImpl;

global_asm!(include_str!("switch.asm"));
extern "C" {
    /// __switch 调用结束后，*current_task_cx_ptr2 会指向保存在内核栈中的
    /// TaskContext next_task_cx_ptr 指向的 TaskContext 都会被 load
    pub fn __switch(current_task_cx_ptr2: &&TaskContextImpl, next_task_cx_ptr: &TaskContextImpl);
}
