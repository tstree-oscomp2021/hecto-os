#![allow(non_snake_case)]

use alloc::{boxed::Box, string::String, sync::Arc};

use bitflags::*;
use lazy_static::lazy_static;

use super::{
    vnode::{CONSOLE_VNODE, VNODE_HASHSET},
    FileSystem, Vnode, *,
};
use crate::{
    arch::{interface::Console, ConsoleImpl},
    drivers::BufBlockDevice,
    io::{Error, ErrorKind, Read, Seek, SeekFrom, Write},
};

bitflags! {
    /// `/usr/include/bits/fcntl-linux.h`
    pub struct OpenFlags: usize {
        /// read only
        const RDONLY    =      0;
        /// write only
        const WRONLY    = 1 << 0;
        /// read write
        const RDWR      = 1 << 1;
        /// create file if it does not exist
        const CREAT    = 1 << 6;
        /// error if CREATE and the file exists
        const EXCL = 1 << 7;
        /// truncate file upon open
        const TRUNC  = 1 << 9;
        /// append on each write
        const APPEND    = 1 << 10;
        /// close on exec
        const CLOEXEC   = 1 << 19;
        ///
        const DIRECTORY = 1 << 21;
    }
}

bitflags! {
    /// 文件类型和访问权限
    /// `man 7 inode`
    pub struct StatMode: u32 {
        const S_IFMT    = 0o017_0000;   /* bit mask for the file type bit field */

        const S_IFSOCK  = 0o014_0000;   /* socket */
        const S_IFLNK   = 0o012_0000;   /* symbolic link */
        const S_IFREG   = 0o010_0000;   /* regular file */
        const S_IFBLK   = 0o006_0000;   /* block device */
        const S_IFDIR   = 0o004_0000;   /* directory */
        const S_IFCHR   = 0o002_0000;   /* character device */
        const S_IFIFO   = 0o001_0000;   /* FIFO */

        const S_ISUID   = 0o000_4000;   /* set-user-ID bit (see execve(2)) */
        const S_ISGID   = 0o000_2000;   /* set-group-ID bit (see below) */
        const S_ISVTX   = 0o000_1000;   /* sticky bit (see below) */

        const S_IRWXU   = 0o000_0700;   /* owner has read, write, and execute permission */
        const S_IRUSR   = 0o000_0400;   /* owner has read permission */
        const S_IWUSR   = 0o000_0200;   /* owner has write permission */
        const S_IXUSR   = 0o000_0100;   /* owner has execute permission */

        const S_IRWXG   = 0o000_0070;   /* group has read, write, and execute permission */
        const S_IRGRP   = 0o000_0040;   /* group has read permission */
        const S_IWGRP   = 0o000_0020;   /* group has write permission */
        const S_IXGRP   = 0o000_0010;   /* group has execute permission */

        const S_IRWXO   = 0o000_0007;   /* others (not in group) have read, write, and execute permission */
        const S_IROTH   = 0o000_0004;   /* others have read permission */
        const S_IWOTH   = 0o000_0002;   /* others have write permission */
        const S_IXOTH   = 0o000_0001;   /* others have execute permission */

    }
}

impl OpenFlags {
    #[inline]
    fn readable(self) -> bool {
        self & (OpenFlags::RDONLY | OpenFlags::RDWR) != OpenFlags::WRONLY
    }

    #[inline]
    fn writable(self) -> bool {
        self & (OpenFlags::WRONLY | OpenFlags::RDWR) != OpenFlags::RDONLY
    }
}

lazy_static! {
    pub static ref STDIN: Arc<FileDescriptor> = Arc::new(FileDescriptor {
        flags: OpenFlags::RDONLY,
        pos: 0,
        vnode: CONSOLE_VNODE.clone(),
    });
    pub static ref STDOUT: Arc<FileDescriptor> = Arc::new(FileDescriptor {
        flags: OpenFlags::WRONLY,
        pos: 0,
        vnode: CONSOLE_VNODE.clone(),
    });
}

pub struct FileDescriptor {
    pub(crate) flags: OpenFlags,
    pub(crate) pos: u64,
    // 多个 fd 可指向同一个 vnode
    pub vnode: Arc<Vnode>,
}

impl Drop for FileDescriptor {
    /// XXX 待测试
    fn drop(&mut self) {
        if alloc::sync::Arc::<Vnode>::strong_count(&self.vnode) == 2 {
            VNODE_HASHSET.critical_section(|hs| hs.remove(&self.vnode));
        }
    }
}

