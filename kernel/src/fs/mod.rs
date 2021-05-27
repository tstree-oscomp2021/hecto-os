use fatfs::{Dir, FileSystem, FsOptions};
use lazy_static::*;

use crate::drivers::BufBlockDevice;

mod file;
pub mod pipe;
mod vnode;

pub use file::{file_open, mkdir, FileDescriptor, OpenFlags, StatMode, STDIN, STDOUT};
pub use pipe::{PipeRead, PipeWrite};
pub use vnode::Vnode;

#[cfg(feature = "qemu-virt-rv64")]
type BlockDeviceImpl = crate::drivers::VirtIOBlock;
#[cfg(feature = "k210")]
type BlockDeviceImpl = crate::drivers::SDCardWrapper;

lazy_static! {
    // 文件系统
    pub static ref FILE_SYSTEM: FileSystem<BufBlockDevice<BlockDeviceImpl>> = {
        println!("init FAT32");
        FileSystem::new(BufBlockDevice::new(), FsOptions::new()).unwrap()
    };
    // 根目录
    pub static ref ROOT_DIR: Dir<'static, BufBlockDevice<BlockDeviceImpl>> = FILE_SYSTEM.root_dir();
}

/// 初始化块设备驱动和文件系统
pub fn init() {
    lazy_static::initialize(&FILE_SYSTEM);
    // test_fat32();
}

#[allow(unused)]
pub fn test_fat32() {
    for app in ROOT_DIR.iter() {
        print!("{}\t", app.unwrap().file_name());
    }
    println!();
}
