//! 预约和处理时钟中断

use riscv::register::{sie, sstatus, time};

use super::sbi::set_timer;
use crate::board::{interface::Config, ConfigImpl};

/// 触发时钟中断计数
pub static mut TICKS: [usize; 2] = [0; 2];

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;
/// 时钟中断的间隔，单位是 CPU 指令
/// 中断间隔 = 每秒时钟周期数 / 每秒 tick 数 = 每次 tick 经过的时钟周期数
const INTERVAL: usize = ConfigImpl::CLOCK_FREQ / TICKS_PER_SEC;

/// 初始化时钟中断
///
/// 开启时钟中断使能，并且预约第一次时钟中断
pub fn init() {
    unsafe {
        // 开启 STIE，允许时钟中断
        sie::set_stimer();
        // 开启 SIE（不是 sie 寄存器），全局中断使能，允许内核态被中断打断
        // TODO 此处仅用于测试 timer，之后删掉
        // sstatus::set_sie();
    }
    // 设置下一次时钟中断
    set_next_timeout();
}

/// 每一次时钟中断时调用
///
/// 设置下一次时钟中断，同时计数 +1
pub fn tick() {
    // println!(".");
    set_next_timeout();
    let hart_id = super::cpu::get_cpu_id();
    unsafe {
        TICKS[hart_id] += 1;
        if TICKS[hart_id] % TICKS_PER_SEC == 0 {
            debug!("{} 秒", TICKS[hart_id] / TICKS_PER_SEC);
        }
    }
}

/// 设置下一次时钟中断
///
/// 获取当前时间，加上中断间隔，通过 SBI 调用预约下一次中断
#[inline]
fn set_next_timeout() {
    set_timer(time::read() + INTERVAL);
}

#[allow(unused)]
#[inline]
pub fn get_time_ms() -> usize {
    // 指令周期数 / 每毫秒时钟周期数
    time::read() / (ConfigImpl::CLOCK_FREQ / MSEC_PER_SEC)
}
