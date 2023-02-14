use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE};
use crate::mm::address_space::MapPermission;
use crate::mm::{address::VirtAddr, KERNEL_SPACE};

use super::pid::PidHandle;

pub struct KernelStack {
    pid: usize,
    bottom: usize,
    top: usize,
}

impl KernelStack {
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (bottom, top) = Self::position(pid);
        KERNEL_SPACE.borrow_mut().insert_framed_area(
            bottom.into(),
            top.into(),
            MapPermission::R | MapPermission::W,
        );
        Self { pid, bottom, top }
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

    pub fn position(pid: usize) -> (usize, usize) {
        let top = TRAMPOLINE - pid * (KERNEL_STACK_SIZE + PAGE_SIZE);
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
    }
}
