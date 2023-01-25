use super::stack::{KERNEL_STACK, USER_STACK};
use crate::safe_refcell::SafeRefCell;
use crate::trap::TrapContext;
use crate::sbi::exit_success;
use core::{arch::asm, cell::Ref};
use lazy_static::lazy_static;

const MAX_NUM_APP: usize = 10;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

struct AppManager {
    num_app: usize,
    next_app: usize,
    app_starts: [usize; MAX_NUM_APP + 1],
}

impl AppManager {
    pub fn new() -> Self {
        extern "C" {
            fn _num_app();
        }
        unsafe {
            let num_app = (_num_app as *const usize).read_volatile();
            let mut app_starts = [0_usize; MAX_NUM_APP + 1];
            let app_start_ptr = _num_app as *const usize;
            let app_start_slice =
                core::slice::from_raw_parts(app_start_ptr.add(1), MAX_NUM_APP + 1);
            app_starts.copy_from_slice(app_start_slice);
            Self {
                num_app,
                next_app: 0,
                app_starts,
            }
        }
    }

    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            println!(
                "[kernel] app_{} [{:#x}, {:#x})",
                i,
                self.app_starts[i],
                self.app_starts[i + 1]
            );
        }
    }

    pub fn get_num_app(&self) -> usize {
        self.num_app
    }

    pub fn get_next_app(&self) -> usize {
        self.next_app
    }

    pub fn done(&self) -> bool {
        self.next_app >= self.num_app
    }

    pub fn load_next(&mut self) -> bool {
        if self.next_app == self.num_app {
            println!("All apps have been executed!");
            return false;
        }
        unsafe {
            self.load_app(self.next_app);
        }
        self.next_app += 1;
        true
    }

    pub fn exit_curr_app(&self) {
        println!("app {} done.", self.next_app - 1);
    }

    unsafe fn load_app(&self, id: usize) {
        // TODO: handle error
        if id >= self.num_app {
            panic!("id({}) >= num_app({})", id, self.num_app);
        }
        let begin = self.app_starts[id];
        let end = self.app_starts[id + 1];
        let size = end - begin;
        if size > APP_SIZE_LIMIT {
            panic!("app too large");
        }
        // clear app area
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
        // copy
        let app_src = core::slice::from_raw_parts(begin as *const u8, size);
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, size);
        app_dst.copy_from_slice(app_src);
        // memory fence about fetching the instruction memory
        asm!("fence.i");
    }
}

/// Only should be used in single thread context.
lazy_static! {
    static ref APP_MANAGER: SafeRefCell<AppManager> = SafeRefCell::new(AppManager::new());
}

pub fn print_app_info() {
    APP_MANAGER.borrow().print_app_info();
}

pub fn next_app() -> usize {
    APP_MANAGER.borrow().get_next_app()
}

pub fn num_app() -> usize {
    APP_MANAGER.borrow().get_num_app()
}

pub fn all_apps_done() -> bool {
    APP_MANAGER.borrow().done()
}

pub fn run_next_app() -> ! {
    let mut am = APP_MANAGER.borrow_mut();
    if !am.load_next() {
        println!("No more apps!");
        exit_success();
    }
    drop(am);

    extern "C" {
        fn __restore(context_ptr: usize);
    }
    let context = TrapContext::new(APP_BASE_ADDRESS, USER_STACK.get_sp());
    unsafe {
        __restore(KERNEL_STACK.push_context(context) as *const _ as usize);
    }
    panic!("unreachable code!");
}
