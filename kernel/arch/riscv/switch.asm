//! __switch 函数，在内核态切换至其他线程

.altmacro
.macro SAVE_SN n
    sd s\n, (\n+1)*8(sp)
.endm
.macro LOAD_SN n
    ld s\n, (\n+1)*8(sp)
.endm

    .section .text
    .globl __switch
__switch:
    addi    sp, sp, -13*8       # sp 指向将要保存的 TaskContext
    sd      sp, 0(a0)           # 将 sp 保存到 *current_task_cx_ptr2
    # fill TaskContext with ra & s0-s11
    sd      ra, 0(sp)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr
    # ready for loading TaskContext a1 points to
    mv      sp, a1              # 让 sp 指向 next_task_cx_ptr 所指向的 TaskContext
    # load registers in the TaskContext
    ld ra, 0(sp)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    # pop TaskContext
    addi sp, sp, 13*8
    ret
