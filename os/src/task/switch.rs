use super::tcb::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

extern "C" {
    // Wrap __switch as a rust function so that
    // rust can help us save some `caller saved regs`.
    pub fn __switch(curr_task_cx: *mut TaskContext, target_task_cx: *const TaskContext);
}
