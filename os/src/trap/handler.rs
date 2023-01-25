use crate::{syscall::syscall, batch::run_next_app};
use super::context::TrapContext;
use riscv::register::{
    scause::{self, Exception, Trap},
    stval, stvec,
};

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
            run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            run_next_app();
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