impl Read for FileDescriptor {
    fn read(&mut self, buf: &mut [u8]) -> core_io::Result<usize> {
        if self.flags.readable() {
            unsafe { Arc::get_mut_unchecked(&mut self.vnode) }
                .inode
                .seek(SeekFrom::Start(self.pos))
                .unwrap();
            unsafe { Arc::get_mut_unchecked(&mut self.vnode) }
                .inode
                .read(buf)
        } else {
            Err(Error::from(ErrorKind::PermissionDenied))
        }
    }
}
impl Write for FileDescriptor {
    fn write(&mut self, buf: &[u8]) -> core_io::Result<usize> {
        if self.flags.writable() {
            unsafe { Arc::get_mut_unchecked(&mut self.vnode) }
                .inode
                .seek(SeekFrom::Start(self.pos))
                .unwrap();
            unsafe { Arc::get_mut_unchecked(&mut self.vnode) }
                .inode
                .write(buf)
        } else {
            Err(Error::from(ErrorKind::PermissionDenied))
        }
    }

    fn flush(&mut self) -> core_io::Result<()> {
        unsafe { Arc::get_mut_unchecked(&mut self.vnode) }
            .inode
            .flush()
    }
}
impl Seek for FileDescriptor {
    fn seek(&mut self, pos: core_io::SeekFrom) -> core_io::Result<u64> {
        unsafe { Arc::get_mut_unchecked(&mut self.vnode) }
            .inode
            .seek(pos)
    }
}

pub fn file_open(full_path: String, flags: OpenFlags) -> core_io::Result<Arc<FileDescriptor>> {
    let mut vnode = Arc::new(Vnode {
        fs: &(None, None),
        full_path,
        inode: Box::new(ConsoleImpl::CONSOLE_INSTANCE),
    });

    let mut vnode_set = VNODE_HASHSET.lock();
    if let Some(v) = vnode_set.get(&vnode) {
        vnode = v.clone();
    } else {
        let fs_dir = filesystem_lookup(&vnode.full_path);
        assert!(fs_dir.0.is_some());
        unsafe { Arc::get_mut_unchecked(&mut vnode) }.fs = fs_dir;

        let path = &vnode.full_path[fs_dir.0.as_ref().unwrap().mount_point.len()..];

        debug!("open {}", &vnode.full_path);
        unsafe { Arc::get_mut_unchecked(&mut vnode) }.inode = if flags.contains(OpenFlags::CREAT) {
            Box::new(fs_dir.1.as_ref().unwrap().create_file(path)?)
        } else if flags.contains(OpenFlags::DIRECTORY) {
            Box::new(fs_dir.1.as_ref().unwrap().open_dir(path)?)
        } else {
            Box::new(fs_dir.1.as_ref().unwrap().open_file(path)?)
        };

        vnode_set.insert(vnode.clone());
    }

    let pos = if flags.contains(OpenFlags::APPEND) {
        unsafe { Arc::get_mut_unchecked(&mut vnode) }
            .inode
            .seek(SeekFrom::End(0))
            .unwrap()
    } else {
        0
    };

    Ok(Arc::new(FileDescriptor { flags, pos, vnode }))
}

/// 删除文件
pub fn file_unlink(full_path: String) -> core_io::Result<()> {
    let fs_dir = filesystem_lookup(&full_path);
    // TODO 返回 Error: No such file
    assert!(fs_dir.0.is_some());
    fs_dir.1.as_ref().unwrap().remove(full_path.as_str())?;

    Ok(())
}

pub fn mkdir(full_path: String, _mode: StatMode) -> isize {
    debug!("mkdir {}", full_path);
    let fs_dir = filesystem_lookup(&full_path);
    // TODO 返回 Error: No such file
    assert!(fs_dir.0.is_some());
    if fs_dir
        .1
        .as_ref()
        .unwrap()
        .create_dir(full_path.as_str())
        .is_ok()
    {
        0
    } else {
        -1
    }
}

pub fn mount(full_path: String) {
    let fs_dir = filesystem_lookup(&full_path);
    // TODO 返回 Error: No such file
    assert!(fs_dir.0.is_some());
    // TODO 如果不是 Dir 返回 -1 而不是 panic
    fs_dir
        .1
        .as_ref()
        .unwrap()
        .open_dir(&full_path[fs_dir.0.as_ref().unwrap().mount_point.len()..])
        .unwrap();

    let fs = FileSystem {
        mount_point: full_path,
        fs: fs_dir.0.as_ref().unwrap().fs.clone(),
    };

    regeister_file_system(fs);
}

pub fn umount(full_path: String) {
    let fs_dir = filesystem_lookup(&full_path);
    // TODO 返回 Error: No such mount point
    assert!(fs_dir.0.is_some());

    // println!("umount {}", fs_dir.0.as_ref().unwrap().mount_point);

    #[allow(mutable_transmutes)]
    let fs_dir: &mut (
        Option<FileSystem>,
        Option<Dir<BufBlockDevice<BlockDeviceImpl>>>,
    ) = unsafe { transmute(fs_dir) };

    *fs_dir = (None, None);
}
