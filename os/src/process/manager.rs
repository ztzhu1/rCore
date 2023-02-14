use super::pcb::ProcessControlBlock;
use crate::safe_refcell::UPSafeRefCell;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub struct ProcessManager {
    ready_queue: VecDeque<Arc<ProcessControlBlock>>,
}

/// A simple FIFO scheduler.
impl ProcessManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    pub fn add(&mut self, task: Arc<ProcessControlBlock>) {
        self.ready_queue.push_back(task);
    }

    pub fn fetch(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    pub static ref PROCESS_MANAGER: UPSafeRefCell<ProcessManager> =
        unsafe { UPSafeRefCell::new(ProcessManager::new()) };
}

pub fn add_proc(proc: Arc<ProcessControlBlock>) {
    PROCESS_MANAGER.borrow_mut().add(proc);
}

pub fn fetch_proc() -> Option<Arc<ProcessControlBlock>> {
    PROCESS_MANAGER.borrow_mut().fetch()
}
