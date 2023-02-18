use crate::block_cache::get_block_cache;
use crate::block_dev::BlockDevice;
use crate::BLOCK_SIZE;

use alloc::sync::Arc;

const BLOCK_BITS: usize = BLOCK_SIZE * 8;
const GROUP_SIZE: usize = 8;
const GROUP_BITS: usize = 64;
const GROUP_NUM: usize = BLOCK_SIZE / GROUP_SIZE;
type BitmapBlock = [u64; GROUP_NUM];

pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {
            let block_cache = get_block_cache(
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            );
            let pos = block_cache
                .lock()
                .modify(0, |bitmap_block: &mut BitmapBlock| {
                    if let Some((group_pos, bit_pos)) = bitmap_block
                        .iter()
                        .enumerate()
                        .find(|(_, bits64)| **bits64 != u64::MAX)
                        .map(|(group_pos, bits64)| (group_pos, bits64.trailing_ones() as usize))
                    {
                        // modify cache
                        bitmap_block[group_pos] |= 1u64 << bit_pos;
                        Some(block_id * BLOCK_BITS + group_pos * 64 + bit_pos as usize)
                    } else {
                        // TODO: block_cache is dirty once we call
                        // `block_cache.modify`. But what if it's
                        // not actually modified, which is the case
                        // in this branch?
                        None
                    }
                });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, group_pos, bit_pos) = decomposition(bit);
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[group_pos] & (1u64 << bit_pos) > 0); // the bit does be alloced
                bitmap_block[group_pos] -= 1u64 << bit_pos;
            });
    }

    /// Get the max number of allocatable blocks
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}

fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_pos = bit / BLOCK_BITS;
    bit = bit % BLOCK_BITS;
    let group_pos = bit / GROUP_BITS;
    let bit_pos = bit % GROUP_BITS;
    (block_pos, group_pos, bit_pos)
}
