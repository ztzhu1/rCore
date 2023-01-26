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

    pub fn from_goto_restore(kstack_ptr: usize) -> Self {
        extern "C" {
            fn __restore();
        }
        Self {
            ra: __restore as usize,
            sp: kstack_ptr,
            reg_s: [0; 12],
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct TaskControlBlock {
    pub status: TaskStatus,
    pub context: TaskContext,
}

impl TaskControlBlock {
    pub fn new(task_context: TaskContext) -> Self {
        Self {
            status: TaskStatus::READY,
            context: task_context,
        }
    }

    pub fn empty() -> Self {
        Self {
            status: TaskStatus::UNINIT,
            context: TaskContext::empty(),
        }
    }
}
