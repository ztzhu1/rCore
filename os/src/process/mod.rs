pub mod kernel_stack;
pub mod manager;
pub mod pcb;
pub mod pid;
pub mod processor;
pub mod switch;

use alloc::sync::Arc;
use lazy_static::lazy_static;

use self::manager::add_proc;
use self::pcb::{ProcessContext, ProcessControlBlock, ProcessStatus};
use self::processor::{schedule, take_curr_proc};
use crate::loader::app_data_by_name;

lazy_static! {
    pub static ref INITPROC: Arc<ProcessControlBlock> = Arc::new(pcb::ProcessControlBlock::new(
        app_data_by_name("initproc").unwrap()
    ));
}

pub fn add_initproc() {
    add_proc(INITPROC.clone());
}

pub fn suspend_curr_and_run_next() {
    // There must be an application running.
    let proc = take_curr_proc().unwrap();

    // ---- access current TCB exclusively
    let mut proc_inner = proc.inner_borrow_mut();
    let proc_cx_ptr = &mut proc_inner.context as *mut ProcessContext;
    // Change status to Ready
    proc_inner.status = ProcessStatus::READY;
    drop(proc_inner);
    // ---- stop exclusively accessing current PCB

    // push back to ready queue.
    add_proc(proc);
    // jump to scheduling cycle
    schedule(proc_cx_ptr);
}

/// Now assuming INITPROC doesn't exit.
pub fn exit_curr_and_run_next(exit_code: i32) {
    // take from Processor
    let proc = take_curr_proc().unwrap();
    // **** access current TCB exclusively
    let mut inner = proc.inner_borrow_mut();
    // Change status to Zombie
    inner.status = ProcessStatus::ZOMBIE;
    // Record exit code
    inner.exit_code = exit_code;
    println!("{}", proc.pid.0);
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_borrow_mut();
        for child in inner.children.iter() {
            child.inner_borrow_mut().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ stop exclusively accessing parent PCB
    inner.children.clear();
    // deallocate user space
    inner.user_space.recycle_data_frames();
    drop(inner);
    // **** stop exclusively accessing current PCB
    // drop proc manually to maintain rc correctly
    drop(proc);
    // we do not have to save proc context
    let mut _unused = ProcessContext::empty();
    schedule(&mut _unused as *mut _);
}
