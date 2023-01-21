#![no_std]
#![no_main]
#![allow(unused)]
#![feature(panic_info_message)]

#[macro_use]

mod console;
mod lang_items;
mod sbi;

use core::arch::global_asm;

global_asm!(include_str!("entry.S"));

#[no_mangle]
fn rust_main() {
    extern "C" {
        fn stext(); // begin addr of text segment
        fn etext(); // end addr of text segment
        fn srodata(); // start addr of Read-Only data segment
        fn erodata(); // end addr of Read-Only data ssegment
        fn sdata(); // start addr of data segment
        fn edata(); // end addr of data segment
        fn sbss(); // start addr of BSS segment
        fn ebss(); // end addr of BSS segment
        fn boot_stack_lower_bound(); // stack lower bound
        fn boot_stack_upper_bound(); // stack top
    }
    clear_bss();
    println!("RustSBI-QEMU booted successfully!");
    println!(".text   [{:#x}, {:#x})", stext as usize, etext as usize);
    println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    println!(".data   [{:#x}, {:#x})", sdata as usize, edata as usize);
    println!(
        "boot_stack  [{:#x}, {:#x})",
        boot_stack_lower_bound as usize, boot_stack_upper_bound as usize
    );
    println!(".bss   [{:#x}, {:#x})", sbss as usize, ebss as usize);

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
