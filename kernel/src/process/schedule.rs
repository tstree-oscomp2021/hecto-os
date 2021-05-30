//! 线程调度

use alloc::sync::Arc;

use algorithm::*;

use super::*;
use crate::{
    arch::{cpu::get_cpu_id, TaskContextImpl},
    sync::SpinLock,
};

/// 调度器
pub static SCHEDULER: SpinLock<SchedulerImpl<Arc<Thread>>> = SpinLock::new(SchedulerImpl::DEFAULT);

/// 调度线程的 TaskContext 指针
static mut SCHEDULER_CX: [*const TaskContextImpl; 2] = unsafe { core::mem::transmute([0usize; 2]) };

/// 获取调度线程的 TaskContext 指针的引用
#[inline]
pub fn get_sched_cx() -> &'static mut *const TaskContextImpl {
    unsafe { &mut SCHEDULER_CX[get_cpu_id()] }
}
