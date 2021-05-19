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
        let mut cur_thread = current_processor().lock(|a| a.current_thread.take().unwrap());
        SCHEDULER.lock(|s| s.remove_thread(&cur_thread));
        Arc::get_mut_unchecked(&mut cur_thread).dealloc_user_stack();
        // 2. 切换到另一个线程
        let next_task_cx: &TaskContextImpl =
            core::mem::transmute(current_processor().lock(|p| p.idle_task_cx));
        let cur_task_cx: &TaskContextImpl = core::mem::transmute(1usize);
        __switch(&cur_task_cx, next_task_cx);
    }
    unreachable!()
}
