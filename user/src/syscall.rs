use core::arch::asm;

const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_GET_TIME: usize = 169;
const SYS_FORK: usize = 220;
const SYS_EXEC: usize = 221;
const SYS_WAITPID: usize = 260;

fn syscall(id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let mut ret: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") id,
        );
    }
    ret as isize
}

pub fn sys_read(fd: usize, buf: &mut [u8]) -> isize {
    syscall(SYS_READ, fd, buf.as_mut_ptr() as usize, buf.len()) as isize
}

pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    syscall(SYS_WRITE, fd, buf.as_ptr() as usize, buf.len())
}

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYS_EXIT, exit_code as usize, 0, 0);
    panic!("should exit!");
}

pub fn sys_yield() {
    syscall(SYS_YIELD, 0, 0, 0);
}

#[allow(unused)]
pub fn sys_get_time() {
    syscall(SYS_GET_TIME, 0, 0, 0);
}

pub fn sys_fork() -> isize {
    syscall(SYS_FORK, 0, 0, 0)
}

pub fn sys_exec(path: &str) -> isize {
    syscall(SYS_EXEC, path.as_ptr() as usize, 0, 0)
}

pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYS_WAITPID, pid as usize, exit_code as usize, 0)
}
