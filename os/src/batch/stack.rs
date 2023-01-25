use crate::trap::TrapContext;
use super::app_manager::run_next_app;

const KERNEL_STACK_SIZE: usize = 4096 * 2;
const USER_STACK_SIZE: usize = 4096 * 2;

#[repr(align(4096))]
pub struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    pub fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + self.data.len()
    }

    pub fn push_context(&self, context: TrapContext) -> &'static TrapContext {
        let context_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *context_ptr = context;
            return context_ptr.as_mut().unwrap();
        }
    }
}

#[repr(align(4096))]
pub struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    pub fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + self.data.len()
    }
}

#[link_section = ".bss.kstack"]
pub static KERNEL_STACK: KernelStack = KernelStack {
    data: [0_u8; KERNEL_STACK_SIZE],
};

#[link_section = ".bss.ustack"]
pub static USER_STACK: UserStack = UserStack {
    data: [0_u8; USER_STACK_SIZE],
};

pub fn run_all_apps() {
    run_next_app();
}
