#![no_std]
#![no_main]
#![allow(unused)]

mod lang_items;

use core::arch::global_asm;

global_asm!(include_str!("entry.s"));

fn rust_main() {

}