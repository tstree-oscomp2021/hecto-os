# 我们将会用一个宏来用循环保存寄存器。这是必要的设置
.altmacro
.set    REG_SIZE, 8             # 寄存器宽度对应的字节数
.set    TRAP_FRAME_SIZE, 34     # TrapFrame 的大小

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
# U->S
# U->S->S，S态中，sscratch必须时刻为 0
__trap:
# 保存 Context
    # 如果 sscratch != 0，则说明是 U->S
    # 如果 sscratch == 0，则说明是 S->S
    csrrw   sp, sscratch, sp    # 交换 sp 和 sscratch
    bnez    sp, .save_context
    csrr    sp, sscratch        # sp（如果是 S->S）

    # 此时 sp 指向内核栈
.save_context:
    addi    sp, sp, -TRAP_FRAME_SIZE * REG_SIZE    # 在内核栈开辟 TrapFrame 的空间

    # SAVE_GP 0             # x0 不用保存，因为它固定为 0
    SAVE_GP 1               # 保存 x1 寄存器
    .set    n, 2            # 保存 x3 ~ x31 寄存器
    .rept   29
        .set    n, n + 1    # n = n + 1
        SAVE_GP %n          # 保存 xn 寄存器
    .endr

    csrr    t0, sstatus         # 读取 sstatus
    csrr    t1, sepc            # 读取 sepc
    csrrw   t2, sscratch, x0    # 读取 sscratch（原先的 sp），并将 sscratch 置 0
    sd      t0, 32*8(sp)        # 保存 sstatus
    sd      t1, 33*8(sp)        # 保存 sepc
    sd      t2, 02*8(sp)        # 保存 x2(sp)

    RESTORE_SYS_GP          # 恢复给内核代码使用的 x3(gp) 寄存器

# 因为可能出现中断嵌套，所以 sstatus sepc 也需要保存，scause stval 与特权级的切换并没有关系。
# 设置参数，然后调用 handle_trap

    mv      a0, sp          # trap_frame: &mut TrapFrameImpl
    csrr    a1, scause      # scause: Scause
    csrr    a2, stval       # stval: usize
    jal     handle_trap



# 从 handle_trap 函数返回。此时的 sp 正是 jal handle_trap 前的 sp
    .globl __restore
__restore:
    # 如果 S->U，则 sscratch == sp + TRAP_FRAME_SIZE * REG_SIZE
    # 如果 S->S，则 sscratch == 0（无需修改）
    # S->U 有一种情况是 直接调用 __restore，此时 SPP 位应设置为 0

    csrr    t0, sstatus     # 读取 sstatus 寄存器
    andi    t0, t0, 0x100   # 将 SPP 位的值读取到 t0
    # 如果 t0 不为 0，说明是 S->S，直接跳转到 .restore_context
    bnez    t0, .restore_context
    addi    t0, sp, TRAP_FRAME_SIZE * REG_SIZE      # 获取内核栈顶地址
    csrw    sscratch, t0                            # 写入 sscratch
.restore_context:
    ld      t0, 32*8(sp)    # 读取 Contex 中的 sstatus 寄存器
    ld      t1, 33*8(sp)    # 读取 Contex 中的 sepc 寄存器
    csrw    sstatus, t0     # 恢复 sstatus 寄存器
    csrw    sepc, t1        # 恢复 sepc 寄存器

    # 恢复通用寄存器
    LOAD_GP 1               # 恢复 x1 寄存器
    .set    n, 2            # 恢复 x3 ~ x31 寄存器
    .rept   29
        .set    n, n + 1    # n = n + 1
        LOAD_GP %n          # 恢复 xn 寄存器
    .endr
    LOAD_GP 2               # 恢复 x2(sp) 寄存器

    sret
