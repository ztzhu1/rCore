use crate::config::*;
use crate::ext::_num_app;
use crate::mm::address::{PhysAddr, VirtAddr};
use crate::mm::memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
use crate::safe_refcell::UPSafeRefCell;
use crate::task::pid::KernelStack;
use crate::task::{TaskContext, TaskControlBlock, TaskStatus};
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub fn examine_app_id_valid(app_id: usize) {
    let n = app_num();
    if app_id >= n {
        panic!("app id({}) > max app id({})!", app_id, n - 1);
    }
}

pub fn kstack_position(pid: usize) -> (usize, usize) {
    let top = TRAMPOLINE - pid * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

pub fn create_app_kernel_stack(pid: usize) -> KernelStack {
    let (bottom, top) = kstack_position(pid);
    KERNEL_SPACE.borrow_mut().insert_framed_area(
        bottom.into(),
        top.into(),
        MapPermission::R | MapPermission::W,
    );
    KernelStack { pid, bottom, top }
}

pub fn init_tcb(app_id: usize) -> TaskControlBlock {
    if app_id >= app_num() {
        return TaskControlBlock::empty();
    }
    // create app kernel stack
    let (_, kernel_sp) = create_app_kernel_stack(app_id);
    // create user space
    let (user_space, user_sp, entry) = MemorySet::from_elf(app_data(app_id));
    // init trap context
    let trap_context = TrapContext::new(entry, user_sp, kernel_sp);

    let trap_cx_addr = user_space.translate(VirtAddr::from(TRAP_CONTEXT).vpn());
    unsafe {
        (trap_cx_addr.0 as *mut TrapContext).write_volatile(trap_context);
    }
    let task_cx = TaskContext::from_goto_trap_return(kernel_sp);
    TaskControlBlock::new(task_cx, user_space, trap_cx_addr.ppn(), user_sp)
}

pub fn app_num() -> usize {
    unsafe { (_num_app as *const usize).read_volatile() }
}

pub fn app_data(app_id: usize) -> &'static [u8] {
    examine_app_id_valid(app_id);
    unsafe {
        let start = (_num_app as *const usize).add(app_id + 1).read_volatile();
        let end = (_num_app as *const usize).add(app_id + 2).read_volatile();
        core::slice::from_raw_parts(start as *const u8, end - start)
    }
}

pub fn app_data_by_name(app_name: &str) -> Option<&'static [u8]> {
    let app_id = APP_NAMES.iter().position(|&name| name == app_name)?;
    Some(app_data(app_id))
}

pub fn app_name(app_id: usize) -> &'static str {
    examine_app_id_valid(app_id);
    APP_NAMES[app_id]
}

lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let mut names: Vec<&str> = Vec::new();
        extern "C" {
            fn _app_names();
        }
        let n = app_num();
        let mut start = _app_names as *const u8;
        let mut end = start;
        unsafe {
            for i in 0..n {
                while end.read_volatile() != 0 {
                    end = end.add(1);
                }
                let name_slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let name = core::str::from_utf8(name_slice).unwrap();
                names.push(name);
                start = end.add(1);
            }
        }
        names
    };
}
