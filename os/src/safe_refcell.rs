use core::cell::{Ref, RefCell, RefMut};

pub struct UPSafeRefCell<T> {
    inner: RefCell<T>,
}

impl<T> UPSafeRefCell<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    pub fn borrow(&self) -> Ref<T> {
        self.inner.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.inner.borrow_mut()
    }
}

unsafe impl<T> Sync for UPSafeRefCell<T> {}
