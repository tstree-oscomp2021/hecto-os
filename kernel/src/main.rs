#![no_std]
#![no_main]
#![allow(incomplete_features)]
#![feature(
    asm,
    global_asm,
    llvm_asm,
    panic_info_message,
    alloc_error_handler,
    format_args_nl,
    step_trait_ext,
    step_trait,
    get_mut_unchecked,
    const_generics
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
