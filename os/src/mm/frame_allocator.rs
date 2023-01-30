use super::address::*;
use crate::{lang_items, safe_refcell::SafeRefCell};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::lazy_static;

pub const MEMORY_END: usize = 0x80800000;

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<ppn_t>;
    fn dealloc(&mut self, ppn: ppn_t);
}

pub struct StackFrameAllocator {
    current: ppn_t,
    end: ppn_t,
    recycled: Vec<ppn_t>,
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> Option<ppn_t> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn)
        } else {
            if self.current == self.end {
                None
            } else {
                self.current += 1;
                Some(self.current - 1)
            }
        }
    }

    fn dealloc(&mut self, ppn: ppn_t) {
        assert!(ppn < self.current);
        assert!(!self.recycled.contains(&ppn));
        self.recycled.push(ppn);
    }
}

impl StackFrameAllocator {
    pub fn init(&mut self, currend: ppn_t, end: ppn_t) {
        self.current = currend;
        self.end = end;
    }
}

pub struct FrameTracker {
    pub ppn: ppn_t,
}

impl FrameTracker {
    pub fn new(ppn: ppn_t) -> Self {
        // page cleaning
        let bytes_array = PhysAddr::from_ppn(ppn).get_bytes();
        bytes_array.into_iter().map(|p| *p = 0);
        Self { ppn }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "[kernel] frame_tracker: ppn:{} ({:#x})",
            self.ppn,
            self.ppn * PAGE_SIZE
        ))
    }
}

lazy_static! {
    static ref STACK_FRAME_ALLOCATOR: SafeRefCell<StackFrameAllocator> = SafeRefCell::new({
        extern "C" {
            fn ekernel();
        }
        let mut sfa = StackFrameAllocator::new();
        sfa.init(
            PhysAddr::from(ekernel as usize).ceil(),
            PhysAddr::from(MEMORY_END).floor(),
        );
        sfa
    });
}

pub fn frame_alloc() -> Option<FrameTracker> {
    Some(FrameTracker {
        ppn: STACK_FRAME_ALLOCATOR.borrow_mut().alloc()?,
    })
}

fn frame_dealloc(ppn: ppn_t) {
    STACK_FRAME_ALLOCATOR.borrow_mut().dealloc(ppn);
}

#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
