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
    get_mut_unchecked,
    const_generics,
    const_raw_ptr_to_usize_cast
)]
// #![allow(dead_code)]
// #![allow(unused)]

#[macro_use]
pub mod logger;
pub mod backtrace;
pub mod drivers;
pub mod fs;
#[macro_use]
pub mod mm;
pub mod process;
pub mod sync;
pub mod syscall;
pub mod trap;

pub mod arch;
pub mod board;

use core_io as io;

extern crate alloc;
extern crate core_io;

pub use algorithm::*;
pub use fs::*;
pub use mm::*;
pub use process::*;
pub use sync::*;
