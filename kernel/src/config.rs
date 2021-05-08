/// 每秒时钟周期数
#[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: usize = 12_500_000;

#[cfg(feature = "board_qemu")]
pub const MMIO: &[(usize, usize)] = &[(0x10001000, 0x1000)];
