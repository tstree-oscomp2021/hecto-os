use crate::board::interface::Config;

pub struct ConfigImpl;

impl Config<14> for ConfigImpl {
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
    /// XXX 内存大小 8M
    const MEMORY_SIZE: usize = 0x80_0000;
    /// PAGE_SIZE = 1 << PAGE_SIZE_BITS
    const PAGE_SIZE_BITS: usize = 12;
    /// MMIO 起始地址
    const MMIO: [(usize, usize); 14] = [
        // we don't need clint in S priv when running
        // we only need claim/complete for target0 after initializing
        (0x0C00_0000, 0x3000), /* PLIC */
        (0x0C20_0000, 0x1000), /* PLIC */
        (0x3800_0000, 0x1000), /* UARTHS */
        (0x3800_1000, 0x1000), /* GPIOHS */
        (0x5020_0000, 0x1000), /* GPIO */
        (0x5024_0000, 0x1000), /* SPI_SLAVE */
        (0x502B_0000, 0x1000), /* FPIOA */
        (0x502D_0000, 0x1000), /* TIMER0 */
        (0x502E_0000, 0x1000), /* TIMER1 */
        (0x502F_0000, 0x1000), /* TIMER2 */
        (0x5044_0000, 0x1000), /* SYSCTL */
        (0x5200_0000, 0x1000), /* SPI0 */
        (0x5300_0000, 0x1000), /* SPI1 */
        (0x5400_0000, 0x1000), /* SPI2 */
    ];
    /// 时钟频率
    const CLOCK_FREQ: u64 = 403000000 / 62;
    /// boot cpu id
    const BOOT_CPU_ID: usize = 0;
}
