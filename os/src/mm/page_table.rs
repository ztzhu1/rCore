use super::address::*;
use super::frame_allocator::{frame_alloc, FrameTracker};
use crate::apply_mask;
use alloc::string::String;
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

    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W).bits as usize != 0
    }

    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X).bits as usize != 0
    }

    pub fn map(&mut self, ppn: ppn_t, flags: PTEFlags) {
        *self = Self::new(ppn, flags);
    }

    pub fn unmap(&mut self) {
        *self = Self::empty();
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

    pub fn empty() -> Self {
        Self {
            root_ppn: 0,
            frames: Vec::new(),
        }
    }

    /**
     * PageTable only saves index_frame. The caller is
     * responsible for saving data_frame and passing ppn
     * to PageTable::map.
     */
    pub fn map(&mut self, vpn: vpn_t, ppn: ppn_t, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).expect("loop zero times!");
        assert!(!pte.is_valid());
        pte.map(ppn, flags | PTEFlags::V);
    }

    pub fn unmap(&mut self, vpn: vpn_t) {
        let pte = self.find_pte(vpn).expect("loop zero times!");
        assert!(pte.is_valid());
        pte.unmap();
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
                pte.map(ft.ppn, PTEFlags::V);
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

    pub fn token(&self) -> usize {
        // for RV64:
        // satp[63:60] = 0 => bare
        // satp[63:60] = 8 => Sv39
        // satp[63:60] = 9 => Sv48
        8_usize << 60 | self.root_ppn
    }

    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: apply_mask!(satp, 44),
            frames: Vec::new(),
        }
    }

    pub fn translate(&self, vpn: vpn_t) -> Option<&PageTableEntry> {
        self.find_pte(vpn).map(|pte| &*pte)
    }

    pub fn vaddr_to_paddr(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            //println!("translate_va:va = {:?}", va);
            let aligned_pa: PhysAddr = PhysAddr::from_ppn(pte.ppn());
            //println!("translate_va:pa_align = {:?}", aligned_pa);
            (aligned_pa.0 + va.offset()).into()
        })
    }
}

/// translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.vpn();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.offset() == 0 {
            v.push(&mut PhysAddr::from_ppn(ppn).get_bytes()[start_va.offset()..]);
        } else {
            v.push(&mut PhysAddr::from_ppn(ppn).get_bytes()[start_va.offset()..end_va.offset()]);
        }
        start = end_va.0;
    }
    v
}

pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8;
        unsafe {
            ch = *(page_table.vaddr_to_paddr(VirtAddr::from(va)).unwrap().0 as *const u8);
        }
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}

///translate a generic through page table and return a mutable reference
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    //println!("into translated_refmut!");
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    //println!("translated_refmut: before translate_va");
    let pa = page_table.vaddr_to_paddr(VirtAddr::from(va)).unwrap();
    unsafe { (pa.0 as *mut T).as_mut().unwrap() }
}
