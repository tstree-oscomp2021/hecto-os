#![no_std]
#![no_main]
#![feature(
    global_asm,
    llvm_asm,
    panic_info_message,
    alloc_error_handler,
    drain_filter,
    linked_list_remove,
    format_args_nl,
    step_trait_ext,
    step_trait,
    rustc_attrs,
    map_first_last,
    get_mut_unchecked
)]
// #![allow(dead_code)]
// #![allow(unused)]

#[macro_use]
pub mod logger;
pub mod backtrace;
pub mod config;
pub mod drivers;
pub mod ffi;
pub mod fs;
pub mod hart;
pub mod interrupt;
pub mod mm;
pub mod process;
pub mod register;
pub mod sbi;
pub mod spinlock;
pub mod syscall;

extern crate alloc;
extern crate core_io;
use core_io as io;

pub use algorithm::*;
pub use fs::*;
pub use mm::*;
pub use process::*;
