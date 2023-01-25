#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    reg_s: [usize; 12],
}

impl TaskContext {
    pub fn new() -> Self {
        Self {
            ra: 0,
            sp: 0,
            reg_s: [0; 12],
        }
    }
}
