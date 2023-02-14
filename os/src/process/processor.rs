use super::manager::fetch_proc;
use super::pcb::{ProcessContext, ProcessControlBlock, ProcessStatus};
use super::switch::__switch;
use crate::mm::address::{PhysAddr, VirtAddr};
use crate::safe_refcell::UPSafeRefCell;
use crate::trap::TrapContext;

use alloc::sync::Arc;
use lazy_static::lazy_static;

pub struct Processor {
    curr: Option<Arc<ProcessControlBlock>>,
    idle_proc_cx: ProcessContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            curr: None,
            idle_proc_cx: ProcessContext::empty(),
        }
    }
}

impl Processor {
    pub fn take_curr(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.curr.take()
    }

    pub fn get_curr(&self) -> Option<Arc<ProcessControlBlock>> {
        self.curr.as_ref().map(|proc| Arc::clone(proc))
    }

    fn get_idle_proc_cx_ptr(&mut self) -> *mut ProcessContext {
        &mut self.idle_proc_cx as *mut _
    }
}

pub fn run_procs() {
    loop {
        let mut processor = PROCESSOR.borrow_mut();
        println!("looping");
        if let Some(proc) = fetch_proc() {
            let idle_proc_cx_ptr = processor.get_idle_proc_cx_ptr();
            // access coming proc TCB exclusively
            let mut proc_inner = proc.inner_borrow_mut();
            let next_proc_cx_ptr = &proc_inner.context as *const ProcessContext;
            proc_inner.status = ProcessStatus::RUNNING;
            // stop exclusively accessing coming proc TCB manually
            drop(proc_inner);
            processor.curr = Some(proc);
            // stop exclusively accessing processor manually
            drop(processor);
            unsafe {
                __switch(idle_proc_cx_ptr, next_proc_cx_ptr);
            }
            // When somewhere else calls
            // `_switch(curr_proc_cx, idle_proc_cx)`,
            // the control flow will return here.
            // By doing so, the schedule info will
            // be stored on boot stack.
        }
    }
}

pub fn schedule(switched_proc_cx_ptr: *mut ProcessContext) {
    let mut processor = PROCESSOR.borrow_mut();
    let idle_proc_cx_ptr = processor.get_idle_proc_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_proc_cx_ptr, idle_proc_cx_ptr);
    }
}

pub fn take_curr_proc() -> Option<Arc<ProcessControlBlock>> {
    PROCESSOR.borrow_mut().take_curr()
}

pub fn get_curr_proc() -> Option<Arc<ProcessControlBlock>> {
    PROCESSOR.borrow().get_curr()
}

pub fn curr_user_token() -> usize {
    let proc = get_curr_proc().unwrap();
    let token = proc.inner_borrow().get_user_token();
    token
}

pub fn curr_trap_cx() -> &'static mut TrapContext {
    get_curr_proc().unwrap().inner_borrow().get_trap_cx()
}

pub fn vaddr_to_paddr(vaddr: VirtAddr) -> PhysAddr {
    let curr_proc = get_curr_proc().unwrap();
    let inner = curr_proc.inner_borrow();
    inner.user_space.vaddr_to_paddr(vaddr).unwrap()
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeRefCell<Processor> = UPSafeRefCell::new(Processor::new());
}
