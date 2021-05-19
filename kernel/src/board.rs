#[path = "../boards/qemu-virt-rv64/main.rs"]
mod board_impl;

pub mod interface {
    pub use crate::mm::interface::Config;
}

pub type ConfigImpl = board_impl::config::ConfigImpl;

pub use board_impl::symbol::*;
