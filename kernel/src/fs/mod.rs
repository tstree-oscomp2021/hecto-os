use crate::{drivers::*, io::*};
use fatfs::{Dir, FileSystem, FsOptions};
use lazy_static::*;

mod stdio;

pub use stdio::{STDIN, STDOUT};

lazy_static! {
    // 文件系统
    pub static ref FILE_SYSTEM: FileSystem<BlockDeviceImpl> = {
        info!("初始化块设备驱动和 FAT32 文件系统");
        FileSystem::new(BlockDeviceImpl::new(), FsOptions::new()).unwrap()
    };
    // 根目录
    pub static ref ROOT_DIR: Dir<'static, BlockDeviceImpl> = FILE_SYSTEM.root_dir();
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
