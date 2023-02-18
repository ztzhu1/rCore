use crate::mm::page_table::PageTableEntry;
use lazy_static::lazy_static;

/// page table
pub const OFFSET_WIDTH: usize = 12;
pub const PAGE_SIZE: usize = 4096;
pub const PTE_SIZE: usize = core::mem::size_of::<PageTableEntry>();
pub const PTE_NUM: usize = PAGE_SIZE / PTE_SIZE;
pub const PA_WIDTH_SV39: usize = 56;
pub const VA_WIDTH_SV39: usize = 39;
pub const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - OFFSET_WIDTH;
pub const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - OFFSET_WIDTH;

/// stack
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const USER_STACK_SIZE: usize = 4096 * 2;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

/// memory
#[macro_export]
macro_rules! MEMORY_START {
    () => {
        crate::ext::smemory as usize
    };
}
pub const MEMORY_END: usize = 0x80800000;

pub use crate::board::MMIO;
// #[cfg(feature = "board_qemu")]
// pub const MMIO: &[(usize, usize)] = &[
//     (0x10001000, 0x1000),
// ];