use super::pcb::ProcessControlBlock;
use super::tcb::{TaskControlBlock, TaskStatus};
use crate::safe_refcell::UPSafeRefCell;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }

    pub fn remove(&mut self, task: Arc<TaskControlBlock>) {
        if let Some((id, _)) = self
            .ready_queue
            .iter()
            .enumerate()
            .find(|(_, t)| Arc::as_ptr(t) == Arc::as_ptr(&task))
        {
            self.ready_queue.remove(id);
        }
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: UPSafeRefCell<TaskManager> =
        unsafe { UPSafeRefCell::new(TaskManager::new()) };
    pub static ref PID2PCB: UPSafeRefCell<BTreeMap<usize, Arc<ProcessControlBlock>>> =
        unsafe { UPSafeRefCell::new(BTreeMap::new()) };
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.borrow_mut().add(task);
}

pub fn wakeup_task(task: Arc<TaskControlBlock>) {
    let mut task_inner = task.inner_borrow_mut();
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    add_task(task);
}

pub fn remove_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.borrow_mut().remove(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.borrow_mut().fetch()
}

pub fn pid2proc(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    let map = PID2PCB.borrow();
    map.get(&pid).map(Arc::clone)
}

pub fn insert_into_pid2proc(pid: usize, process: Arc<ProcessControlBlock>) {
    PID2PCB.borrow_mut().insert(pid, process);
}

pub fn remove_from_pid2proc(pid: usize) {
    let mut map = PID2PCB.borrow_mut();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2proc!", pid);
    }
}
