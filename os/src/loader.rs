use core::char::MAX;
use core::num;

use crate::safe_refcell::SafeRefCell;
use crate::trap::TrapContext;
use crate::task::{TaskControlBlock, TaskStatus, TaskContext};
use lazy_static::lazy_static;

const MAX_NUM_APP: usize = 6;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const USER_STACK_SIZE: usize = 4096;

#[repr(align(4096))]
#[derive(Clone, Copy)]
pub struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    pub fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + self.data.len()
    }

    pub fn push_trap_context(&self, trap_context: TrapContext) -> usize {
        let trap_context_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *trap_context_ptr = trap_context;
        }
        trap_context_ptr as usize
    }
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
pub struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    pub fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + self.data.len()
    }
}

pub struct Loader {
    num_app: usize,
    app_starts: [usize; MAX_NUM_APP + 1],
}

impl Loader {
    pub fn new() -> Self {
        extern "C" {
            fn _num_app();
        }
        unsafe {
            let mut num_app = (_num_app as *const usize).read_volatile();
            if num_app > MAX_NUM_APP {
                num_app = MAX_NUM_APP;
            }
            let mut app_starts = [0_usize; MAX_NUM_APP + 1];
            let app_start_ptr = _num_app as *const usize;
            let app_start_slice =
                core::slice::from_raw_parts(app_start_ptr.add(1), num_app + 1);
            app_starts[0..=num_app].copy_from_slice(app_start_slice);
            Self {
                num_app,
                app_starts,
            }
        }
    }

    pub fn get_num_app(&self) -> usize {
        self.num_app
    }

    pub unsafe fn load_app(&self, app_id: usize) {
        examine_app_id_valid(app_id);
        let src_start = self.app_starts[app_id];
        let src_end = self.app_starts[app_id + 1];
        let app_size = src_end - src_start;
        let dst_start = get_base(app_id);
        core::slice::from_raw_parts_mut(dst_start as *mut u8, APP_SIZE_LIMIT).fill(0);
        let dst_raw = core::slice::from_raw_parts_mut(dst_start as *mut u8, app_size);
        let src_raw = core::slice::from_raw_parts(src_start as *mut u8, app_size);
        dst_raw.copy_from_slice(src_raw);
    }
}

fn get_base(app_id: usize) -> usize {
    examine_app_id_valid(app_id);
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

pub fn examine_app_id_valid(app_id: usize) {
    let n = app_num();
    if app_id >= n {
        panic!("app id({}) > max app id({})!", app_id, n - 1);
    }
}

/// Only should be used in single thread context.
lazy_static! {
    static ref LOADER: SafeRefCell<Loader> = SafeRefCell::new(Loader::new());
}

#[link_section = ".bss.kstack"]
static KERNEL_STACK: [KernelStack; MAX_NUM_APP] = [KernelStack {
    data: [0_u8; KERNEL_STACK_SIZE],
}; MAX_NUM_APP];

#[link_section = ".bss.ustack"]
static USER_STACK: [UserStack; MAX_NUM_APP] = [UserStack {
    data: [0_u8; USER_STACK_SIZE],
}; MAX_NUM_APP];

fn init_trap_context(app_id: usize) {
    examine_app_id_valid(app_id);
    let trap_context = TrapContext::new(get_base(app_id), USER_STACK[app_id].get_sp());
    KERNEL_STACK[app_id].push_trap_context(trap_context);
}

fn init_tcb(app_id: usize) -> TaskControlBlock {
    if app_id >= app_num() {
        return TaskControlBlock::empty();
    }
    let task_context = TaskContext::from_goto_restore(KERNEL_STACK[app_id].get_sp() - core::mem::size_of::<TrapContext>());
    TaskControlBlock::new(task_context)
}

pub fn init_app(app_id: usize) -> TaskControlBlock {
    if app_id < app_num() {
        unsafe {
            LOADER.borrow().load_app(app_id);
        }
        init_trap_context(app_id);
    }
    init_tcb(app_id)
}

pub fn app_num() -> usize {
    LOADER.borrow().get_num_app()
}