use super::handler::trap_handler;
use crate::mm::{address::VirtAddr, KERNEL_SPACE};
use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapContext {
    pub gp: [usize; 32], // general purpose regs
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub kernel_satp: usize,
    pub kernel_sp: usize,
    pub trap_handler: usize,
}

impl TrapContext {
    pub fn new(entry: usize, sp: usize, kernel_sp: usize) -> Self {
        let mut sstatus = riscv::register::sstatus::read();
        // previous privilege mode: user
        sstatus.set_spp(SPP::User);

        let mut context = Self {
            gp: [0; 32],                                // 0-31
            sstatus,                                    // 32
            sepc: entry,                                // 33
            kernel_satp: KERNEL_SPACE.borrow().token(), // 34
            kernel_sp,                                  // 35
            trap_handler: trap_handler as usize,        // 36
        };
        context.set_sp(sp);
        context
    }

    fn set_sp(&mut self, sp: usize) {
        self.gp[2] = sp;
    }

    pub fn app_init_context(entry_point: usize, user_sp: usize, kernel_stack_top: usize) -> Self {
        let cx = Self::new(entry_point, user_sp, kernel_stack_top);
        cx
    }
}
