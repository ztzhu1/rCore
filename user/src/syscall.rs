use core::arch::asm;

const SYS_WRITE: usize = 64;
const SYS_EXIT:  usize = 93;
const SYS_YIELD: usize = 124;

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

pub fn sys_write(fd: usize, buf: &[u8]) -> usize {
    syscall(SYS_WRITE, fd, buf.as_ptr() as usize, buf.len()) as usize
}

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYS_EXIT, exit_code as usize, 0, 0);
    panic!("should exit!");
}

pub fn sys_yield() {
    syscall(SYS_YIELD, 0, 0, 0);
}