# 我们将会用一个宏来用循环保存寄存器。这是必要的设置
.altmacro
.set    REG_SIZE, 8             # 寄存器宽度对应的字节数
.set    CONTEXT_SIZE, 34        # TrapFrame 的大小

# 宏：将寄存器存到栈上
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
# 宏：将寄存器从栈中取出
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
.macro RESTORE_SYS_GP
    .option push
    .option norelax
    1:
        auipc gp, %pcrel_hi(__global_pointer$)
        addi  gp, gp, %pcrel_lo(1b)
    .option pop
.endm

    .section .text
    .globl __trap
# 进入中断
__trap:
# 保存 Context
    # 如果是从 U->S，那么 sp = sscratch
    # 如果是从 S->S，那么 sp = sp
    mv      t0, sp          # 读取 sp
    csrr    t1, sstatus     # 读取 sstatus 寄存器

    andi    t2, t1, 0x100   # 将 SPP 位的值读取到 t2
    # 如果 t2 不为 0 则跳转到 .save_context
    bnez    t2, .save_context
    csrr    sp, sscratch    # 从 U trap 到 S，将 sp 设为内核栈顶
.save_context:
    addi    sp, sp, -CONTEXT_SIZE * REG_SIZE    # 在内核栈开辟 Context 的空间

    sd      t0, 2*8(sp)     # 保存 x2(sp) 寄存器
    sd      t1, 32*8(sp)    # 保存 sstatus 寄存器

    csrr    t1, sepc        # 读取 sepc 寄存器
    sd      t1, 33*8(sp)    # 保存 sepc 寄存器

    # SAVE_GP 0             # x0 不用保存，因为它固定为 0
    SAVE_GP 1               # 保存 x1 寄存器
                            # 暂时略过 x2(sp) 寄存器，之后再保存
    SAVE_GP 3               # 保存 x3 寄存器
                            # 略过 x4(tp) 寄存器，tp 寄存器用来保存 cpuid
    .set    n, 4            # 保存 x5 ~ x31 寄存器
    .rept   27
        .set    n, n + 1    # n = n + 1
        SAVE_GP %n          # 保存 xn 寄存器
    .endr

    RESTORE_SYS_GP          # 恢复内核的 gp 寄存器

# 因为可能出现中断嵌套，所以 sstatus sepc 也需要保存，scause stval 与特权级的切换并没有关系。
# 设置参数，然后调用 handle_trap

    csrr    a0, scause      # scause: Scause
    csrr    a1, stval       # stval: usize
    mv      a2, t2          # sstatus 的 SPP 位
    jal     handle_trap



# 从 handle_trap 函数返回。此时的 sp 正是 jal handle_trap 前的 sp
    .globl __restore
__restore:

    addi    t0, sp, CONTEXT_SIZE * REG_SIZE     # 获取内核栈顶地址
    csrw    sscratch, t0                        # 写入 sscratch

    ld      t0, 32*8(sp)    # 读取 Contex 中的 sstatus 寄存器
    ld      t1, 33*8(sp)    # 读取 Contex 中的 sepc 寄存器
    csrw    sstatus, t0     # 恢复 sstatus 寄存器
    csrw    sepc, t1        # 恢复 sepc 寄存器

    # 恢复通用寄存器
    LOAD_GP 1               # 恢复 x1 寄存器
                            # 暂时略过 x2(sp) 寄存器，之后再恢复
    LOAD_GP 3               # 恢复 x3 寄存器
                            # 略过 x4(tp) 寄存器，tp 寄存器用来保存 cpuid
    .set    n, 4            # 恢复 x5 ~ x31 寄存器
    .rept   27
        .set    n, n + 1    # n = n + 1
        LOAD_GP %n          # 恢复 xn 寄存器
    .endr
    LOAD_GP 2               # 恢复 x2(sp) 寄存器

    sret
