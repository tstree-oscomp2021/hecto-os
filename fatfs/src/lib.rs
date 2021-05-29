//! A FAT filesystem library implemented in Rust.
//!
//! # Usage
//!
//! This crate is [on crates.io](https://crates.io/crates/fatfs) and can be
//! used by adding `fatfs` to the dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! fatfs = "0.3"
//! ```
//!
//! And this in your crate root:
//!
//! ```rust
//! extern crate fatfs;
//! ```
//!
//! # Examples
//!
//! ```rust
//! // Declare external crates
//! // Note: `fscommon` crate is used to speedup IO operations
//! extern crate fatfs;
//! extern crate fscommon;
//!
//! use std::io::prelude::*;
//!
//! fn main() -> std::io::Result<()> {
//!     # std::fs::copy("resources/fat16.img", "tmp/fat.img")?;
//!     // Initialize a filesystem object
//!     let img_file = std::fs::OpenOptions::new().read(true).write(true)
//!         .open("tmp/fat.img")?;
//!     let buf_stream = fscommon::BufStream::new(img_file);
//!     let fs = fatfs::FileSystem::new(buf_stream, fatfs::FsOptions::new())?;
//!     let root_dir = fs.root_dir();
//!
//!     // Write a file
//!     root_dir.create_dir("foo")?;
//!     let mut file = root_dir.create_file("foo/hello.txt")?;
//!     file.truncate()?;
//!     file.write_all(b"Hello World!")?;
//!
//!     // Read a directory
//!     let dir = root_dir.open_dir("foo")?;
//!     for r in dir.iter() {
//!         let entry = r?;
//!         println!("{}", entry.file_name());
//!     }
//!     # std::fs::remove_file("tmp/fat.img")?;
//!     # Ok(())
//! }
//! ```

#![crate_type = "lib"]
#![crate_name = "fatfs"]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(all(not(feature = "std"), feature = "alloc"), feature(alloc))]
// Disable warnings to not clutter code with cfg too much
#![cfg_attr(not(feature = "alloc"), allow(dead_code, unused_imports))]
// Inclusive ranges requires Rust 1.26.0
#![allow(ellipsis_inclusive_range_patterns)]
// `dyn` syntax requires Rust 1.27.0
#![allow(bare_trait_objects)]
// `alloc` compiler feature is needed in Rust before 1.36
#![cfg_attr(all(not(feature = "std"), feature = "alloc"), allow(stable_features))]

extern crate byteorder;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;

#[cfg(feature = "chrono")]
extern crate chrono;

#[cfg(not(feature = "std"))]
extern crate core_io;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

mod boot_sector;
mod dir;
mod dir_entry;
mod file;
mod fs;
mod table;
mod time;

#[cfg(not(feature = "std"))]
mod byteorder_core_io;

#[cfg(feature = "std")]
use byteorder as byteorder_ext;
#[cfg(not(feature = "std"))]
use byteorder_core_io as byteorder_ext;
#[cfg(not(feature = "std"))]
use core_io as io;
#[cfg(feature = "std")]
use std as core;

#[cfg(feature = "std")]
use std::io;

pub use dir::*;
pub use dir_entry::*;
pub use file::*;
pub use fs::*;
pub use time::*;

bitflags! {
    /// 文件类型和访问权限
    /// `man 7 inode`
    #[derive(Default)]
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

/// file status. 128 byte
#[derive(Default)]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,          /* ID of device containing file */
    pub st_ino: u64,          /* Inode number */
    pub st_mode: StatMode,    /* File type and mode */
    pub st_nlink: u32,        /* Number of hard links */
    pub st_uid: u32,          /* User ID of owner */
    pub st_gid: u32,          /* Group ID of owner */
    pub st_rdev: u64,         /* Device ID (if special file) */
    pub __pad: usize,         /*  */
    pub st_size: isize,       /* Total size, in bytes */
    pub st_blksize: u32,      /* Block size for filesystem I/O */
    pub __pad2: i32,          /*  */
    pub st_blocks: u64,       /* Number of 512B blocks allocated */
    pub st_atime_sec: isize,  /* Time of last access */
    pub st_atime_nsec: isize, /*  */
    pub st_mtime_sec: isize,  /* Time of last modification */
    pub st_mtime_nsec: isize, /*  */
    pub st_ctime_sec: isize,  /* Time of last status change */
    pub st_ctime_nsec: isize, /*  */
    pub __unused: [u32; 2],   /*  */
}

/// DirEntry. 24 byte
#[derive(Default)]
#[repr(C)]
pub struct LinuxDirent64 {
    pub d_ino: u64,       /* 64-bit inode number */
    pub d_off: i64,       /* 64-bit offset to next structure */
    pub d_reclen: u16,    /* Size of this dirent */
    pub d_type: u8,       /* File type */
    pub d_name: [u8; 11], /* Filename (null-terminated) */
}

pub trait Inode: ReadWriteSeek {
    fn get_fstat(&self) -> Stat;
    fn get_dents64(&self) -> LinuxDirent64;
}
