use super::frame_allocator::{frame_alloc, FrameTracker};
use super::page_table::{PTEFlags, PageTable};
use super::{address::*, page_table};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

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
                let dst = core::slice::from_raw_parts_mut(dst.0 as *mut u8, PAGE_SIZE);
                dst.copy_from_slice(src);
            }
            start += PAGE_SIZE;
            vpn_curr.step();
        }
    }
}

pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    pub fn new() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
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

    // pub fn new_kernel() -> Self {
    //     extern "C" {
    //         fn stext();
    //         fn etext();
    //         fn srodata();
    //         fn erodata();
    //         fn sdata();
    //         fn edata();
    //         fn sbss_with_stack();
    //         fn ebss();
    //         fn ekernel();
    //         fn strampoline();
    //     }
    //     let mut memory_set = Self::new();
    // }
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
