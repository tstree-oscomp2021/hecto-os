//! 预约和处理时钟中断

use core::time::Duration;

use riscv::register::{sie, time};

use super::sbi::set_timer;
use crate::{
    board::{interface::Config, ConfigImpl},
    timer::TIMER,
};

/// 触发时钟中断计数
pub static mut TICKS: [u64; 2] = [0; 2];

const TICKS_PER_SEC: u64 = 100;
const MSEC_PER_SEC: u64 = 1_000;
const USEC_PER_SEC: u64 = 1_000_000;
const NSEC_PER_SEC: u64 = 1_000_000_000;
/// 时钟中断的间隔，单位是 CPU 指令
/// 中断间隔 = 每秒时钟周期数 / 每秒 tick 数 = 每次 tick 经过的时钟周期数
pub const INTERVAL: u64 = ConfigImpl::CLOCK_FREQ / TICKS_PER_SEC;

/// 初始化时钟中断
///
/// 开启时钟中断使能，并且预约第一次时钟中断
pub fn init() {
    unsafe {
        // 开启 STIE，允许时钟中断
        sie::set_stimer();
    }
    // 设置下一次时钟中断
    set_next_timeout();
}

/// 每一次时钟中断时调用
///
/// 设置下一次时钟中断，同时计数 +1
pub fn tick() {
    set_next_timeout();
    let hart_id = super::cpu::get_cpu_id();
    unsafe {
        TICKS[hart_id] += 1;
        if TICKS[hart_id] % TICKS_PER_SEC == 0 {
            debug!("{} 秒", TICKS[hart_id] / TICKS_PER_SEC);
        }
    }
    TIMER.critical_section(|t| t.expire(super::cpu::get_duration()));
}

/// 设置下一次时钟中断
///
/// 获取当前时间，加上中断间隔，通过 SBI 调用预约下一次中断
#[inline]
fn set_next_timeout() {
    set_timer(time::read() + INTERVAL as usize);
}

#[allow(unused)]
pub fn get_time_ms() -> u64 {
    // 指令周期数 / 每毫秒时钟周期数
    time::read() as u64 / (ConfigImpl::CLOCK_FREQ / MSEC_PER_SEC)
}

/// 返回 (sec, usec)
#[allow(unused)]
pub fn get_time() -> (u64, u64) {
    let mut usec = time::read() as u64 / (ConfigImpl::CLOCK_FREQ / USEC_PER_SEC);

    (usec / USEC_PER_SEC, usec % USEC_PER_SEC)
}

pub fn get_duration() -> Duration {
    let nsec = time::read() as u64 * (NSEC_PER_SEC / 1000) / (ConfigImpl::CLOCK_FREQ / 1000);

    Duration::new(nsec / NSEC_PER_SEC, (nsec % NSEC_PER_SEC) as u32)
}

pub fn get_cycles() -> u64 {
    time::read() as u64
}
