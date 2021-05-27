global_asm!(include_str!("entry.asm"));

pub mod config;

use crate::{arch::cpu, PA};

pub fn init_board(hart_id: usize, _dtb_pa: PA) {
    unsafe {
        // 保存 hart_id
        cpu::set_cpu_id(hart_id);
        // 允许内核读写用户态内存
        riscv::register::sstatus::set_sum();
    }
}

/// linker.ld 中的 symbols
pub mod symbol {
    #[allow(dead_code)]
    extern "C" {
        pub fn skernel();
        pub fn stext();
        pub fn etext();
        pub fn srodata();
        pub fn erodata();
        pub fn sdata();
        pub fn edata();
        pub fn sbss_with_stack();
        pub fn sbss();
        pub fn ebss();
        pub fn ekernel();
    }
}
