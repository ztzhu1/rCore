use core::arch::asm;

pub fn print_call_stack() {
    extern "C" {
        fn boot_stack_top();
    }
    let mut fp: usize;
    let mut parent_fp: usize;
    unsafe {
        asm!("mv {0}, fp", out(reg) fp);
    }

    loop {
        println!("-----------");
        println!("fp: {:#x}", fp);
        if fp == boot_stack_top as usize {
            println!("no parent!");
            break;
        }
        unsafe {
            asm!("ld {0}, -16(fp)", out(reg) parent_fp);
        }
        println!("parent_fp: {:#x}", parent_fp);

        fp = parent_fp;
    }
}
