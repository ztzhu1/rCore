use alloc::sync::Arc;

use crate::fs::inode::{open_file, OpenFlags};
use crate::mm::address::VirtAddr;
use crate::mm::page_table::{
    translated_byte_buffer, translated_refmut, translated_str, PageTable, UserBuffer,
};
use crate::process::manager::add_proc;
use crate::process::processor::{curr_user_token, get_curr_proc, vaddr_to_paddr};
use crate::process::{exit_curr_and_run_next, suspend_curr_and_run_next};
use crate::sbi::console_getchar;
use crate::sbi::console_putchar;
use crate::timer::get_time_ms;

const FD_STDIN: usize = 0;

const SYS_OPEN: usize = 56;
const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_GET_TIME: usize = 169;
const SYS_GETPID: usize = 172;
const SYS_FORK: usize = 220;
const SYS_EXEC: usize = 221;
const SYS_WAITPID: usize = 260;

pub fn syscall(id: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret = 0;
    match id {
        SYS_OPEN => {
            ret = sys_open(arg0 as *const u8, arg1 as u32) as usize;
        }
        SYS_READ => {
            ret = sys_read(arg0, arg1 as *mut u8, arg2) as usize;
        }
        SYS_WRITE => {
            ret = sys_write(arg0, arg1 as *const u8, arg2) as usize;
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
        SYS_GETPID => {
            ret = sys_get_pid();
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

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let proc = get_curr_proc().unwrap();
    let token = curr_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = proc.inner_borrow_mut();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let proc = get_curr_proc().unwrap();
    let mut inner = proc.inner_borrow_mut();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = curr_user_token();
    let proc = get_curr_proc().unwrap();
    let inner = proc.inner_borrow();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current proc TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = curr_user_token();
    let proc = get_curr_proc().unwrap();
    let inner = proc.inner_borrow();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current proc TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
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

fn sys_get_pid() -> usize {
    get_curr_proc().unwrap().pid.0
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
    let new_trap_cx = new_proc.inner_borrow_mut().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    new_trap_cx.gp[10] = 0; //gp[10] is a0 reg
    add_proc(new_proc); // add new process to scheduler

    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = curr_user_token();
    // Now we are in kernel space, but the path is stored
    // in user space, so we need to translate the address.
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let proc = get_curr_proc().unwrap();
        proc.exec(all_data.as_slice());
        0
    } else {
        -1
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
