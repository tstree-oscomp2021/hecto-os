# 从 rustsbi 跳转至 _start 时，a0 = hartid, a1 = dtb_pa

    .section .text.entry
    .globl _start
_start:
    li x1, 0
    li x2, 0
    li x3, 0
    li x4, 0
    li x5, 0
    li x6, 0
    li x7, 0
    li x8, 0
    li x9, 0
    li x10,0
    li x11,0
    li x12,0
    li x13,0
    li x14,0
    li x15,0
    li x16,0
    li x17,0
    li x18,0
    li x19,0
    li x20,0
    li x21,0
    li x22,0
    li x23,0
    li x24,0
    li x25,0
    li x26,0
    li x27,0
    li x28,0
    li x29,0
    li x30,0
    li x31,0

    # /* set to disable FPU */
    # li t0, 0x00006000U
    # csrc sstatus, t0
    # li t0, 0x40000 // SUM in sstatus
    # csrs sstatus, t0

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
    .zero 4096 * 2
    .globl boot_stack_top
boot_stack_top:
