use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    pub gp: [usize; 32], // general purpose regs
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    pub fn new(sepc: usize, sp: usize) -> Self {
        let mut sstatus = riscv::register::sstatus::read();
        // previous privilege mode: user
        sstatus.set_spp(SPP::User);

        let mut context = Self {
            gp: [0; 32],
            sstatus,
            sepc
        };
        context.set_sp(sp);
        context
    }

    fn set_sp(&mut self, sp: usize) {
        self.gp[2] = sp;
    }
}