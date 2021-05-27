#[cfg(feature = "qemu-virt-rv64")]
#[path = "../boards/qemu-virt-rv64/main.rs"]
mod board_impl;

#[cfg(feature = "k210")]
#[path = "../boards/k210/main.rs"]
mod board_impl;

pub mod interface {
    pub use crate::mm::interface::Config;
}

pub type ConfigImpl = board_impl::config::ConfigImpl;

pub use board_impl::{init_board, symbol::*};
