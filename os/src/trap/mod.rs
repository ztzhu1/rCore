use core::arch::global_asm;
use riscv::register::mtvec::TrapMode;
use riscv::register::stvec;

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

mod context;
mod handler;
pub use context::TrapContext;