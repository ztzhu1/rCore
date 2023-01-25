use super::context::TaskContext;
use super::tcb::TaskControlBlock;
use crate::safe_refcell::SafeRefCell;

const MAX_TASK_NUM: usize = 10;

pub struct TaskManager {
    task_num: usize,
    curr_task: usize,
    tcbs: [TaskControlBlock; MAX_TASK_NUM],
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            task_num: 0,
            curr_task: 0,
            tcbs: [TaskControlBlock::new(); MAX_TASK_NUM],
        }
    }
}

#[link_section = ".data"]
static TASK_MANAGER: SafeRefCell<TaskManager> = SafeRefCell::new(TaskManager::new());
