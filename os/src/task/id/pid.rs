use lazy_static::lazy_static;

use super::RecycleAllocator;
use crate::safe_refcell::UPSafeRefCell;

lazy_static! {
    static ref PID_ALLOCATOR: UPSafeRefCell<RecycleAllocator> =
        UPSafeRefCell::new(RecycleAllocator::new());
}

pub struct PidHandle(pub usize);

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.borrow_mut().dealloc(self.0);
    }
}

pub fn pid_alloc() -> PidHandle {
    PidHandle(PID_ALLOCATOR.borrow_mut().alloc())
}
