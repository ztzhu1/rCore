use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE};
use crate::safe_refcell::UPSafeRefCell;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::loader::create_app_kernel_stack;

// --------- PidHandle -------
pub struct PidHandle(pub usize);

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.borrow_mut().dealloc(self.0);
    }
}

// --------- PidAllocator -------
struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    pub fn new() -> Self {
        Self {
            current: 0,
            recycled: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        assert!(self.recycled.iter().find(|&&x| x == pid).is_none());
        self.recycled.push(pid);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: UPSafeRefCell<PidAllocator> = UPSafeRefCell::new(PidAllocator::new());
}

pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.borrow_mut().alloc()
}

// --------- KernelStack -------
pub struct KernelStack {
    pid: usize,
    bottom: usize,
    top: usize,
}

impl KernelStack {
    pub fn new(pid_handle: &PidHandle) -> Self {
        create_app_kernel_stack(pid_handle.0)
    }

    pub fn get_top(&self) -> usize {
        self.top
    }
}

