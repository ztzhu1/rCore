use super::kernel_stack::KernelStack;
use super::pid::{pid_alloc, PidHandle};
use super::suspend_curr_and_run_next;
use crate::config::TRAP_CONTEXT;
use crate::fs::stdio::{Stdin, Stdout};
use crate::fs::File;
use crate::mm::address::{ppn_t, PhysAddr, VirtAddr};
use crate::mm::address_space::AddressSpace;
use crate::mm::page_table::translated_refmut;
use crate::safe_refcell::UPSafeRefCell;
use crate::trap::trap_handler;
use crate::trap::{trap_return, TrapContext};

use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::{Borrow, BorrowMut};
use core::cell::{Ref, RefMut};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ProcessStatus {
    UNINIT,
    READY,
    RUNNING,
    ZOMBIE,
    EXITED,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ProcessContext {
    ra: usize,
    sp: usize,
    reg_s: [usize; 12],
}

impl ProcessContext {
    pub fn empty() -> Self {
        Self {
            ra: 0,
            sp: 0,
            reg_s: [0; 12],
        }
    }

    pub fn from_goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            reg_s: [0; 12],
        }
    }
}
pub struct ProcessControlBlock {
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeRefCell<ProcessControlBlockInner>,
}

impl ProcessControlBlock {
    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (user_space, user_sp, entry_point) = AddressSpace::from_elf(elf_data);
        let trap_cx_ppn = user_space
            .translate(VirtAddr::from(TRAP_CONTEXT).vpn())
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // push a process context which goes to trap_return to the top of kernel stack
        let process_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeRefCell::new(ProcessControlBlockInner {
                    status: ProcessStatus::READY,
                    context: ProcessContext::from_goto_trap_return(kernel_stack_top),
                    trap_cx_ppn,
                    user_space,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: vec![
                        // 0 -> stdin
                        Some(Arc::new(Stdin)),
                        // 1 -> stdout
                        Some(Arc::new(Stdout)),
                        // 2 -> stderr
                        Some(Arc::new(Stdout)),
                    ],
                })
            },
        };
        // prepare TrapContext in user space
        let trap_cx = process_control_block.borrow().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(entry_point, user_sp, kernel_stack_top);
        process_control_block
    }

    pub fn fork(self: &Arc<ProcessControlBlock>) -> Arc<ProcessControlBlock> {
        // ---- access parent PCB exclusively
        let mut parent_inner = self.inner_borrow_mut();
        // copy user space(include trap context)
        let child_user_space = AddressSpace::from_user_space(&parent_inner.user_space);
        let trap_cx_ppn = child_user_space
            .translate(VirtAddr::from(TRAP_CONTEXT).vpn())
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // copy fd table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent_inner.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
        let process_control_block = Arc::new(ProcessControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeRefCell::new(ProcessControlBlockInner {
                    trap_cx_ppn,
                    context: ProcessContext::from_goto_trap_return(kernel_stack_top),
                    status: ProcessStatus::READY,
                    user_space: child_user_space,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                })
            },
        });
        // add child
        parent_inner.children.push(process_control_block.clone());
        // modify kernel_sp in trap_cx
        // **** access children PCB exclusively
        let trap_cx = process_control_block.inner_borrow().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;
        // return
        process_control_block
        // ---- stop exclusively accessing parent/children PCB automatically
    }

    pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (user_space, user_sp, entry_point) = AddressSpace::from_elf(elf_data);
        let trap_cx_ppn = user_space
            .translate(VirtAddr::from(TRAP_CONTEXT).vpn())
            .ppn();

        let mut user_sp = user_sp;
        // push arguments on user stack
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    user_space.token(),
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0;
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(user_space.token(), p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(user_space.token(), p as *mut u8) = 0;
        }
        // make the user_sp aligned to 8B
        user_sp -= user_sp % core::mem::size_of::<usize>();

        // **** access inner exclusively
        let mut inner = self.inner_borrow_mut();
        // substitute user space
        inner.user_space = user_space;
        // update trap_cx ppn
        inner.trap_cx_ppn = trap_cx_ppn;
        // initialize trap_cx
        let trap_cx = inner.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(entry_point, user_sp, self.kernel_stack.get_top());
        // for `main(int argc, char **argv)`
        trap_cx.gp[10] = args.len();
        trap_cx.gp[11] = argv_base;
    }

    pub fn get_pid(&self) -> usize {
        self.pid.0
    }

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        unsafe {
            ((PhysAddr::from_ppn(self.inner_borrow().trap_cx_ppn).0) as *mut TrapContext)
                .as_mut()
                .unwrap()
        }
    }

    pub fn get_user_token(&self) -> usize {
        self.inner_borrow().user_space.token()
    }

    pub fn inner_borrow(&self) -> Ref<ProcessControlBlockInner> {
        self.inner.borrow()
    }

    pub fn inner_borrow_mut(&self) -> RefMut<ProcessControlBlockInner> {
        self.inner.borrow_mut()
    }
}

pub struct ProcessControlBlockInner {
    pub trap_cx_ppn: ppn_t,
    pub context: ProcessContext,
    pub status: ProcessStatus,
    pub user_space: AddressSpace,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
}

impl ProcessControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        unsafe {
            (PhysAddr::from_ppn(self.trap_cx_ppn).0 as *mut TrapContext)
                .as_mut()
                .unwrap()
        }
    }

    pub fn get_user_token(&self) -> usize {
        self.user_space.token()
    }

    pub fn get_status(&self) -> ProcessStatus {
        self.status
    }

    pub fn is_zombie(&self) -> bool {
        self.get_status() == ProcessStatus::ZOMBIE
    }

    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
}
