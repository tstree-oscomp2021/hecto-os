use alloc::{string::String, sync::Arc};
use core::mem::transmute;

use fatfs::{Dir, FileSystem as FatFs, FsOptions};
use lazy_static::*;

use crate::drivers::BufBlockDevice;

pub mod file;
pub mod flag;
pub mod pipe;
pub mod vnode;

pub use file::{FileDescriptor, OpenFlags, STDIN, STDOUT};
pub use pipe::{PipeRead, PipeWrite};
pub use vnode::Vnode;

#[cfg(feature = "qemu-virt-rv64")]
type BlockDeviceImpl = crate::drivers::VirtIOBlock;
#[cfg(feature = "k210")]
type BlockDeviceImpl = crate::drivers::SDCardWrapper;

/// 初始化块设备驱动和并挂载根文件系统
pub fn init() {
    println!("init rootfs");
    let rootfs = FileSystem {
        mount_point: String::from("/"),
        fs: Arc::new(FatFs::new(BufBlockDevice::new(), FsOptions::new()).unwrap()),
    };

    regeister_file_system(rootfs);
}

/// 文件系统
pub struct FileSystem {
    /// 挂载点
    pub mount_point: String,
    /// 实际的文件系统实现。
    /// TODO 换成 Box<dyn VFS>
    fs: Arc<FatFs<BufBlockDevice<BlockDeviceImpl>>>,
}

const MAX_FILE_SYSTEM: usize = 4;

lazy_static! {
    /// 文件系统表
    pub static ref FILE_SYSTEM_TABLE: [(Option<FileSystem>, Option<Dir<'static, BufBlockDevice<BlockDeviceImpl>>>); MAX_FILE_SYSTEM] = Default::default();
}

/// 查找 full_path 所在的文件系统
pub fn filesystem_lookup(
    full_path: &str,
) -> &'static (
    Option<FileSystem>,
    Option<Dir<'static, BufBlockDevice<BlockDeviceImpl>>>,
) {
    // println!("full_path {}", full_path);
    let path_len = full_path.len();
    let mut ret = &FILE_SYSTEM_TABLE[0];
    let mut fspath_len;
    let mut prev_len = 0;

    for fs_dir in FILE_SYSTEM_TABLE.iter().skip(1) {
        if let Some(fs) = fs_dir.0.as_ref() {
            // debug!("fs.mount_point {}", fs.mount_point);
            fspath_len = fs.mount_point.len();

            // 如果挂载点路径小于 full_path，并且比上次得到的长，并且 full_path
            // 的开头和挂载点一样
            if fspath_len <= path_len
                && fspath_len > prev_len
                && full_path.starts_with(&fs.mount_point)
            {
                // 如果挂载点路径与 full_path 长度相等，或者 full_path...
                if fspath_len == path_len || full_path.as_bytes()[fspath_len] == b'/' {
                    ret = fs_dir;
                    prev_len = fspath_len;
                }
            }
        }
    }

    ret
}

/// 在 FILE_SYSTEM_TABLE 中注册文件系统
pub fn regeister_file_system(fs: FileSystem) {
    for fs_dir in FILE_SYSTEM_TABLE.iter() {
        if fs_dir.0.is_none() {
            #[allow(mutable_transmutes)]
            let fs_dir: &mut (
                Option<FileSystem>,
                Option<Dir<BufBlockDevice<BlockDeviceImpl>>>,
            ) = unsafe { transmute(fs_dir) };

            let s: *const str = &*fs.mount_point;

            (*fs_dir).0 = Some(fs);
            (*fs_dir).1 = Some(fs_dir.0.as_ref().unwrap().fs.root_dir());

            vnode::VNODE_HASHSET.lock().insert(
                unsafe { &*s },
                Arc::new(Vnode {
                    fs: fs_dir,
                    full_path: fs_dir.0.as_ref().unwrap().mount_point.clone(),
                    inode: alloc::boxed::Box::new(fs_dir.0.as_ref().unwrap().fs.root_dir()),
                }),
            );

            return;
        }
    }
    panic!("FILE_SYSTEM_TABLE full");
}
