use super::id::pid::{pid_alloc, PidHandle};
use super::id::RecycleAllocator;
use super::kernel_stack::KernelStack;
use super::manager::{add_task, insert_into_pid2proc};
use super::signal::{SignalActions, SignalFlags};
use super::suspend_curr_and_run_next;
use super::tcb::TaskControlBlock;
use crate::config::TRAP_CONTEXT;
use crate::fs::stdio::{Stdin, Stdout};
use crate::fs::File;
use crate::mm::address::{ppn_t, PhysAddr, VirtAddr};
use crate::mm::address_space::AddressSpace;
use crate::mm::page_table::translated_refmut;
use crate::mm::{kernel_token, KERNEL_SPACE};
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

pub struct ProcessControlBlock {
    // immutable
    pub pid: PidHandle,
    // mutable
    inner: UPSafeRefCell<ProcessControlBlockInner>,
}

impl ProcessControlBlock {
    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        // user_space with elf program headers/trampoline/trap context/user stack
        let (user_space, ustack_base, entry_point) = AddressSpace::from_elf(elf_data);
        // pid
        let pid_handle = pid_alloc();
        // process
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: unsafe {
                UPSafeRefCell::new(ProcessControlBlockInner {
                    is_zombie: false,
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
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                })
            },
        });
        // create a main thread, we should allocate ustack and trap_cx here
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            ustack_base,
            true,
        ));
        // prepare trap_cx of main thread
        let task_inner = task.inner_borrow();
        let trap_cx = task_inner.get_trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(entry_point, ustack_top, kstack_top);

        // add main thread to the process
        let mut process_inner = process.inner_borrow_mut();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        drop(process_inner);
        insert_into_pid2proc(process.get_pid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }

    pub fn fork(self: &Arc<ProcessControlBlock>) -> Arc<ProcessControlBlock> {
        /// Only support processes with a single thread.
        let mut parent_inner = self.inner_borrow_mut();
        assert_eq!(parent_inner.tasks.len(), 1);
        // copy user space(include trap context)
        let child_user_space = AddressSpace::from_user_space(&parent_inner.user_space);
        // alloc a pid
        let pid_handle = pid_alloc();
        // copy fd table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent_inner.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
        // create child process pcb
        let child = Arc::new(ProcessControlBlock {
            pid: pid_handle,
            inner: unsafe {
                UPSafeRefCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    user_space: child_user_space,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                })
            },
        });
        // add child
        parent_inner.children.push(child.clone());
        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent_inner
                .get_task(0)
                .inner_borrow_mut()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));
        // attach task to child process
        let mut child_inner = child.inner_borrow_mut();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);
        // modify kstack_top in trap_cx of this thread
        let task_inner = task.inner_borrow_mut();
        let trap_cx = task_inner.get_trap_cx();
        trap_cx.kernel_sp = task.kstack.get_top();
        drop(task_inner);
        insert_into_pid2proc(child.get_pid(), Arc::clone(&child));
        // add this thread to scheduler
        add_task(task);
        child
    }

    pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
        assert_eq!(self.inner_borrow().tasks.len(), 1);
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (user_space, ustack_base, entry_point) = AddressSpace::from_elf(elf_data);
        let new_token = user_space.token();
        // substitute user_space
        let mut inner = self.inner_borrow_mut();
        inner.user_space = user_space;
        let user_space_token = inner.user_space.token();
        // then we alloc user resource for main thread again
        // since memory_set has been changed
        let task = inner.get_task(0);
        drop(inner);

        let mut task_inner = task.inner_borrow_mut();
        task_inner.res.as_mut().unwrap().ustack_base = ustack_base;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();
        // push arguments on user stack
        let mut user_sp = task_inner.res.as_mut().unwrap().ustack_top();
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    user_space_token,
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
                *translated_refmut(user_space_token, p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(user_space_token, p as *mut u8) = 0;
        }
        // make the user_sp aligned to 8B
        user_sp -= user_sp % core::mem::size_of::<usize>();
        // initialize trap_cx
        let mut trap_cx =
            TrapContext::app_init_context(entry_point, user_sp, task.kstack.get_top());
        trap_cx.gp[10] = args.len();
        trap_cx.gp[11] = argv_base;
        *task_inner.get_trap_cx() = trap_cx;
    }

    pub fn get_pid(&self) -> usize {
        self.pid.0
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
    pub is_zombie: bool,
    pub user_space: AddressSpace,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    pub signals: SignalFlags,
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    pub task_res_allocator: RecycleAllocator,
}

impl ProcessControlBlockInner {
    pub fn get_user_token(&self) -> usize {
        self.user_space.token()
    }

    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }

    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }

    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }

    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }
}
