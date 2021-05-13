//! 一个带关中断功能的互斥锁 [`SpinLock`]

use spin::Mutex;

/// 关闭中断的互斥锁
#[derive(Default)]
pub struct SpinLock<T>(Mutex<T>);

impl<T> SpinLock<T> {
    /// 创建一个新对象
    pub fn new(obj: T) -> Self {
        Self(Mutex::new(obj))
    }

    /// 进入临界区
    pub fn lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
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
}
