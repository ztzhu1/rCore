#![no_std]
#![no_main]
#![allow(unused)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
mod console;
#[macro_use]
mod config;
mod ext;
mod lang_items;
mod loader;
mod mm;
mod process;
mod safe_refcell;
mod sbi;
mod syscall;
mod timer;
mod trap;
#[macro_use]
extern crate bitflags;
extern crate alloc;

use alloc::vec;
use core::arch::{asm, global_asm};
use ext::*;
use mm::address_space::remap_test;

use crate::config::MEMORY_END;

global_asm!(include_str!("entry.S"));
global_asm!(include_str!("link_app.S"));

#[no_mangle]
fn os_main() {
    clear_bss();
    print_addr_info();

    trap::init();
    mm::init(); // Sv39 paging

    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    process::processor::run_procs();
    sbi::exit_success();
}

fn clear_bss() {
    (sbss as usize..ebss as usize).for_each(|x| unsafe {
        (x as *mut u8).write_volatile(0);
    });
    kernel!("bss cleared.");
}

fn print_addr_info() {
    kernel!("RustSBI-QEMU booted successfully!");
    kernel!(".text   [{:#x}, {:#x})", stext as usize, etext as usize);
    kernel!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    kernel!(".data   [{:#x}, {:#x})", sdata as usize, edata as usize);
    kernel!(
        "boot_stack    [{:#x}, {:#x})",
        boot_stack_lower_bound as usize,
        boot_stack_upper_bound as usize
    );
    kernel!(".bss    [{:#x}, {:#x})", sbss as usize, ebss as usize);

    assert!(MEMORY_START!() < MEMORY_END, "overflow!");
    kernel!(
        "memory  [{:#x}, {:#x}) ({} KiB)",
        MEMORY_START!() as usize,
        MEMORY_END as usize,
        (MEMORY_END - MEMORY_START!()) / 1024
    );
}
