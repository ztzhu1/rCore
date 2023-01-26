use crate::sbi::console_putchar;
use crate::task::{suspend_curr_and_run_next, exit_curr_and_run_next};

const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;

pub fn syscall(id: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret = 0;
    match id {
        SYS_WRITE => {
            ret = sys_write(arg0, arg1, arg2);
        }
        SYS_EXIT => {
            sys_exit(arg0);
        }
        SYS_YIELD => {
            ret = sys_yield();
        }
        _ => panic!("unhandled syscall: {}.", id),
    }
    ret
}

fn sys_write(fd: usize, buf: usize, len: usize) -> usize {
    let mut count = 0_usize;
    let begin = buf as *const u8;
    unsafe {
        loop {
            let ch = begin.add(count).read_volatile();
            if ch == 0 {
                break;
            }
            console_putchar(ch as usize);
            count += 1;
            if count >= len {
                break;
            }
        }
    }
    count
}

fn sys_yield() -> usize {
    suspend_curr_and_run_next();
    0
}

fn sys_exit(exit_code: usize) -> ! {
    println!("[kernel] Application exited with code {}", exit_code as isize);
    exit_curr_and_run_next();
    panic!("Unreachable in sys_exit!");
}