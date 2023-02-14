pub mod address;
pub mod address_space;
pub mod frame_allocator;
pub mod heap_allocator;
pub mod page_table;

use alloc::sync::Arc;
use lazy_static::lazy_static;

use self::address_space::AddressSpace;
use crate::safe_refcell::UPSafeRefCell;

pub fn init() {
    heap_allocator::init_heap();
    KERNEL_SPACE.borrow().activate();
}

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeRefCell<AddressSpace>> =
        Arc::new(UPSafeRefCell::new(AddressSpace::new_kernel()));
}
