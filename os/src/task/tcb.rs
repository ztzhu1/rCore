use crate::mm::address::{ppn_t, PhysAddr};
use crate::mm::memory_set::MemorySet;
use crate::trap::{trap_return, TrapContext};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TaskStatus {
    UNINIT,
    READY,
    RUNNING,
    EXITED,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    reg_s: [usize; 12],
}

impl TaskContext {
    pub fn empty() -> Self {
        Self {
            ra: 0,
            sp: 0,
            reg_s: [0; 12],
        }
    }

    pub fn from_goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            reg_s: [0; 12],
        }
    }
}
pub struct TaskControlBlock {
    pub status: TaskStatus,
    pub context: TaskContext,
    pub user_space: MemorySet,
    pub trap_cx_ppn: ppn_t,
    pub base_size: usize,
}

impl TaskControlBlock {
    pub fn new(
        task_context: TaskContext,
        user_space: MemorySet,
        trap_cx_ppn: ppn_t,
        user_sp: usize,
    ) -> Self {
        Self {
            status: TaskStatus::READY,
            context: task_context,
            user_space,
            trap_cx_ppn,
            base_size: user_sp,
        }
    }

    pub fn empty() -> Self {
        Self {
            status: TaskStatus::UNINIT,
            context: TaskContext::empty(),
            user_space: MemorySet::empty(),
            trap_cx_ppn: 0,
            base_size: 0,
        }
    }

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        unsafe {
            ((PhysAddr::from_ppn(self.trap_cx_ppn).0) as *mut TrapContext)
                .as_mut()
                .unwrap()
        }
    }

    pub fn get_user_token(&self) -> usize {
        self.user_space.token()
    }
}
