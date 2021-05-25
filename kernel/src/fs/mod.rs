use fatfs::{Dir, FileSystem, FsOptions};
use lazy_static::*;

use crate::{drivers::*, io::Read};

mod file;
mod vnode;

pub use file::{file_open, mkdir, FileDescriptor, OpenFlags, StatMode, STDIN, STDOUT};
pub use vnode::Vnode;

#[cfg(feature = "qemu-virt-rv64")]
type BlockDeviceImpl = VirtIOBlock;
#[cfg(feature = "k210")]
type BlockDeviceImpl = SDCardWrapper;

lazy_static! {
    // 文件系统
    pub static ref FILE_SYSTEM: FileSystem<BufBlockDevice<BlockDeviceImpl>> = {
        info!("初始化块设备驱动和 FAT32 文件系统");
        FileSystem::new(BufBlockDevice::new(), FsOptions::new()).unwrap()
    };
    // 根目录
    pub static ref ROOT_DIR: Dir<'static, BufBlockDevice<BlockDeviceImpl>> = FILE_SYSTEM.root_dir();
}

/// 初始化块设备驱动和文件系统
pub fn init() {
    lazy_static::initialize(&FILE_SYSTEM);
}

#[allow(unused)]
pub fn test_fat32() {
    // for app in ROOT_DIR.iter() {
    //     print!("{}\t", app.unwrap().file_name());
    // }
    // println!();
    let mut file = ROOT_DIR.open_file("test_fat32/FAT32.md").unwrap();
    let mut data = alloc::string::String::new();
    file.read_to_string(&mut data).unwrap();
    println!("{}", data);
}
