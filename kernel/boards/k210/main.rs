global_asm!(include_str!("entry.asm"));

pub mod config;

use k210_hal::prelude::*;
use k210_pac::Peripherals;
use k210_soc::{sleep, sysctl};

use crate::{arch::cpu, PA};

pub fn init_board(hart_id: usize, _dtb_pa: PA) {
    unsafe {
        // 保存 hart_id
        cpu::set_cpu_id(hart_id);
        // 等待 sbi 输出完
        sleep::usleep(100000);
        // 配置系统时钟和串口
        sysctl::pll_set_freq(sysctl::pll::PLL0, 800_000_000).unwrap();
        sysctl::pll_set_freq(sysctl::pll::PLL1, 300_000_000).unwrap();
        sysctl::pll_set_freq(sysctl::pll::PLL2, 45_158_400).unwrap();
        let clocks = k210_hal::clock::Clocks::new();
        let peripherals = Peripherals::steal();
        peripherals.UARTHS.configure(115_200.bps(), &clocks);
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
