#![no_std]
#![allow(unreachable_code)]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
mod heap_allocator;
mod lang_items;
mod syscall;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss();
    init_heap();
    exit(main());
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    });
}

use syscall::*;

use crate::heap_allocator::init_heap;

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code);
}

pub fn yield_() {
    sys_yield();
}

pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            // -1 or a real pid
            exit_pid => return exit_pid,
        }
    }
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut i32) {
            -2 => {
                yield_();
            }
            // -1 or a real pid
            exit_pid => return exit_pid,
        }
    }
}

pub fn fork() -> isize {
    sys_fork()
}

pub fn exec(path: &str) -> isize {
    sys_exec(path)
}
