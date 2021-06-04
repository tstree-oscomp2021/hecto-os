use crate::board::interface::Config;

pub struct ConfigImpl;

impl Config<1> for ConfigImpl {
    /// 内核使用线性映射的偏移量
    const KERNEL_MAP_OFFSET: usize = 0xFFFF_FFC0_0000_0000;
    /// 用户栈大小
    const USER_STACK_SIZE: usize = 1 << 13;
    /// 每个内核栈的栈顶都为 1 << KERNEL_STACK_SIZE_BITS 的倍数
    const KERNEL_STACK_ALIGN_BITS: usize = 14;
    /// 内核栈大小，最大为 1 << KERNEL_STACK_SIZE_BITS - PAGE_SIZE
    const KERNEL_STACK_SIZE: usize = 1 << 13;
    /// 内核堆大小
    const KERNEL_HEAP_SIZE: usize = 0x20_0000;
    /// 内存起始地址
    const MEMORY_START: usize = 0xFFFF_FFC0_8000_0000;
    /// 内存大小
    const MEMORY_SIZE: usize = 0x80_0000;
    /// PAGE_SIZE = 1 << PAGE_SIZE_BITS
    const PAGE_SIZE_BITS: usize = 12;
    /// MMIO 起始地址
    const MMIO: [(usize, usize); 1] = [(0x10001000, 0x1000)];
    /// 时钟频率
    const CLOCK_FREQ: u64 = 10_000_000;
    /// boot cpu id
    const BOOT_CPU_ID: usize = 0;
}
