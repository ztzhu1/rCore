use super::context::TaskContext;
use super::pcb::{self, ProcessControlBlock, ProcessStatus};
use super::switch::__switch;
use super::tcb::TaskControlBlock;
use crate::mm::address::{PhysAddr, VirtAddr};
use crate::safe_refcell::UPSafeRefCell;
use crate::task::add_initproc;
use crate::task::manager::fetch_task;
use crate::task::tcb::TaskStatus;
use crate::trap::TrapContext;

use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PROCESSOR: UPSafeRefCell<Processor> = UPSafeRefCell::new(Processor::new());
}

pub struct Processor {
    curr: Option<Arc<TaskControlBlock>>,
    idle_task_cx: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            curr: None,
            idle_task_cx: TaskContext::empty(),
        }
    }

    pub fn take_curr(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.curr.take()
    }

    pub fn get_curr(&self) -> Option<Arc<TaskControlBlock>> {
        self.curr.as_ref().map(|proc| Arc::clone(proc))
    }

    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
}

pub fn run_tasks() {
    add_initproc();
    info!("processes running");

    loop {
        let mut processor = PROCESSOR.borrow_mut();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            let mut task_inner = task.inner_borrow_mut();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            // stop exclusively accessing coming task TCB manually
            drop(task_inner);
            processor.curr = Some(task);
            // stop exclusively accessing processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
            // When somewhere else calls
            // `__switch(curr_task_cx, idle_task_cx_ptr)`,
            // the control flow will return here.
            // By doing so, the schedule info will
            // be stored on boot stack.

            // info!("scheduled");
        }
    }
}

pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.borrow_mut();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}

pub fn take_curr_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.borrow_mut().take_curr()
}

pub fn get_curr_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.borrow().get_curr()
}

pub fn get_curr_proc() -> Option<Arc<ProcessControlBlock>> {
    get_curr_task().unwrap().process.upgrade()
}

pub fn curr_user_token() -> usize {
    get_curr_task().unwrap().get_user_token()
}

pub fn curr_trap_cx() -> &'static mut TrapContext {
    get_curr_task().unwrap().inner_borrow().get_trap_cx()
}

pub fn curr_trap_cx_va() -> usize {
    get_curr_task().unwrap().inner_borrow().get_trap_cx_va()
}

pub fn vaddr_to_paddr(vaddr: VirtAddr) -> PhysAddr {
    let curr_task = get_curr_task().unwrap();
    curr_task
        .process
        .upgrade()
        .unwrap()
        .inner_borrow()
        .user_space
        .vaddr_to_paddr(vaddr)
        .unwrap()
}
