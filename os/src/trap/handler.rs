use super::context::TrapContext;
use super::set_kernel_trap_entry;
use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::mm::address::VirtAddr;
use crate::mm::page_table::{translated_refmut, PageTable};
use crate::process::processor::{curr_trap_cx, curr_user_token, get_curr_proc};
use crate::process::signal::{SignalFlags, MAX_SIG};
use crate::process::{
    current_add_signal, exit_curr_and_run_next, suspend_curr_and_run_next, INITPROC,
};
use crate::syscall::syscall;
use crate::timer::set_next_trigger;
use crate::trap::set_user_trap_entry;
use core::arch::asm;
use core::borrow::Borrow;
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
            // println!(
            //     "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
            //     scause.cause(),
            //     stval,
            //     curr_trap_cx().sepc,
            // );
            // page fault exit code
            // exit_curr_and_run_next(-2);
            current_add_signal(SignalFlags::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            // println!("[kernel] IllegalInstruction in application, core dumped.");
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
    handle_signals();
    // check error signals (if error then exit)
    if let Some((errno, msg)) = check_signals_error_of_current() {
        println!("[kernel] {}", msg);
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
    let trap_cx_ptr = TRAP_CONTEXT;
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

pub fn handle_signals() {
    loop {
        check_pending_signals();
        let (frozen, killed) = {
            let proc = get_curr_proc().unwrap();
            let proc_inner = proc.inner_borrow_mut();
            (proc_inner.frozen, proc_inner.killed)
        };
        if !frozen || killed {
            break;
        }
        suspend_curr_and_run_next();
    }
}

fn check_pending_signals() {
    for sig in 0..(MAX_SIG + 1) {
        let proc = get_curr_proc().unwrap();
        let proc_inner = proc.inner_borrow_mut();
        let signal = SignalFlags::from_bits(1 << sig).unwrap();
        if proc_inner.signals.contains(signal) && (!proc_inner.signal_mask.contains(signal)) {
            let mut masked = true;
            let handling_sig = proc_inner.handling_sig;
            if handling_sig == -1 {
                masked = false;
            } else {
                let handling_sig = handling_sig as usize;
                if !proc_inner.signal_actions.table[handling_sig]
                    .mask
                    .contains(signal)
                {
                    masked = false;
                }
            }
            if !masked {
                drop(proc_inner);
                drop(proc);
                if signal == SignalFlags::SIGKILL
                    || signal == SignalFlags::SIGSTOP
                    || signal == SignalFlags::SIGCONT
                    || signal == SignalFlags::SIGDEF
                {
                    // signal is a kernel signal
                    call_kernel_signal_handler(signal);
                } else {
                    // signal is a user signal
                    call_user_signal_handler(sig, signal);
                    return;
                }
            }
        }
    }
}

fn call_kernel_signal_handler(signal: SignalFlags) {
    let proc = get_curr_proc().unwrap();
    let mut proc_inner = proc.inner_borrow_mut();
    match signal {
        SignalFlags::SIGSTOP => {
            proc_inner.frozen = true;
            proc_inner.signals ^= SignalFlags::SIGSTOP;
        }
        SignalFlags::SIGCONT => {
            if proc_inner.signals.contains(SignalFlags::SIGCONT) {
                proc_inner.signals ^= SignalFlags::SIGCONT;
                proc_inner.frozen = false;
            }
        }
        _ => {
            // println!(
            //     "[K] call_kernel_signal_handler:: current task sigflag {:?}",
            //     task_inner.signals
            // );
            proc_inner.killed = true;
        }
    }
}

fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    let proc = get_curr_proc().unwrap();
    let mut proc_inner = proc.inner_borrow_mut();

    let handler = proc_inner.signal_actions.table[sig].handler;
    if handler != 0 {
        // user handler

        // handle flag
        proc_inner.handling_sig = sig as isize;
        proc_inner.signals ^= signal;

        // backup trapframe
        let mut trap_cx = proc_inner.get_trap_cx();
        proc_inner.trap_cx_backup = Some(*trap_cx);

        // modify trapframe
        trap_cx.sepc = handler;

        // put args (a0)
        trap_cx.gp[10] = sig;
    } else {
        // default action
        println!("[K] task/call_user_signal_handler: default action: ignore it or kill process");
    }
}

fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let proc = get_curr_proc().unwrap();
    let proc_inner = proc.inner_borrow_mut();
    proc_inner.signals.check_error()
}
