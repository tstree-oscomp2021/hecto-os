use algorithm::Scheduler;

use super::*;
use crate::{
    arch::{TaskContextImpl, __switch},
    process::*,
    processor::current_processor,
    trap::interface::TrapFrame,
};

/// 线程退出
/// 如果是进程中的最后一个线程，进程也退出，向父进程发送消息
pub(super) fn sys_exit(_status: isize) -> ! {
    unsafe {
        // 1. unmap 当前线程的用户栈
        let mut cur_thread = Arc::from_raw(get_current_thread());
        cur_thread.inner.lock().status = ThreadStatus::Zombie;
        SCHEDULER.lock(|s| s.remove_thread(&cur_thread));
        Arc::get_mut_unchecked(&mut cur_thread).dealloc_user_stack();
        // 此时引用计数应为 1，线程将在调度器处析构
        // debug!("引用计数 {}", alloc::sync::Arc::strong_count(&cur_thread));
        // 防止析构，因为这是个从裸指针构造的 Arc。但 __switch 不会返回，所以没必要
        // core::mem::forget(cur_thread);
        // 2. 切换到另一个线程
        let next_task_cx: &TaskContextImpl =
            core::mem::transmute(current_processor().lock(|p| p.idle_task_cx));
        let cur_task_cx: &TaskContextImpl = core::mem::transmute(1usize);
        __switch(&cur_task_cx, next_task_cx);
    }
    unreachable!()
}

/// TODO stack 为 0 的情况、设置参数
pub(super) fn sys_clone(
    _flags: u64,
    stack: *mut usize,
    _parent_tid: *mut usize,
    _tls: usize,
    _child_tid: *mut usize,
) -> isize {
    let new_thread = get_current_thread().fork();
    let trap_frame = new_thread.get_trapframe();
    trap_frame.set_sp(stack as usize);
    trap_frame.set_entry_point(unsafe { *stack.offset(0) });

    SCHEDULER.lock(|s| s.add_thread(new_thread.clone()));
    // 让新的线程先一步调度
    get_current_thread().switch_to(&new_thread);

    new_thread.get_tid() as isize
}
