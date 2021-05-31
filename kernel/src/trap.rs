use crate::{get_current_thread, VA};

pub mod interface {
    /// 发生中断时，保存的寄存器
    pub trait TrapFrame: Default {
        /// 获取栈指针
        fn sp(&self) -> usize;

        /// 设置栈指针
        fn set_sp(&mut self, value: usize) -> &mut Self;

        /// 获取返回地址
        fn ra(&self) -> usize;

        /// 设置返回地址
        fn set_ra(&mut self, value: usize) -> &mut Self;

        /// 设置返回值
        fn set_return_value(&mut self, value: usize) -> &mut Self;

        /// 设置入口
        fn set_entry_point(&mut self, value: usize) -> &mut Self;

        /// 按照函数调用规则写入参数
        fn set_arguments(&mut self, arguments: &[usize]) -> &mut Self;

        /// 为线程构建初始 `TrapFrameImpl`
        fn init(
            &mut self,
            stack_top: usize,
            entry_point: usize,
            arguments: Option<&[usize]>,
            is_user: bool,
        );
    }

    pub trait Trap {
        fn init();
    }
}

pub fn handle_pagefault(addr: usize) {
    get_current_thread()
        .process
        .inner
        .lock()
        .address_space
        .handle_pagefault(VA(addr));
}
