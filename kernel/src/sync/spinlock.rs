//! 一个带关中断功能的互斥锁 [`SpinLock`]

use spin::{Mutex, MutexGuard};

/// 关闭中断的互斥锁
#[derive(Default)]
pub struct SpinLock<T>(Mutex<T>);

/// 封装 [`MutexGuard`] 来实现 drop 时恢复 sstatus
pub struct LockGuard<'a, T> {
    /// 在 drop 时需要先 drop 掉 [`MutexGuard`] 再恢复 sstatus
    guard: Option<MutexGuard<'a, T>>,
    /// 保存的关中断前 sstatus
    sstatus: usize,
}

impl<T> SpinLock<T> {
    /// 创建一个新对象
    pub const fn new(obj: T) -> Self {
        Self(Mutex::new(obj))
    }

    /// 进入临界区
    pub fn critical_section<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let sstatus: usize;
        unsafe {
            // 清除 SIE 中断使能位，并保存 sstatus
            llvm_asm!("csrrci $0, sstatus, 1 << 1" : "=r"(sstatus) ::: "volatile");
        }
        let ret = f(&mut self.0.lock());
        // 将 SIE 位恢复为之前的状态
        unsafe { llvm_asm!("csrs sstatus, $0" :: "r"(sstatus & 2) :: "volatile") };

        ret
    }

    /// 获得上锁的对象
    pub fn lock(&self) -> LockGuard<'_, T> {
        let sstatus: usize;
        unsafe {
            llvm_asm!("csrrci $0, sstatus, 1 << 1" : "=r"(sstatus) ::: "volatile");
        }
        LockGuard {
            guard: Some(self.0.lock()),
            sstatus,
        }
    }
}

/// 释放时，先释放内部的 MutexGuard，再恢复 sstatus 寄存器
impl<'a, T> Drop for LockGuard<'a, T> {
    fn drop(&mut self) {
        self.guard.take();
        unsafe { llvm_asm!("csrs sstatus, $0" :: "r"(self.sstatus & 2) :: "volatile") };
    }
}

impl<'a, T> core::ops::Deref for LockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap().deref()
    }
}

impl<'a, T> core::ops::DerefMut for LockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().unwrap().deref_mut()
    }
}
