# 从 rustsbi 跳转至 _start 时，a0 = hartid, a1 = dtb_pa

    .section .text.entry
    .globl _start
_start:
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
