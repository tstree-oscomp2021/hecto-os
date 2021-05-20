use algorithm::Scheduler;

use super::*;
use crate::{
    arch::{TaskContextImpl, __switch},
    process::*,
    processor::current_processor,
};

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
