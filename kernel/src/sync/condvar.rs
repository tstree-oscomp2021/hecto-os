use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
};

use crate::{get_current_thread, sync::SpinLock, Thread, ThreadStatus};

/// A Condition Variable
///
/// Condition variables represent the ability to block a thread such that it
/// consumes no CPU time while waiting for an event to occur.
#[derive(Default)]
pub struct Condvar {
    wait_queue: SpinLock<VecDeque<Weak<Thread>>>,
}

impl Condvar {
    /// Mark the current thread state as waiting, and will not do the actual
    /// sleep operation
    pub fn wait(&self) {
        // Arc::from_raw 不会增加引用计数
        let cur_thead = unsafe { Arc::from_raw(get_current_thread()) };
        cur_thead.inner.lock().status = ThreadStatus::Waiting;
        self.wait_queue
            .critical_section(|q| q.push_back(Arc::downgrade(&cur_thead)));
        // 防止减少引用计数
        core::mem::forget(cur_thead);
    }

    pub fn notify_one(&self) {
        if let Some(thread) = self.wait_queue.critical_section(|q| q.pop_front()) {
            if let Some(thread) = thread.upgrade() {
                thread.inner.lock().status = ThreadStatus::Ready;
            }
        }
    }

    pub fn notify_all(&self) {
        while let Some(thread) = self.wait_queue.critical_section(|q| q.pop_front()) {
            if let Some(thread) = thread.upgrade() {
                thread.inner.lock().status = ThreadStatus::Ready;
            }
        }
    }
}
