#![no_std]
#![allow(unreachable_code)]
#![allow(unused)]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

mod bitmap;
mod block_cache;
mod block_dev;
mod efs;
mod layout;
mod vfs;

extern crate alloc;

pub const BLOCK_SIZE: usize = 512;
pub use block_dev::BlockDevice;
pub use efs::EasyFileSystem;
