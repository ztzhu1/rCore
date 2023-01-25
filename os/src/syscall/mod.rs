use crate::batch::{all_apps_done, next_app, run_next_app};
use crate::sbi::console_putchar;

const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;

pub fn syscall(id: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret = 0;
    match id {
        SYS_WRITE => {
            ret = sys_write(arg0, arg1, arg2);
        }
        SYS_EXIT => {
            let na = next_app();
            println!("app{} exits.", na - 1);
            if all_apps_done() {
                println!("There are no more apps!");
            } else {
                run_next_app();
            }
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
