# 从 rustsbi 跳转至 _start 时，a0 = hartid, a1 = dtb_pa

    .section .text.entry
    .globl _start
_start:
    # %pcrel_hi 取得 boot_page_table 距离 PC 的相对地址的高 20 位
    # auipc 将 %pcrel_hi(boot_page_table) 的值左移 12 位，与 PC 相加，存入 t0
    auipc   t0, %pcrel_hi(boot_page_table)
    # %pcrel_lo(_start) 是 _start 的低 12 位，相加得到 boot_page_table 的物理地址
    addi    t0, t0, %pcrel_lo(_start)
    srli    t0, t0, 12              # 右移 12 位，得到 boot_page_table 的物理页号
    li      t1, (8 << 60)           # 8 << 60 是 satp 中使用 Sv39 模式的记号
    or      t0, t0, t1              #
    csrw    satp, t0                # 写入 satp 并更新 TLB
    sfence.vma

    li t1, 0xffffffc000000000       # 虚拟地址的偏移量
.A: # 将 sp 设置为 boot_stack_top
    auipc   sp, %pcrel_hi(boot_stack_top)
    addi    sp, sp, %pcrel_lo(.A)   # 得到物理地址
    add     sp, sp, t1              # 得到虚拟地址
.B: # 跳转至 rust_main
    auipc   t0, %pcrel_hi(rust_main)
    addi    t0, t0, %pcrel_lo(.B)   # 得到物理地址
    add     t0, t0, t1              # 得到虚拟地址
    jr      t0



# 4K 大小的 boot_stack，放在 .bss，如果放在 .data 会占用空间
    .section .bss.stack
    .align 12
    .globl boot_stack
boot_stack:
    .zero 4096 * 1
    .globl boot_stack_top
boot_stack_top:

# 初始内核映射所用的页表
    .section .data.page_table
    .align 12
    .globl boot_page_table
boot_page_table:
    .zero 8 * 2
    # PPN[2]=2   0000_0000_8000_0000 -> 0000_0000_8000_0000 0xcf 表示 VRWXAD 均为 1
    .8byte (0x80000 << 10) | 0xcf
    .zero 8 * 253
    # PPN[2]=256 FFFF_FFC0_0000_0000 -> 0000_0000_0000_0000 0xcf 表示 VRWXAD 均为 1
    .8byte (0x00000 << 10) | 0xcf
    # PPN[2]=257 FFFF_FFC0_4000_0000 -> 0000_0000_4000_0000 0xcf 表示 VRWXAD 均为 1
    .8byte (0x40000 << 10) | 0xcf
    # PPN[2]=258 FFFF_FFC0_8000_0000 -> 0000_0000_8000_0000 0xcf 表示 VRWXAD 均为 1
    .8byte (0x80000 << 10) | 0xcf
    # PPN[2]=259 FFFF_FFC0_C000_0000 -> 0000_0000_C000_0000 0xcf 表示 VRWXAD 均为 1
    .8byte (0xC0000 << 10) | 0xcf
    .zero 8 * 252
