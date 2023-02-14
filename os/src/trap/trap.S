.altmacro

.macro SAVE_GP n
    sd x\n, 8 * \n(sp)
.endm

.macro LOAD_GP n
    ld x\n, 8 * \n(sp)
.endm

.section .text.trampoline
.global __alltraps
.global __restore
.align 2
__alltraps:
    csrrw sp, sscratch, sp // sp <-> sscratch

    //-------now sp->*TrapContext-----
    /**
     * store all gp regs.
     */
    sd ra, 8(sp) // ra is x1.
    // We won't save sp since it was already
    // changed above.
    // Instead, we save `sscratch`, which stored
    // user stack address.
    // Nonetheless, we save `sscratch` later,
    // Because we have to read csr to some gp then
    // push that gp into stack, but we shouldn't
    // overwrite any gp before they are saved (`ra`
    // has been pushed but it's not a good solution
    // to use ra as temp reg).
    .set n, 3
    .rept 29
        SAVE_GP %n
        .set n, n + 1
    .endr
    /**
     * Store csr. Note that the order should 
     * be the same as defined in context.rs.
     */
    csrr t0, sstatus
    csrr t1, sepc
    // Finally, we can store `sscratch`.
    csrr t2, sscratch
    sd t0, 32 * 8(sp)
    sd t1, 33 * 8(sp)
    sd t2, 2 * 8(sp) // x2 is (user)sp
    // load kernel_satp into t0
    ld t0, 34 * 8(sp)
    // load trap_handler into t1
    ld t1, 36 * 8(sp)
    // move to kernel_sp
    ld sp, 35 * 8(sp)
    // switch to kernel space
    csrw satp, t0
    sfence.vma
    // jump to trap_handler
    jr t1

/**
 * This function can be used when:
 *   1. recovering from trap.
 *   2. starting to run an app
 */
__restore:
    #  a0: *TrapContext in user space(Constant); a1: user space token
    # switch to user space
    csrw satp, a1
    # clear TLB
    sfence.vma

    csrw sscratch, a0
    mv sp, a0
    # now sp points to TrapContext in user space, start restoring based on it
    # restore sstatus/sepc
    ld t0, 32 * 8(sp)
    ld t1, 33 * 8(sp)
    csrw sstatus, t0
    csrw sepc, t1

    # restore general purpose registers except x0/sp/tp
    ld ra, 1 * 8(sp)
    .set n, 3
    .rept 29
        LOAD_GP %n
        .set n, n + 1
    .endr
    # back to user stack
    ld sp, 2 * 8(sp)
    sret
