.section .text.entry
.global _start
_start:
    la sp, boot_stack_upper_bound
    la t0, boot_stack_lower_bound
    jal rust_main

.section .bss.stack
.global boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
.global boot_stack_upper_bound
boot_stack_upper_bound: