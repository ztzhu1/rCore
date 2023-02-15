use crate::config::TRAMPOLINE;
use core::arch::global_asm;
use riscv::register::mtvec::TrapMode;
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    let scause = riscv::register::scause::read();
    let stval = riscv::register::stval::read();
    let sepc = riscv::register::sepc::read();
    match scause.cause() {
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            error!(
                "[kernel] {:?} in kernel, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
                scause.cause(),
                stval,
                sepc
            );
        }
        _ => {
            error!("[kernel] unhandled kernel trap: {:?}", scause.cause());
        }
    }
    panic!("a trap from kernel!");
}

mod context;
mod handler;
pub use context::TrapContext;
pub use handler::{trap_handler, trap_return};
