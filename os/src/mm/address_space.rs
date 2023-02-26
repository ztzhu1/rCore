use super::frame_allocator::{frame_alloc, FrameTracker};
use super::page_table::{PTEFlags, PageTable, PageTableEntry};
use super::{address::*, page_table};
use crate::config::*;
use crate::ext::*;
use crate::mm::KERNEL_SPACE;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::arch::asm;
use core::clone::Clone;
use riscv::register::satp;
use xmas_elf;

pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<vpn_t, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    pub fn new(
        va_start: VirtAddr,
        va_end: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        assert!(va_start.aligned());
        let vpn_start = va_start.floor();
        let vpn_end = va_end.ceil();
        Self {
            vpn_range: VPNRange::new(vpn_start, vpn_end),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range.into_iter() {
            self.map_one(page_table, vpn);
        }
    }

    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range.into_iter() {
            self.unmap_one(page_table, vpn);
        }
    }

    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: vpn_t) {
        let ppn;
        match self.map_type {
            MapType::IDENTICAL => {
                // There is no need to alloc another data frame.
                ppn = vpn;
            }
            MapType::FRAMED => {
                let ft = frame_alloc().unwrap();
                ppn = ft.ppn;
                self.data_frames.insert(vpn, ft);
            }
        }
        page_table.map(vpn, ppn, PTEFlags::from_bits(self.map_perm.bits).unwrap())
    }

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: vpn_t) {
        if self.map_type == MapType::FRAMED {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }

    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::FRAMED);
        let mut start: usize = 0;
        let len = data.len();
        let mut vpn_curr = self.vpn_range.get_start();
        let vpn_end = self.vpn_range.get_end();
        while start < len && vpn_curr < vpn_end {
            let dst = PhysAddr::from_ppn(page_table.translate(vpn_curr).unwrap().ppn());
            unsafe {
                let src = &data[start..len.min(start + PAGE_SIZE)];
                let dst = core::slice::from_raw_parts_mut(dst.0 as *mut u8, src.len());
                dst.copy_from_slice(src);
            }
            start += PAGE_SIZE;
            vpn_curr.step();
        }
    }
}

impl Clone for MapArea {
    fn clone(&self) -> Self {
        Self {
            vpn_range: VPNRange::new(self.vpn_range.get_start(), self.vpn_range.get_end()),
            data_frames: BTreeMap::new(),
            map_type: self.map_type,
            map_perm: self.map_perm,
        }
    }
}

pub struct AddressSpace {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl AddressSpace {
    pub fn new() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self {
            page_table: PageTable::empty(),
            areas: Vec::new(),
        }
    }

    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }

    pub fn recycle_data_frames(&mut self) {
        self.areas.clear();
    }

    pub fn vaddr_to_paddr(&self, vaddr: VirtAddr) -> Option<PhysAddr> {
        self.page_table.vaddr_to_paddr(vaddr)
    }

    pub fn translate(&self, vpn: vpn_t) -> PhysAddr {
        PhysAddr::from_ppn(self.page_table.translate(vpn).unwrap().ppn())
    }

    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }

    pub fn from_user_space(user_space: &Self) -> Self {
        let mut new_user_space = Self::new();
        // map trampoline
        new_user_space.map_trampoline();
        // copy data sections/trap_context/user_stack
        for area in user_space.areas.iter() {
            let new_area = area.clone();
            new_user_space.push(new_area, None);
            // copy data from another space
            for vpn in area.vpn_range {
                let src = user_space.translate(vpn).get_bytes();
                let dst = new_user_space.translate(vpn).get_bytes();
                dst.copy_from_slice(src);
            }
        }
        new_user_space
    }

    /// Assume that no conflicts.
    pub fn insert_framed_area(
        &mut self,
        va_start: VirtAddr,
        va_end: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(va_start, va_end, MapType::FRAMED, permission),
            None,
        );
    }

    pub fn insert_identical_area(
        &mut self,
        va_start: VirtAddr,
        va_end: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(va_start, va_end, MapType::IDENTICAL, permission),
            None,
        );
    }

    /// Create app address space.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut user_space = Self::new();
        user_space.map_trampoline();
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn: vpn_t = 0;
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va = VirtAddr::from(ph.virtual_addr() as usize);
                let end_va = VirtAddr::from((ph.virtual_addr() + ph.mem_size()) as usize);
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::FRAMED, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                user_space.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }
        // map user stack with U flags
        let max_end_va = VirtAddr::from_vpn(max_end_vpn);
        let mut user_stack_bottom: usize = VirtAddr::from_vpn(max_end_va.ceil()).0;
        // guard page
        let user_stack_base = user_stack_bottom + PAGE_SIZE;

        (
            user_space,
            user_stack_base,
            elf.header.pt2.entry_point() as usize,
        )
    }

    /// Create kernel address space.
    pub fn new_kernel() -> Self {
        let mut kernel_space = Self::new();
        kernel_space.map_trampoline();

        kernel_space.insert_identical_area(
            VirtAddr::from(stext as usize),
            VirtAddr::from(etext as usize),
            MapPermission::R | MapPermission::X,
        );
        kernel_space.insert_identical_area(
            VirtAddr::from(srodata as usize),
            VirtAddr::from(erodata as usize),
            MapPermission::R,
        );
        kernel_space.insert_identical_area(
            VirtAddr::from(sdata as usize),
            VirtAddr::from(edata as usize),
            MapPermission::R | MapPermission::W,
        );
        kernel_space.insert_identical_area(
            VirtAddr::from(sbss_with_stack as usize),
            VirtAddr::from(ebss as usize),
            MapPermission::R | MapPermission::W,
        );
        kernel_space.insert_identical_area(
            VirtAddr::from(MEMORY_START!()),
            VirtAddr::from(MEMORY_END),
            MapPermission::R | MapPermission::W,
        );
        for pair in MMIO {
            kernel_space.push(
                MapArea::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    MapType::IDENTICAL,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            );
        }
        kernel_space
    }

    /**
     * trampoline is not collected by areas.
     * It does be framed mapping but it stores text
     * instead of data. The space has been alloced
     * at `.text` section during compiling.
     *
     * In another word, trampoline is data framed
     * but we need its paddr to locate at a specified
     * area, it's not consistent with how MapArea works.
     */
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).vpn(),
            PhysAddr::from(strampoline as usize).ppn(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    pub fn remove_area_with_start_vpn(&mut self, start_vpn: vpn_t) {
        if let Some((idx, area)) = self
            .areas
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_range.get_start() == start_vpn)
        {
            area.unmap(&mut self.page_table);
            self.areas.remove(idx);
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    IDENTICAL,
    FRAMED,
}

bitflags! {
    /// map permission corresponding to that in pte: `R W X U`
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        // self.areas.iter_mut().for_each(|area| {
        //     area.unmap(&mut self.page_table);
        // });
    }
}

pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.borrow_mut();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_text.floor())
            .unwrap()
            .writable(),
        false
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_rodata.floor())
            .unwrap()
            .writable(),
        false,
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_data.floor())
            .unwrap()
            .executable(),
        false,
    );
    println!("remap_test passed!");
}
