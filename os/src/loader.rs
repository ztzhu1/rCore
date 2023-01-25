const MAX_NUM_APP: usize = 6;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

pub struct Loader {
    num_app: usize,
    app_starts: [usize; MAX_NUM_APP + 1],
}

impl Loader {
    pub fn new() -> Self {
        extern "C" {
            fn _num_app();
        }
        unsafe {
            let num_app = (_num_app as *const usize).read_volatile();
            let mut app_starts = [0_usize; MAX_NUM_APP + 1];
            let app_start_ptr = _num_app as *const usize;
            let app_start_slice =
                core::slice::from_raw_parts(app_start_ptr.add(1), MAX_NUM_APP + 1);
            app_starts.copy_from_slice(app_start_slice);
            Self {
                num_app,
                app_starts,
            }
        }
    }

    pub fn get_num_app(&self) -> usize {
        self.num_app
    }

}

/// Only should be used in single thread context.
lazy_static! {
    static ref LOADER: SafeRefCell<Loader> = SafeRefCell::new(Loader::new());
}