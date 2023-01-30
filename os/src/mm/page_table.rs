use crate::apply_mask;

#[macro_use]
use super::address::*;
use super::frame_allocator::{frame_alloc, FrameTracker};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry(pub usize);

impl PageTableEntry {
    pub fn new(ppn: ppn_t, flags: PTEFlags) -> Self {
        Self(ppn << 10 | flags.bits as usize)
    }

    pub fn empty() -> Self {
        Self(0)
    }

    pub fn ppn(&self) -> ppn_t {
        self.0 >> 10
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.0 as u8).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V).bits as usize != 0
    }
}

pub struct PageTable {
    root_ppn: ppn_t,
    frames: Vec<FrameTracker>,
}

impl PageTable {
    pub fn new() -> Self {
        let ft = frame_alloc().unwrap();
        Self {
            root_ppn: ft.ppn,
            frames: vec![ft],
        }
    }

    pub fn map(&mut self, vpn: vpn_t, ppn: ppn_t, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).expect("loop zero times!");
        assert!(!pte.is_valid());
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    pub fn unmap(&mut self, vpn: vpn_t) {
        let pte = self.find_pte(vpn).expect("loop zero times!");
        assert!(pte.is_valid());
        *pte = PageTableEntry::empty();
    }

    pub fn find_pte_create(&mut self, vpn: vpn_t) -> Option<&mut PageTableEntry> {
        let vaddr = VirtAddr::from_vpn(vpn);
        let indices = vaddr.indices();
        let mut paddr = PhysAddr::from_ppn(self.root_ppn);
        let mut result = None;
        for i in 0..indices.len() {
            let pte = &mut paddr.get_ptes()[indices[i]];
            if i == indices.len() - 1 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let ft = frame_alloc().expect("No more free memory!");
                *pte = PageTableEntry::new(ft.ppn, PTEFlags::V);
                self.frames.push(ft);
            }
            paddr = PhysAddr::from_ppn(pte.ppn());
        }
        result
    }

    /**
     * It will return the final pte whether it's valid
     * or not. But if the medium ptes is invalid, it
     * returns `None`.
     */
    fn find_pte(&self, vpn: vpn_t) -> Option<&mut PageTableEntry> {
        let vaddr = VirtAddr::from_vpn(vpn);
        let indices = vaddr.indices();
        let mut paddr = PhysAddr::from_ppn(self.root_ppn);
        let mut result = None;
        for i in 0..indices.len() {
            let pte = &mut paddr.get_ptes()[indices[i]];
            if i == indices.len() - 1 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            paddr = PhysAddr::from_ppn(pte.ppn());
        }
        result
    }

    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: apply_mask!(satp, 44),
            frames: Vec::new(),
        }
    }

    pub fn translate(&self, vpn: vpn_t) -> Option<&PageTableEntry> {
        Some(self.find_pte(vpn)?)
    }
}
