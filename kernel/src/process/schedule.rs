//! 线程调度

use alloc::sync::Arc;
use core::intrinsics::transmute;

use algorithm::*;

use super::*;
use crate::{arch::TaskContextImpl, sync::SpinLock};

/// 调度器
pub static SCHEDULER: SpinLock<SchedulerImpl<Arc<Thread>>> = SpinLock::new(SchedulerImpl::DEFAULT);

/// 调度线程
pub static mut SCHEDULE_THREAD: *mut Thread = unsafe { core::mem::transmute(0usize) };

/// 获取调度线程的 TaskContext 指针的引用
#[inline]
pub fn get_sched_cx() -> &'static mut *const TaskContextImpl {
    unsafe { transmute(&mut (&mut *SCHEDULE_THREAD).task_cx) }
}
