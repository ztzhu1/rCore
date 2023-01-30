#![no_std]
#![no_main]
#![allow(unused)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
mod console;
mod lang_items;
mod loader;
mod mm;
mod safe_refcell;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;
#[macro_use]
extern crate bitflags;
extern crate alloc;

use core::arch::{asm, global_asm};
use alloc::vec;
use mm::heap_allocator::{heap_test, init_heap};

global_asm!(include_str!("entry.S"));
global_asm!(include_str!("link_app.S"));

#[no_mangle]
fn os_main() {
    clear_bss();
    print_addr_info();
    trap::init();
    init_heap();
    // heap_test();
    // mm::frame_allocator::frame_allocator_test();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    task::run_first_task();
    sbi::exit_success();
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|x| unsafe {
        (x as *mut u8).write_volatile(0);
    });
    kernel!("bss cleared.");
}

fn print_addr_info() {
    extern "C" {
        fn stext(); // begin addr of text segment
        fn etext(); // end addr of text segment
        fn srodata(); // start addr of Read-Only data segment
        fn erodata(); // end addr of Read-Only data ssegment
        fn sdata(); // start addr of data segment
        fn edata(); // end addr of data segment
        fn boot_stack_lower_bound(); // stack lower bound
        fn boot_stack_upper_bound(); // stack top
        fn skstack(); // start addr of kernel stack
        fn ekstack(); // start addr of kernel stack
        fn sustack(); // start addr of user   stack
        fn eustack(); // start addr of user   stack
        fn sbss(); // start addr of BSS segment
        fn ebss(); // end addr of BSS segment
    }
    kernel!("RustSBI-QEMU booted successfully!");
    kernel!(
        ".text   [{:#x}, {:#x})",
        stext as usize, etext as usize
    );
    kernel!(
        ".rodata [{:#x}, {:#x})",
        srodata as usize, erodata as usize
    );
    kernel!(
        ".data   [{:#x}, {:#x})",
        sdata as usize, edata as usize
    );
    kernel!(
        "boot_stack    [{:#x}, {:#x})",
        boot_stack_lower_bound as usize, boot_stack_upper_bound as usize
    );
    kernel!(
        "kernel_stack  [{:#x}, {:#x})",
        skstack as usize, ekstack as usize
    );
    kernel!(
        "user_stack    [{:#x}, {:#x})",
        sustack as usize, eustack as usize
    );
    kernel!(
        ".bss    [{:#x}, {:#x})",
        sbss as usize, ebss as usize
    );
}
