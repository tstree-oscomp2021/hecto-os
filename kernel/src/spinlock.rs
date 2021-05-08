//! 一个带关中断功能的互斥锁 [`SpinLock`]

use spin::{Mutex, MutexGuard};

/// 关闭中断的互斥锁
#[derive(Default)]
pub struct SpinLock<T>(pub(self) Mutex<T>);

/// 封装 [`MutexGuard`] 来实现 drop 时恢复 sstatus
pub struct SpinLockGuard<'a, T> {
    /// 在 drop 时需要先 drop 掉 [`MutexGuard`] 再恢复 sstatus，不然可能会在还持有锁的时候发生中断
    guard: Option<MutexGuard<'a, T>>,
    /// 保存的关中断前 sstatus
    sstatus: usize,
}

impl<T> SpinLock<T> {
    /// 创建一个新对象
    pub fn new(obj: T) -> Self {
        Self(Mutex::new(obj))
    }

    /// 获得上锁的对象
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        // print!("+");
        let sstatus: usize;
        unsafe {
            llvm_asm!("csrrci $0, sstatus, 1 << 1" : "=r"(sstatus) ::: "volatile");
        }
        SpinLockGuard {
            guard: Some(self.0.lock()),
            sstatus,
        }
    }
}

/// 释放时，先释放内部的 MutexGuard，再恢复 sstatus 寄存器
impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        // print!("-");
        self.guard.take();
        unsafe { llvm_asm!("csrs sstatus, $0" :: "r"(self.sstatus & 2) :: "volatile") };
    }
}

impl<'a, T> core::ops::Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*(self.guard.as_ref().unwrap())
    }
}

impl<'a, T> core::ops::DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().unwrap().deref_mut()
    }
}
