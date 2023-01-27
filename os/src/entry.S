.section .text.entry
.global _start
_start:
    la sp, boot_stack_upper_bound
    jal os_main

.section .bss.bstack
.global boot_stack_lower_bound
.global boot_stack_upper_bound
boot_stack_lower_bound:
    .space 4096 * 16
boot_stack_upper_bound: