use lazy_static::lazy_static;

use super::id::RecycleAllocator;
use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE};
use crate::mm::address_space::MapPermission;
use crate::mm::{address::VirtAddr, KERNEL_SPACE};
use crate::safe_refcell::UPSafeRefCell;

lazy_static! {
    static ref KSTACK_ALLOCATOR: UPSafeRefCell<RecycleAllocator> =
        unsafe { UPSafeRefCell::new(RecycleAllocator::new()) };
}

pub fn kstack_alloc() -> KernelStack {
    KernelStack::alloc()
}

pub struct KernelStack {
    kid: usize,
    bottom: usize,
    top: usize,
}

impl KernelStack {
    pub fn alloc() -> Self {
        let kstack_id = KSTACK_ALLOCATOR.borrow_mut().alloc();
        let (kstack_bottom, kstack_top) = Self::position(kstack_id);
        KERNEL_SPACE.borrow_mut().insert_framed_area(
            kstack_bottom.into(),
            kstack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        Self {
            kid: kstack_id,
            bottom: kstack_bottom,
            top: kstack_top,
        }
    }

    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let ptr_mut = (self.top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }

    pub fn get_top(&self) -> usize {
        self.top
    }

    pub fn position(kstack_id: usize) -> (usize, usize) {
        let top = TRAMPOLINE - kstack_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
        let bottom = top - KERNEL_STACK_SIZE;
        (bottom, top)
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let start_vpn = VirtAddr::from(self.bottom).vpn();
        KERNEL_SPACE
            .borrow_mut()
            .remove_area_with_start_vpn(start_vpn);
        KSTACK_ALLOCATOR.borrow_mut().dealloc(self.kid);
    }
}
