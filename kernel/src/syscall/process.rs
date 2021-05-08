use super::*;

use crate::{hart::*, process::*};
use algorithm::Scheduler;

pub(super) fn sys_exit(_status: isize) -> ! {
    unsafe {
        // 1. unmap 当前线程的用户栈
        let mut cur_thread = PROCESSORS[get_hart_id()]
            .lock()
            .current_thread
            .take()
            .unwrap();
        SCHEDULER.lock().remove_thread(&cur_thread);
        Arc::get_mut_unchecked(&mut cur_thread).dealloc_user_stack();
        // 2. 切换到另一个线程
        let next_task_cx: &TaskContext =
            core::mem::transmute(PROCESSORS[get_hart_id()].lock().idle_task_cx);
        let cur_task_cx: &TaskContext = core::mem::transmute(1usize);
        __switch(&cur_task_cx, next_task_cx);
    }
    unreachable!()
}
