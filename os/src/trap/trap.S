.altmacro

.macro SAVE_GP n
    sd x\n, 8 * \n(sp)
.endm

.macro LOAD_GP n
    ld x\n, 8 * \n(sp)
.endm

.section .text
.global __alltraps
.align 2
__alltraps:
    /**
     * At the beginnig, sp->user_stack, sscratch->kernel_stack.
     * Use csrrw to swap the values.
     * After this, we have switched from
     * user stack to kernel stack. And
     * sscratch helps us remember the
     * original value of sp (i.e., user stack).
     */
    csrrw sp, sscratch, sp // sp <-> sscratch

    //-------now we are on kernel stack-----
    /**
     * Allocate space
     */
    addi sp, sp, -34 * 8
    /**
     * store all gp regs.
     */
    sd ra, 8(sp) // x1 is ra.
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
    /**
     * Pass sp as the first arg to trap_handler.
     */
    mv a0, sp
    jal trap_handler

.section .text
.global __restore
.align 2
/**
 * This function can be used when:
 *   1. recovering from trap.
 *   2. starting to run an app
 */
__restore:
    mv sp, a0 // usage: __restore(&context)

    ld t0, 32 * 8(sp)
    ld t1, 33 * 8(sp)
    ld t2, 2 * 8(sp) // x2 is (user)sp
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2

    ld ra, 8(sp) // x1 is ra.
    .set n, 3
    .rept 29
        LOAD_GP %n
        .set n, n + 1
    .endr

    addi sp, sp, 34 * 8

    /** After executing this instruction,
     * sp->user_stack
     * sscratch->kernel_stack
     */
    csrrw sp, sscratch, sp // sp <-> sscratch

    /**
     * return from S mode to U mode
     */
    sret
    