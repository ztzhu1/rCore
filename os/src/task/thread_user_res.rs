use alloc::sync::{Arc, Weak};

use super::pcb::ProcessControlBlock;
use crate::config::{PAGE_SIZE, TRAP_CONTEXT, USER_STACK_SIZE};
use crate::mm::address::{ppn_t, VirtAddr};
use crate::mm::address_space::MapPermission;

pub struct ThreadUserRes {
    pub tid: usize,
    pub ustack_base: usize,
    pub process: Weak<ProcessControlBlock>,
}

impl ThreadUserRes {
    pub fn new(
        process: Arc<ProcessControlBlock>,
        ustack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let tid = process.inner_borrow_mut().alloc_tid();
        let thread_user_res = Self {
            tid,
            ustack_base,
            process: Arc::downgrade(&process),
        };
        if alloc_user_res {
            thread_user_res.alloc_user_res();
        }
        thread_user_res
    }

    pub fn alloc_user_res(&self) {
        let proc = self.process.upgrade().unwrap();
        let mut proc_inner = proc.inner_borrow_mut();
        // alloc user stack
        let ustack_bottom = ustack_bottom_from_tid(self.ustack_base, self.tid);
        let ustack_top = ustack_bottom + USER_STACK_SIZE;
        proc_inner.user_space.insert_framed_area(
            ustack_bottom.into(),
            ustack_top.into(),
            MapPermission::R | MapPermission::W | MapPermission::U,
        );
        // alloc trap context
        let trap_cx_bottom = trap_cx_bottom_from_tid(self.tid);
        let trap_cx_top = trap_cx_bottom + PAGE_SIZE;
        proc_inner.user_space.insert_framed_area(
            trap_cx_bottom.into(),
            trap_cx_top.into(),
            MapPermission::R | MapPermission::W,
        );
    }

    fn dealloc_user_res(&self) {
        // dealloc tid
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_borrow_mut();
        // dealloc ustack manually
        let ustack_bottom_va: VirtAddr = ustack_bottom_from_tid(self.ustack_base, self.tid).into();
        process_inner
            .user_space
            .remove_area_with_start_vpn(ustack_bottom_va.vpn());
        // dealloc trap_cx manually
        let trap_cx_bottom_va: VirtAddr = trap_cx_bottom_from_tid(self.tid).into();
        process_inner
            .user_space
            .remove_area_with_start_vpn(trap_cx_bottom_va.vpn());
    }

    pub fn dealloc_tid(&self) {
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_borrow_mut();
        process_inner.dealloc_tid(self.tid);
    }

    pub fn trap_cx_ppn(&self) -> ppn_t {
        let process = self.process.upgrade().unwrap();
        let process_inner = process.inner_borrow_mut();
        let trap_cx_bottom_va: VirtAddr = trap_cx_bottom_from_tid(self.tid).into();
        process_inner
            .user_space
            .translate(trap_cx_bottom_va.vpn())
            .ppn()
    }

    pub fn ustack_base(&self) -> usize {
        self.ustack_base
    }

    pub fn ustack_top(&self) -> usize {
        ustack_bottom_from_tid(self.ustack_base, self.tid) + USER_STACK_SIZE
    }
}

impl Drop for ThreadUserRes {
    fn drop(&mut self) {
        self.dealloc_tid();
        self.dealloc_user_res();
    }
}

pub fn trap_cx_bottom_from_tid(tid: usize) -> usize {
    TRAP_CONTEXT - tid * PAGE_SIZE
}

fn ustack_bottom_from_tid(ustack_base: usize, tid: usize) -> usize {
    ustack_base + tid * (USER_STACK_SIZE + PAGE_SIZE)
}
