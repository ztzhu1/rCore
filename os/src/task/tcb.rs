use alloc::sync::{Arc, Weak};
use core::cell::{Ref, RefMut};

use super::context::TaskContext;
use super::kernel_stack::KernelStack;
use super::pcb::ProcessControlBlock;
use super::thread_user_res::{trap_cx_bottom_from_tid, ThreadUserRes};
use crate::mm::address::{ppn_t, PhysAddr};
use crate::safe_refcell::UPSafeRefCell;
use crate::trap::TrapContext;

pub struct TaskControlBlock {
    // immutable
    pub process: Weak<ProcessControlBlock>,
    pub kstack: KernelStack,
    // mutable
    inner: UPSafeRefCell<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    pub res: Option<ThreadUserRes>,
    pub trap_cx_ppn: ppn_t,
    pub task_cx: TaskContext,
    pub task_status: TaskStatus,
    pub exit_code: Option<i32>,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Blocked,
}

impl TaskControlBlock {
    pub fn new(
        process: Arc<ProcessControlBlock>,
        ustack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let res = ThreadUserRes::new(Arc::clone(&process), ustack_base, alloc_user_res);
        let kstack = KernelStack::alloc();
        let kstack_top = kstack.get_top();
        Self {
            process: Arc::downgrade(&process),
            kstack,
            inner: unsafe { UPSafeRefCell::new(TaskControlBlockInner::new(res, kstack_top)) },
        }
    }

    pub fn get_user_token(&self) -> usize {
        let proc = self.process.upgrade().unwrap();
        let inner = proc.inner_borrow_mut();
        inner.user_space.token()
    }

    pub fn inner_borrow(&self) -> Ref<TaskControlBlockInner> {
        self.inner.borrow()
    }

    pub fn inner_borrow_mut(&self) -> RefMut<TaskControlBlockInner> {
        self.inner.borrow_mut()
    }
}

impl TaskControlBlockInner {
    pub fn new(res: ThreadUserRes, kstack_top: usize) -> Self {
        let trap_cx_ppn = res.trap_cx_ppn();
        Self {
            res: Some(res),
            trap_cx_ppn: trap_cx_ppn,
            task_cx: TaskContext::from_goto_trap_return(kstack_top),
            task_status: TaskStatus::Ready,
            exit_code: None,
        }
    }

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        let trap_pa = PhysAddr::from_ppn(self.trap_cx_ppn).0;
        unsafe { (trap_pa as *mut TrapContext).as_mut().unwrap() }
    }

    pub fn get_trap_cx_va(&self) -> usize {
        trap_cx_bottom_from_tid(self.res.as_ref().unwrap().tid)
    }
}
