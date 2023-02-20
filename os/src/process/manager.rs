use super::pcb::ProcessControlBlock;
use crate::safe_refcell::UPSafeRefCell;

use alloc::collections::{BTreeMap, VecDeque};
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
    pub static ref PID2PCB: UPSafeRefCell<BTreeMap<usize, Arc<ProcessControlBlock>>> =
        unsafe { UPSafeRefCell::new(BTreeMap::new()) };
}

pub fn add_proc(proc: Arc<ProcessControlBlock>) {
    PID2PCB.borrow_mut().insert(proc.pid.0, Arc::clone(&proc));
    PROCESS_MANAGER.borrow_mut().add(proc);
}

pub fn fetch_proc() -> Option<Arc<ProcessControlBlock>> {
    PROCESS_MANAGER.borrow_mut().fetch()
}

pub fn pid2proc(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    let map = PID2PCB.borrow();
    map.get(&pid).map(Arc::clone)
}

pub fn remove_from_pid2proc(pid: usize) {
    let mut map = PID2PCB.borrow_mut();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2proc!", pid);
    }
}
