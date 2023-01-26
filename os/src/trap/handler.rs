use crate::{syscall::syscall};
use super::context::TrapContext;
use riscv::register::{
    scause::{self, Exception, Trap},
    stval, stvec,
};
use crate::task::exit_curr_and_run_next;

#[no_mangle]
pub fn trap_handler(context: &mut TrapContext) -> &mut TrapContext {
    let scause = riscv::register::scause::read();
    let stval = riscv::register::stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            context.sepc += 4;
            context.gp[10] = syscall(
                context.gp[17],
                context.gp[10],
                context.gp[11],
                context.gp[12],
            );
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.");
            exit_curr_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_curr_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    context
}
