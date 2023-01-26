#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::yield_;

#[no_mangle]
fn main() -> i32 {
    println!("[user05] before yield.");
    yield_();
    println!("[user05] after yield.");
    0
}
