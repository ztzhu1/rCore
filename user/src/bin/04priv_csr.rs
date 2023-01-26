#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use riscv::register::sstatus::{self, SPP};

#[no_mangle]
fn main() -> i32 {
    println!("[user04] Try to access privileged CSR in U Mode");
    println!("[user04] Kernel should kill this application!");
    unsafe {
        sstatus::set_spp(SPP::User);
    }
    0
}
