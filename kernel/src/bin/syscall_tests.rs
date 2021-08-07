#![no_main]
#![no_std]
#![feature(format_args_nl, asm)]

use libkernel::{kernel_init, switch_to_schedule, PA};

#[no_mangle]
pub fn rust_main(hart_id: usize, dtb_pa: PA) -> ! {
    kernel_init(hart_id, dtb_pa);
    switch_to_schedule(ARGS);
}

const ARGS: &[&[&str]] = &[
    &["yield"],
    &["write"],
    &["waitpid"],
    &["wait"],
    &["unlink"],
    &["uname"],
    &["umount"],
    &["times"],
    &["read"],
    &["pipe"],
    &["openat"],
    &["open"],
    &["munmap"],
    &["mount"],
    &["mmap"],
    &["mkdir_"],
    &["gettimeofday"],
    &["getppid"],
    &["getpid"],
    &["getdents"],
    &["getcwd"],
    &["fstat"],
    &["fork"],
    &["exit"],
    &["execve"],
    &["dup2"],
    &["dup"],
    &["close"],
    &["clone"],
    &["chdir"],
    &["brk"],
    &["sleep"],
];
