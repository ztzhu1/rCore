use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::fs::inode::{open_file, OpenFlags};
use crate::fs::pipe::make_pipe;
use crate::mm::address::VirtAddr;
use crate::mm::page_table::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, PageTable,
    UserBuffer,
};
use crate::process::manager::{add_proc, pid2proc};
use crate::process::processor::{curr_user_token, get_curr_proc, vaddr_to_paddr};
use crate::process::signal::{SignalAction, SignalFlags, MAX_SIG};
use crate::process::{exit_curr_and_run_next, suspend_curr_and_run_next};
use crate::sbi::console_getchar;
use crate::sbi::console_putchar;
use crate::timer::get_time_ms;

const FD_STDIN: usize = 0;

const SYS_DUP: usize = 27;
const SYS_OPEN: usize = 56;
const SYS_CLOSE: usize = 57;
const SYS_PIPE: usize = 59;
const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_KILL: usize = 129;
const SYS_SIGACTION: usize = 134;
const SYS_SIGPROCMASK: usize = 135;
const SYS_SIGRETURN: usize = 139;
const SYS_GET_TIME: usize = 169;
const SYS_GETPID: usize = 172;
const SYS_FORK: usize = 220;
const SYS_EXEC: usize = 221;
const SYS_WAITPID: usize = 260;

pub fn syscall(id: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret = 0;
    match id {
        SYS_DUP => {
            ret = sys_dup(arg0) as usize;
        }
        SYS_OPEN => {
            ret = sys_open(arg0 as *const u8, arg1 as u32) as usize;
        }
        SYS_CLOSE => {
            ret = sys_close(arg0) as usize;
        }
        SYS_PIPE => {
            ret = sys_pipe(arg0 as *mut usize) as usize;
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
        SYS_KILL => {
            ret = sys_kill(arg0, arg1 as i32) as usize;
        }
        SYS_SIGACTION => {
            ret = sys_sigaction(
                arg0 as i32,
                arg1 as *const SignalAction,
                arg2 as *mut SignalAction,
            ) as usize;
        }
        SYS_SIGPROCMASK => {
            ret = sys_sigprocmask(arg0 as u32) as usize;
        }
        SYS_SIGRETURN => {
            ret = sys_sigreturn() as usize;
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
            ret = sys_exec(arg0 as *const u8, arg1 as *const usize) as usize;
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

pub fn sys_dup(fd: usize) -> isize {
    let proc = get_curr_proc().unwrap();
    let mut inner = proc.inner_borrow_mut();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(Arc::clone(inner.fd_table[fd].as_ref().unwrap()));
    new_fd as isize
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

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
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

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
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

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let proc = get_curr_proc().unwrap();
    let token = curr_user_token();
    let mut inner = proc.inner_borrow_mut();
    let (pipe_read, pipe_write) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    *translated_refmut(token, pipe) = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    0
}

pub fn sys_yield() -> usize {
    suspend_curr_and_run_next();
    0
}

pub fn sys_kill(pid: usize, signum: i32) -> isize {
    if let Some(proc) = pid2proc(pid) {
        if let Some(flag) = SignalFlags::from_bits(1 << signum) {
            // insert the signal if legal
            let mut proc = proc.inner_borrow_mut();
            if proc.signals.contains(flag) {
                return -1;
            }
            proc.signals.insert(flag);
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_sigaction(
    signum: i32,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    let token = curr_user_token();
    let proc = get_curr_proc().unwrap();
    let mut inner = proc.inner_borrow_mut();
    if signum as usize > MAX_SIG {
        return -1;
    }
    if let Some(flag) = SignalFlags::from_bits(1 << signum) {
        if check_sigaction_error(flag, action as usize, old_action as usize) {
            return -1;
        }
        let prev_action = inner.signal_actions.table[signum as usize];
        *translated_refmut(token, old_action) = prev_action;
        inner.signal_actions.table[signum as usize] = *translated_ref(token, action);
        0
    } else {
        -1
    }
}

pub fn sys_sigprocmask(mask: u32) -> isize {
    if let Some(proc) = get_curr_proc() {
        let mut inner = proc.inner_borrow_mut();
        let old_mask = inner.signal_mask;
        if let Some(flag) = SignalFlags::from_bits(mask) {
            inner.signal_mask = flag;
            old_mask.bits() as isize
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_sigreturn() -> isize {
    if let Some(proc) = get_curr_proc() {
        let mut inner = proc.inner_borrow_mut();
        inner.handling_sig = -1;
        // restore the trap context
        let trap_cx = inner.get_trap_cx();
        *trap_cx = inner.trap_cx_backup.unwrap();
        trap_cx.gp[10] as isize
    } else {
        -1
    }
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

pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = curr_user_token();
    // Now we are in kernel space, but the path is stored
    // in user space, so we need to translate the address.
    let path = translated_str(token, path);

    let mut args_vec: Vec<String> = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(translated_str(token, arg_str_ptr as *const u8));
        unsafe {
            args = args.add(1);
        }
    }

    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let proc = get_curr_proc().unwrap();
        let argc = args_vec.len();
        proc.exec(all_data.as_slice(), args_vec);
        argc as isize
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

fn check_sigaction_error(signal: SignalFlags, action: usize, old_action: usize) -> bool {
    if action == 0
        || old_action == 0
        || signal == SignalFlags::SIGKILL
        || signal == SignalFlags::SIGSTOP
    {
        true
    } else {
        false
    }
}
