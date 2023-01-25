mod app_manager;
mod stack;
pub use app_manager::{all_apps_done, next_app, num_app, run_next_app};
pub use stack::run_all_apps;
