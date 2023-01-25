#![no_std]
#![no_main]
#![allow(unused)]
#![feature(panic_info_message)]

#[macro_use]
mod console;
mod batch;
mod lang_items;
mod safe_refcell;
mod sbi;
mod syscall;
mod trap;

use core::arch::{global_asm, asm};

global_asm!(include_str!("entry.S"));
global_asm!(include_str!("link_app.S"));

#[no_mangle]
fn rust_main() {
    clear_bss();
    print_addr_info();
    trap::init();
    batch::run_all_apps();
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
    println!("bss cleared.");
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
    println!("RustSBI-QEMU booted successfully!");
    println!(".text   [{:#x}, {:#x})", stext as usize, etext as usize);
    println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    println!(".data   [{:#x}, {:#x})", sdata as usize, edata as usize);
    println!(
        "boot_stack    [{:#x}, {:#x})",
        boot_stack_lower_bound as usize, boot_stack_upper_bound as usize
    );
    println!(
        "kernel_stack  [{:#x}, {:#x})",
        skstack as usize, ekstack as usize
    );
    println!(
        "user_stack    [{:#x}, {:#x})",
        sustack as usize, eustack as usize
    );
    println!(".bss    [{:#x}, {:#x})", sbss as usize, ebss as usize);
}
