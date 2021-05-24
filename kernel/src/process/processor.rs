//! 实现线程的调度和管理 [`Processor`]

use alloc::sync::Arc;

use algorithm::*;
use hashbrown::HashSet;
use lazy_static::*;

use super::*;
use crate::{
    arch::{cpu::get_cpu_id, TaskContextImpl},
    spinlock::*,
};

lazy_static! {
    /// 调度器，保存 Ready 状态的线程
    pub static ref SCHEDULER: SpinLock<SchedulerImpl<Arc<Thread>>> = SpinLock::new(SchedulerImpl::default());
}

static mut SCHEDULER_CX: [*const TaskContextImpl; 2] = unsafe { core::mem::transmute([0usize; 2]) };

#[inline]
pub fn get_sched_cx() -> &'static mut *const TaskContextImpl {
    unsafe { &mut SCHEDULER_CX[get_cpu_id()] }
}

/// 每 cpu 变量
#[derive(Default)]
pub struct Processor {
    /// 保存休眠线程
    pub sleeping_threads: HashSet<Arc<Thread>>,
}

// fn get_runnable_thread() -> Some(Arc<Thread>) {
//     SCHEDULER.lock(|s| loop {
//         if let Some(next) = s.get_next() {
//             if next== get

//             match next.inner.lock().status {
//                 ThreadStatus::Running => {

//                 }
//                 ThreadStatus::Ready => {}
//                 ThreadStatus::Waiting => {}
//                 ThreadStatus::Zombie => {}
//             }
//         } else {
//             return None;
//         }
//     });

//     todo!()
// }

// fn schedule() {
//     if get_cpu_id() == ConfigImpl::BOOT_CPU_ID {
//         // 添加用户线程
//         SCHEDULER.lock(|s| {
//             s.add_thread(Thread::new_thread("clone", None));
//             s.add_thread(Thread::new_thread("dup2", None));
//             s.add_thread(Thread::new_thread("dup", None));
//             s.add_thread(Thread::new_thread("chdir", None));
//             s.add_thread(Thread::new_thread("mkdir_", None));
//             s.add_thread(Thread::new_thread("getcwd", None));
//             s.add_thread(Thread::new_thread("openat", None));
//             s.add_thread(Thread::new_thread("open", None));
//         });
//     }
//     info!("运行用户线程");

//     loop {
//         if let Some(next_thread) = SCHEDULER.lock(|v| v.get_next()) {
//             let next_task_cx = next_thread.task_cx;
//             let cur_task_cx2: &&TaskContextImpl =
// current_processor().lock(|p| {                 next_thread
//                     .process
//                     .inner
//                     .lock()
//                     .memory_set
//                     .page_table
//                     .activate();
//                 unsafe { core::mem::transmute(&p.idle_task_cx) }
//             });

//             unsafe {
//                 // hart::send_ipi(1); // 唤醒 hart1
//                 // 切换线程
//                 __switch(cur_task_cx2, next_task_cx);
//             }
//         }
//     }
// }
