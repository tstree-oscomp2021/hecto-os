# 从 rustsbi 跳转至 _start 时，a0 = hartid, a1 = dtb_pa

    .section .text.entry
    .globl _start
_start:
    li      t0, 0x2000
    csrs    sstatus, t0             # 开启 FPU
    csrw    sscratch, x0

    li      t1, 0xffffffc000000000  # 虚拟地址的偏移量

    .option push
    .option norelax
1:
    auipc   gp, %pcrel_hi(__global_pointer$)
    addi    gp, gp, %pcrel_lo(1b)   # 由于此时 pc 仍是物理地址，所以此时 gp 还是物理地址
    add     gp, gp, t1              # 得到虚拟地址
    .option pop

.A: # sp = boot_stack
    auipc   sp, %pcrel_hi(boot_stack)
    addi    sp, sp, %pcrel_lo(.A)   # 得到物理地址
    add     sp, sp, t1              # 得到虚拟地址
    # sp = boot_stack + 0x1000 * a0+1
    li      t2, 4096
    addi    t3, a0, 1
    mul     t2, t2, t3
    c.add   sp, t2

.B: # 跳转至 rust_main
    auipc   t0, %pcrel_hi(rust_main)
    addi    t0, t0, %pcrel_lo(.B)   # 得到物理地址
    add     t0, t0, t1              # 得到虚拟地址
    jr      t0



# boot_stack
    .section .bss.stack
    .align 12
    .globl boot_stack
boot_stack:
    .zero 4096 * 2
    .globl boot_stack_top
boot_stack_top:
