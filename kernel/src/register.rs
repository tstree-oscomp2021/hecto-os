//! 通用寄存器
//!

///TODO 用宏
#[inline(always)]
pub fn sp() -> usize {
    let ptr: usize;
    unsafe {
        llvm_asm!("mv $0, sp" : "=r"(ptr));
    }
    ptr
}

#[inline(always)]
pub fn fp() -> usize {
    let ptr: usize;
    unsafe {
        llvm_asm!("mv $0, s0" : "=r"(ptr));
    }
    ptr
}

#[inline(always)]
pub fn ra() -> usize {
    let ptr: usize;
    unsafe {
        llvm_asm!("mv $0, ra" : "=r"(ptr));
    }
    ptr
}
