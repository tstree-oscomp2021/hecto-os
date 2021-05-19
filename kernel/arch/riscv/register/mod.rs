//! 通用寄存器
use crate::{arch::interface::Register, impl_register};

pub struct RegisterImpl;

impl Register for RegisterImpl {
    impl_register!(sp fp ra);
}

#[macro_export]
macro_rules! impl_register {
    ($($reg:ident)+) => ($(
        #[inline(always)]
        fn $reg() -> usize {
            let value: usize;
            unsafe {
                llvm_asm!(concat!("mv $0, ", stringify!($reg)) : "=r"(value));
            }
            value
        }
    )+)
}
