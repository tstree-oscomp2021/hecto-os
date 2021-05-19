use crate::board::interface::Config;

pub struct ConfigImpl;

impl Config<1> for ConfigImpl {
    const CLOCK_FREQ: usize = 12_500_000;
    const KERNEL_HEAP_SIZE: usize = 0x20_0000;
    /// 内核使用线性映射的偏移量
    const KERNEL_MAP_OFFSET: usize = 0xFFFF_FFC0_0000_0000;
    const KERNEL_STACK_SIZE: usize = 2 << 13;
    const MEMORY_SIZE: usize = 0x80_0000;
    const MEMORY_START: usize = 0xFFFF_FFC0_8000_0000;
    const MMIO: [(usize, usize); 1] = [(0x10001000, 0x1000)];
    const PAGE_SIZE: usize = 0x1000;
    const PAGE_SIZE_BITS: usize = 12;
    const USER_STACK_SIZE: usize = 2 << 13;
}
