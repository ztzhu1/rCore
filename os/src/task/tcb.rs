use super::context::TaskContext;

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    UNINIT,
    READY,
    RUNNING,
    EXITED,
}

#[derive(Clone, Copy)]
pub struct TaskControlBlock {
    status: TaskStatus,
    context: TaskContext,
}

impl TaskControlBlock {
    pub fn new() -> Self {
        Self {
            status: TaskStatus::UNINIT,
            context: TaskContext::new(),
        }
    }
}
