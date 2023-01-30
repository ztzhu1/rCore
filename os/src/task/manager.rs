use super::switch::__switch;
use super::tcb::{TaskContext, TaskControlBlock, TaskStatus};
use crate::loader::{app_num, init_app, examine_app_id_valid};
use crate::safe_refcell::SafeRefCell;
use lazy_static::lazy_static;
use crate::sbi::exit_success;

const MAX_TASK_NUM: usize = 6;

macro_rules! break_if_match {
    ($inst: ident, $next_id: ident, $id: ident) => {
        if $inst.tcbs[$id].status == TaskStatus::READY {
            $next_id = $id as isize;
            break;
        }
    };
}

pub struct TaskManager {
    task_num: usize,
    inner: SafeRefCell<TaskManagerInner>,
}

pub struct TaskManagerInner {
    curr_task: usize,
    tcbs: [TaskControlBlock; MAX_TASK_NUM],
}

impl TaskManager {
    pub fn run_first_task(&self) {
        // on boot stack
        let mut unused_cx = TaskContext::empty();
        let mut inner = self.inner.borrow_mut();
        let curr_task = inner.curr_task;
        examine_app_id_valid(curr_task);

        inner.tcbs[curr_task].status = TaskStatus::RUNNING;
        let target_task_cx = &inner.tcbs[curr_task].context as *const TaskContext;
        drop(inner);
        unsafe {
            __switch(&mut unused_cx as *mut TaskContext, target_task_cx);
        }
    }

    pub fn suspend_curr(&self) {
        let mut inner = self.inner.borrow_mut();
        let curr_task = inner.curr_task;
        let curr_tcb = &mut inner.tcbs[curr_task];
        if curr_tcb.status != TaskStatus::RUNNING {
            panic!();
        }
        curr_tcb.status = TaskStatus::READY;
    }

    pub fn exit_curr(&self) {
        let mut inner = self.inner.borrow_mut();
        let curr_task = inner.curr_task;
        let mut curr_tcb = &mut inner.tcbs[curr_task];
        if curr_tcb.status != TaskStatus::RUNNING {
            panic!();
        }
        curr_tcb.status = TaskStatus::EXITED;
    }

    pub fn switch_to_next(&self) {
        let mut inner = self.inner.borrow_mut();
        if inner.tcbs[inner.curr_task].status != TaskStatus::READY
            && inner.tcbs[inner.curr_task].status != TaskStatus::EXITED
        {
            panic!();
        }
        let mut next_id: isize = -1;
        if inner.curr_task == MAX_TASK_NUM - 1 {
            for id in 0..=inner.curr_task {
                break_if_match!(inner, next_id, id);
            }
        } else {
            for id in ((inner.curr_task + 1)..MAX_TASK_NUM).chain(0..=inner.curr_task) {
                break_if_match!(inner, next_id, id);
            }
        }

        if next_id == -1 {
            info!("No more apps!");
            exit_success();

        }

        let next_id = next_id as usize;
        let curr_task = inner.curr_task;

        inner.tcbs[next_id].status = TaskStatus::RUNNING;

        let curr_task_cx = &mut inner.tcbs[curr_task].context as *mut TaskContext;
        let target_task_cx = &inner.tcbs[next_id].context as *const TaskContext;
        inner.curr_task = next_id;
        drop(inner);
        unsafe {
            __switch(curr_task_cx, target_task_cx);
        }
    }
}

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        let mut tcbs = [TaskControlBlock::empty(); MAX_TASK_NUM];
        for app_id in 0..MAX_TASK_NUM {
            tcbs[app_id] = init_app(app_id);
        }
        TaskManager {
            task_num: app_num(),
            inner: SafeRefCell::new(TaskManagerInner {
                curr_task: 2,
                tcbs: tcbs,
            }),
        }
    };
}

pub fn suspend_curr_and_run_next() {
    TASK_MANAGER.suspend_curr();
    TASK_MANAGER.switch_to_next();
}

pub fn exit_curr_and_run_next() {
    TASK_MANAGER.exit_curr();
    TASK_MANAGER.switch_to_next();
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}
