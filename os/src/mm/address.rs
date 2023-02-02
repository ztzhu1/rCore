#![allow(non_camel_case_types)]

use super::page_table::PageTableEntry;
use crate::config::*;
use core::fmt::{self, Debug, Formatter};

pub type offset_t = usize;
pub type ppn_t = usize;
pub type vpn_t = usize;

#[macro_export]
macro_rules! apply_mask {
    ($v: expr, $width: expr) => {
        $v & ((1usize << $width) - 1)
    };
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {
        Self(apply_mask!(v, PA_WIDTH_SV39))
    }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "paddr:{:#x}, ppn:{}, offset:{}",
            self.0,
            self.ppn(),
            self.offset()
        ))
    }
}

impl PhysAddr {
    pub fn from_ppn(ppn: ppn_t) -> Self {
        Self(ppn << OFFSET_WIDTH)
    }

    pub fn ppn(&self) -> ppn_t {
        apply_mask!(self.0 >> OFFSET_WIDTH, PPN_WIDTH_SV39)
    }

    pub fn offset(&self) -> offset_t {
        apply_mask!(self.0, OFFSET_WIDTH)
    }

    pub fn aligned(&self) -> bool {
        self.offset() == 0
    }

    pub fn floor(&self) -> ppn_t {
        self.ppn()
    }

    pub fn ceil(&self) -> ppn_t {
        (self.0 + (PAGE_SIZE - 1)) / PAGE_SIZE
    }

    pub fn head(&self) -> Self {
        Self(self.ppn() << OFFSET_WIDTH)
    }

    pub fn get_bytes(&self) -> &'static mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.head().0 as *mut u8, PAGE_SIZE) }
    }

    pub fn get_ptes(&self) -> &'static mut [PageTableEntry] {
        unsafe { core::slice::from_raw_parts_mut(self.head().0 as *mut PageTableEntry, PTE_NUM) }
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

impl From<usize> for VirtAddr {
    fn from(v: usize) -> Self {
        Self(apply_mask!(v, VA_WIDTH_SV39))
    }
}

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "vaddr:{:#x}, vpn:{}, offset:{}",
            self.0,
            self.vpn(),
            self.offset()
        ))
    }
}

impl VirtAddr {
    pub fn from_vpn(vpn: vpn_t) -> Self {
        Self(vpn << OFFSET_WIDTH)
    }

    pub fn vpn(&self) -> vpn_t {
        apply_mask!(self.0 >> OFFSET_WIDTH, VPN_WIDTH_SV39)
    }

    pub fn offset(&self) -> offset_t {
        apply_mask!(self.0, OFFSET_WIDTH)
    }

    pub fn aligned(&self) -> bool {
        self.offset() == 0
    }

    pub fn floor(&self) -> vpn_t {
        self.vpn()
    }

    pub fn ceil(&self) -> vpn_t {
        (self.0 + (PAGE_SIZE - 1)) / PAGE_SIZE
    }

    pub fn head(&self) -> Self {
        Self(self.vpn() << OFFSET_WIDTH)
    }

    pub fn indices(&self) -> [usize; 3] {
        let mut vpn = self.vpn();
        let mut ret = [0; 3];
        for i in (0..3).rev() {
            ret[i] = apply_mask!(vpn, 9);
            vpn >>= 9;
        }
        ret
    }
}

pub trait StepByOne {
    fn step(&mut self);
}

impl StepByOne for vpn_t {
    fn step(&mut self) {
        *self += 1;
    }
}

#[derive(Copy, Clone)]
/// a simple range structure for type T
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}

impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }
    pub fn get_start(&self) -> T {
        self.l
    }
    pub fn get_end(&self) -> T {
        self.r
    }
}

impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}

/// iterator for the simple range structure
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}

impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}

impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}

/// a simple range structure for virtual page number
pub type VPNRange = SimpleRange<vpn_t>;
