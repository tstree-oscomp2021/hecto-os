pub mod interface {
    pub use crate::mm::interface::Config;
}

#[cfg(feature = "qemu-virt-rv64")]
#[path = "../boards/qemu-virt-rv64/mod.rs"]
mod board_impl;

#[cfg(feature = "k210")]
#[path = "../boards/k210/mod.rs"]
mod board_impl;

pub use board_impl::{config::ConfigImpl, init_board, symbol::*};
