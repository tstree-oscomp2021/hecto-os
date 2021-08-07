#![no_main]
#![no_std]
#![feature(format_args_nl, asm)]

use libkernel::{kernel_init, switch_to_schedule, PA};

#[no_mangle]
pub fn rust_main(hart_id: usize, dtb_pa: PA) -> ! {
    kernel_init(hart_id, dtb_pa);
    switch_to_schedule(ARGS);
}

#[rustfmt::skip]
const ARGS: &[&[&str]] = &[
    &["busybox", "ash"],
];
