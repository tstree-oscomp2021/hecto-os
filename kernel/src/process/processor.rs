//! 实现线程的调度和管理 [`Processor`]

use super::*;
use crate::spinlock::*;
use algorithm::*;
use alloc::sync::Arc;
use hashbrown::HashSet;
use lazy_static::*;

lazy_static! {
    /// 全局的 [`Processor`]，保存每 CPU 变量
    pub static ref PROCESSORS: [SpinLock<Processor>; 2] = [Default::default(), Default::default()];
}

lazy_static! {
    /// 调度器，保存 Ready 状态的线程
    pub static ref SCHEDULER: SpinLock<SchedulerImpl<Arc<Thread>>> = SpinLock::new(SchedulerImpl::default());
}

/// 每 cpu 变量
#[derive(Default)]
pub struct Processor {
    /// 当前正在执行的线程
    pub current_thread: Option<Arc<Thread>>,
    /// 保存休眠线程
    pub sleeping_threads: HashSet<Arc<Thread>>,
    pub idle_task_cx: usize,
}

#[allow(unused)]
impl Processor {
    /// 获取一个当前线程的 `Arc` 引用
    pub fn current_thread(&self) -> Arc<Thread> {
        self.current_thread.as_ref().unwrap().clone()
    }
}
