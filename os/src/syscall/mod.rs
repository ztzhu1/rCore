use alloc::sync::Arc;

use crate::loader::app_data_by_name;
use crate::mm::address::VirtAddr;
use crate::mm::page_table::{translated_byte_buffer, translated_refmut, translated_str, PageTable};
use crate::process::manager::add_proc;
use crate::process::processor::{curr_user_token, get_curr_proc, vaddr_to_paddr};
use crate::process::{exit_curr_and_run_next, suspend_curr_and_run_next};
use crate::sbi::console_getchar;
use crate::sbi::console_putchar;
use crate::timer::get_time_ms;

const FD_STDIN: usize = 0;

const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_GET_TIME: usize = 169;
const SYS_FORK: usize = 220;
const SYS_EXEC: usize = 221;
const SYS_WAITPID: usize = 260;

pub fn syscall(id: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret = 0;
    match id {
        SYS_WRITE => {
            ret = sys_write(arg0, arg1, arg2);
        }
        SYS_EXIT => {
            sys_exit(arg0 as i32);
        }
        SYS_YIELD => {
            ret = sys_yield();
        }
        SYS_GET_TIME => {
            ret = sys_get_time();
        }
        SYS_FORK => {
            ret = sys_fork() as usize;
        }
        SYS_EXEC => {
            ret = sys_exec(arg0 as *const u8) as usize;
        }
        SYS_WAITPID => {
            ret = sys_waitpid(arg0 as isize, arg1 as *mut i32) as usize;
        }
        _ => panic!("unhandled syscall: {}.", id),
    }
    ret
}

fn vbuf_to_pbuf(buf: usize) -> usize {
    let vaddr = VirtAddr::from(buf);
    vaddr_to_paddr(vaddr).0
}

fn sys_write(fd: usize, buf: usize, len: usize) -> usize {
    let mut count = 0_usize;
    let buf = vbuf_to_pbuf(buf);
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

fn sys_exit(exit_code: i32) -> ! {
    info!("Application exited with code {}", exit_code);
    exit_curr_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

fn sys_get_time() -> usize {
    get_time_ms()
}
#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

fn sys_fork() -> isize {
    let curr_proc = get_curr_proc().unwrap();
    let new_proc = curr_proc.fork();
    let new_pid = new_proc.pid.0;
    // modify trap context of new_proc, because it returns immediately after switching
    let trap_cx = new_proc.inner_borrow_mut().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.gp[10] = 0; //gp[10] is a0 reg
    add_proc(new_proc); // add new process to scheduler

    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = curr_user_token();
    // Now we are in kernel space, but the path is stored
    // in user space, so we need to translate the address.
    let path = translated_str(token, path);
    if let Some(data) = app_data_by_name(path.as_str()) {
        get_curr_proc().unwrap().exec(data);
        0
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    suspend_curr_and_run_next();
                    continue;
                } else {
                    break;
                }
            }
            let ch = c as u8;
            let mut buffers = translated_byte_buffer(curr_user_token(), buf, len);
            unsafe {
                buffers[0].as_mut_ptr().write_volatile(ch);
            }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let curr_proc = get_curr_proc().unwrap();
    // find a child process

    // ---- access current TCB exclusively
    let mut inner = curr_proc.inner_borrow_mut();
    if inner
        .children
        .iter()
        .find(|p| pid == -1 || pid as usize == p.get_pid())
        .is_none()
    {
        return -1;
        // ---- stop exclusively accessing current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_borrow_mut().is_zombie() && (pid == -1 || pid as usize == p.get_pid())
        // ++++ stop exclusively accessing child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after removing from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.get_pid();
        // ++++ temporarily access child TCB exclusively
        let exit_code = child.inner_borrow_mut().exit_code;
        // ++++ stop exclusively accessing child PCB
        *translated_refmut(inner.user_space.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- stop exclusively accessing current PCB automatically
}
