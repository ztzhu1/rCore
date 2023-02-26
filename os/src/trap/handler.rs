use super::context::TrapContext;
use super::set_kernel_trap_entry;
use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::mm::address::VirtAddr;
use crate::mm::page_table::{translated_refmut, PageTable};
use crate::syscall::syscall;
use crate::task::processor::{curr_trap_cx, curr_trap_cx_va, curr_user_token, get_curr_proc};
use crate::task::signal::{SignalFlags, MAX_SIG};
use crate::task::{
    current_add_signal, exit_curr_and_run_next, suspend_curr_and_run_next, INITPROC,
};
use crate::timer::set_next_trigger;
use crate::trap::set_user_trap_entry;
use core::arch::asm;
use core::borrow::Borrow;
use riscv::register::sepc;
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    stval, stvec,
};

#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = riscv::register::scause::read();
    let stval = riscv::register::stval::read();
    let mut context = curr_trap_cx();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            context.sepc += 4;
            let result = syscall(
                context.gp[17],
                context.gp[10],
                context.gp[11],
                context.gp[12],
            );
            // cx is changed during sys_exec, so we have to call it again
            context = curr_trap_cx();
            context.gp[10] = result;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            error!(
                "{:?} in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
                scause.cause(),
                stval,
                curr_trap_cx().sepc,
            );
            // page fault exit code
            // exit_curr_and_run_next(-2);
            current_add_signal(SignalFlags::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            // error!("IllegalInstruction in application, core dumped.");
            // illegal instruction exit code
            // exit_curr_and_run_next(-3);
            current_add_signal(SignalFlags::SIGILL);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // info!("timer interrupt: yield.");
            set_next_trigger();
            suspend_curr_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    // handle_signals();
    // check error signals (if error then exit)
    if let Some((errno, msg)) = check_signals_error_of_current() {
        error!("{}", msg);
        exit_curr_and_run_next(errno);
    }

    trap_return();
}

#[no_mangle]
/// set the new addr of __restore asm function in TRAMPOLINE page,
/// set the reg a0 = trap_cx_ptr, reg a1 = phy addr of usr page table,
/// finally, jump to new addr of __restore asm function
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = curr_trap_cx_va();
    let user_satp = curr_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;

    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",             // jump to new addr of __restore asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = virt addr of Trap Context
            in("a1") user_satp,        // a1 = phy addr of usr page table
            options(noreturn)
        );
    }
}

fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let proc = get_curr_proc().unwrap();
    let proc_inner = proc.inner_borrow_mut();
    proc_inner.signals.check_error()
}
