#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_};

#[no_mangle]
fn main() -> i32 {
    println!("[user][initproc] running");
    if fork() == 0 {
        println!("[user][user_shell] executing user_shell");
        exec("user_shell\0");
    } else {
        println!("[user][initproc] waiting for child process exiting");
        loop {
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                println!("[user][initproc] can't find a child process, yields");
                yield_();
                continue;
            }
            println!(
                "[user][initproc] Released a zombie process, pid={}, exit_code={}",
                pid, exit_code,
            );
        }
    }
    0
}
