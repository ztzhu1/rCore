pub mod context;
pub mod id;
pub mod kernel_stack;
pub mod manager;
pub mod pcb;
pub mod processor;
pub mod signal;
pub mod switch;
pub mod tcb;
pub mod thread_user_res;

use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;

use self::context::TaskContext;
use self::manager::{add_task, remove_from_pid2proc, remove_task};
use self::pcb::{ProcessControlBlock, ProcessStatus};
use self::processor::{get_curr_proc, get_curr_task, schedule, take_curr_task};
use self::signal::SignalFlags;
use self::tcb::{TaskControlBlock, TaskStatus};
use self::thread_user_res::ThreadUserRes;
use crate::fs::inode::{open_file, OpenFlags};
use crate::sbi::{exit_failure, exit_success};

lazy_static! {
    pub static ref INITPROC: Arc<ProcessControlBlock> = {
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        ProcessControlBlock::new(v.as_slice())
    };
}

pub fn add_initproc() {
    // active lazy INITPROC
    let _ = INITPROC.clone();
}

pub fn suspend_curr_and_run_next() {
    // There must be an application running.
    let task = take_curr_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_borrow_mut();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- stop exclusively accessing current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

pub fn block_current_and_run_next() {
    let task = take_curr_task().unwrap();
    let mut task_inner = task.inner_borrow_mut();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    task_inner.task_status = TaskStatus::Blocked;
    drop(task_inner);
    schedule(task_cx_ptr);
}

pub const IDLE_PID: usize = 0;

/// Now assuming INITPROC doesn't exit.
pub fn exit_curr_and_run_next(exit_code: i32) {
    let task = take_curr_task().unwrap();
    let mut task_inner = task.inner_borrow_mut();
    let process = task.process.upgrade().unwrap();
    let tid = task_inner.res.as_ref().unwrap().tid;
    // record exit code
    task_inner.exit_code = Some(exit_code);
    task_inner.res = None;
    // here we do not remove the thread since we are still using the kstack
    // it will be deallocated when sys_waittid is called
    drop(task_inner);
    drop(task);
    // however, if this is the main thread of current process
    // the process should terminate at once
    if tid == 0 {
        let pid = process.get_pid();
        if pid == IDLE_PID {
            kernel!("Idle process exit with exit_code {} ...", exit_code);
            if exit_code != 0 {
                exit_failure();
            } else {
                exit_success();
            }
        }
        remove_from_pid2proc(pid);
        let mut process_inner = process.inner_borrow_mut();
        // mark this process as a zombie process
        process_inner.is_zombie = true;
        // record exit code of main process
        process_inner.exit_code = exit_code;

        {
            // move all child processes under init process
            let mut initproc_inner = INITPROC.inner_borrow_mut();
            for child in process_inner.children.iter() {
                child.inner_borrow_mut().parent = Some(Arc::downgrade(&INITPROC));
                initproc_inner.children.push(child.clone());
            }
        }

        // deallocate user res (including tid/trap_cx/ustack) of all threads
        // it has to be done before we dealloc the whole memory_set
        // otherwise they will be deallocated twice
        let mut recycle_res = Vec::<ThreadUserRes>::new();
        for task in process_inner.tasks.iter().filter(|t| t.is_some()) {
            let task = task.as_ref().unwrap();
            // if other tasks are Ready in TaskManager or waiting for a timer to be
            // expired, we should remove them.
            //
            // Mention that we do not need to consider Mutex/Semaphore since they
            // are limited in a single process. Therefore, the blocked tasks are
            // removed when the PCB is deallocated.
            remove_inactive_task(Arc::clone(&task));
            let mut task_inner = task.inner_borrow_mut();
            if let Some(res) = task_inner.res.take() {
                recycle_res.push(res);
            }
        }
        // dealloc_tid and dealloc_user_res require access to PCB inner, so we
        // need to collect those user res first, then release process_inner
        // for now to avoid deadlock/double borrow problem.
        drop(process_inner);
        recycle_res.clear();

        let mut process_inner = process.inner_borrow_mut();
        process_inner.children.clear();
        // deallocate other data in user space i.e. program code/data section
        process_inner.user_space.recycle_data_frames();
        // drop file descriptors
        process_inner.fd_table.clear();
        // remove all tasks
        process_inner.tasks.clear();
    }
    drop(process);
    // we do not have to save task context
    let mut _unused = TaskContext::empty();
    schedule(&mut _unused as *mut _);
}

pub fn current_add_signal(signal: SignalFlags) {
    let proc = get_curr_proc().unwrap();
    let mut task_inner = proc.inner_borrow_mut();
    task_inner.signals |= signal;
}

pub fn remove_inactive_task(task: Arc<TaskControlBlock>) {
    remove_task(Arc::clone(&task));
    // remove_timer(Arc::clone(&task));
}
