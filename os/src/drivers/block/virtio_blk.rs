use super::BlockDevice;
use crate::{
    mm::{
        address::{ppn_t, PhysAddr, VirtAddr},
        frame_allocator::{frame_alloc, frame_dealloc, FrameTracker},
        kernel_token,
        page_table::PageTable,
    },
    safe_refcell::UPSafeRefCell,
};
use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};

#[allow(unused)]
const VIRTIO0: usize = 0x10001000;

pub struct VirtIOBlock(UPSafeRefCell<VirtIOBlk<'static, VirtioHal>>);

lazy_static! {
    static ref QUEUE_FRAMES: UPSafeRefCell<Vec<FrameTracker>> =
        unsafe { UPSafeRefCell::new(Vec::new()) };
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .borrow_mut()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .borrow_mut()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        unsafe {
            Self(UPSafeRefCell::new(
                VirtIOBlk::<VirtioHal>::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}

pub struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> usize {
        let mut ppn_base: ppn_t = 0;
        for i in 0..pages {
            let frame = frame_alloc().unwrap();
            if i == 0 {
                ppn_base = frame.ppn;
            }
            assert_eq!(frame.ppn, ppn_base + i);
            QUEUE_FRAMES.borrow_mut().push(frame);
        }
        let pa: PhysAddr = ppn_base.into();
        pa.0
    }

    fn dma_dealloc(pa: usize, pages: usize) -> i32 {
        let pa = PhysAddr::from(pa);
        let mut ppn_base = pa.ppn();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base += 1;
        }
        0
    }

    fn phys_to_virt(addr: usize) -> usize {
        addr
    }

    fn virt_to_phys(vaddr: usize) -> usize {
        PageTable::from_token(kernel_token())
            .vaddr_to_paddr(VirtAddr::from(vaddr))
            .unwrap()
            .0
    }
}
