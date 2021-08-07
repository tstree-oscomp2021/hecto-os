use alloc::sync::Arc;
use core::slice::from_raw_parts_mut;

use bitflags::bitflags;

use crate::{
    arch::{interface::PageTable, PTEImpl},
    get_current_thread,
    io::{Read, Seek, SeekFrom},
    mm::{flag::MapFlags, VA},
    VARangeOrd,
};

bitflags! {
    #[derive(Default)]
    pub struct PROT: usize {
        const PROT_READ         =           0x1;    /* page can be read */
        const PROT_WRITE        =           0x2;    /* page can be written */
        const PROT_EXEC         =           0x4;    /* page can be executed */
        const PROT_SEM          =           0x8;    /* page may be used for atomic ops */
        /* 0x10 forreserved for arch-specific use */
        /* 0x20 reserved for arch-specific use */
        const PROT_NONE         =           0x0;    /* page can not be accessed */
        const PROT_GROWSDOWN    =   0x0100_0000;    /* mprotect flag: extend change to start of growsdown vma */
        const PROT_GROWSUP      =   0x0200_0000;    /* mprotect flag: extend change to end of growsup vma */
    }
}

/// Linux 中 brk(0) 只返回 0，此处与 Linux 表现不同，返回数据段末
pub(super) fn sys_brk(addr: VA) -> isize {
    debug!("sys_brk(addr={:#x});", addr.0);
    get_current_thread()
        .process
        .inner
        .lock()
        .address_space
        .brk(addr)
}

/// 将文件或设备映射到内存中
/// mmap通过pagefault来实现？每次pagefault就读取文件内容放在页面里
pub(super) fn sys_mmap(
    addr: VA,        // 映射起始位置
    length: usize,   // 映射区域的长度
    prot: PROT,      // 映射的内存保护方式（即 map_perm）
    flags: MapFlags, // 映射是否与其他进程共享的标志
    fd: isize,       // 文件描述符
    offset: usize,   // 文件偏移量
) -> isize {
    debug!(
        "sys_mmap(addr={:#x}, length={:#x}, prot={:?}, flags={:?}, fd={}, offset={:#});",
        addr.0, length, prot, flags, fd, offset
    );

    let mut map_perm = PTEImpl::USER;
    map_perm.set(PTEImpl::READABLE, prot.contains(PROT::PROT_READ));
    map_perm.set(PTEImpl::WRITABLE, prot.contains(PROT::PROT_WRITE));
    map_perm.set(PTEImpl::EXECUTABLE, prot.contains(PROT::PROT_EXEC));

    if addr.0 != 0 {
        let mut process_inner = get_current_thread().process.inner.lock();
        for vpn in VARangeOrd(addr..addr + length).vpn_range() {
            // println!("vpn : {:#x?}", vpn);
            process_inner
                .address_space
                .page_table
                .modify_flags(vpn, map_perm);
        }
        return addr.0 as isize;
    }

    let mut process_inner = get_current_thread().process.inner.lock();
    let va_end = process_inner.address_space.alloc_user_area(length);
    process_inner
        .address_space
        .insert_framed_area(va_end - length..va_end, map_perm, None);

    if let Some(fd) = process_inner.fd_table.get_mut(fd as usize) {
        let fd = fd.as_mut().unwrap();
        unsafe {
            let buffer = from_raw_parts_mut((va_end.0 - length) as *mut u8, length);
            let fd = Arc::get_mut_unchecked(fd);
            fd.seek(SeekFrom::Start(offset as u64)).unwrap();
            fd.read(buffer).unwrap();
            *(va_end.0 as *mut u8) = b'\0';
        }
    }

    (va_end.0 - length) as isize
}

pub(super) fn sys_mprotect(
    _addr: VA,  // 映射起始位置
    len: usize, // 映射区域的长度
    prot: PROT, // 映射的内存保护方式（即 map_perm）
) -> isize {
    debug!(
        "sys_mprotect(addr={:#x}, len={:#x}, prot={:?});",
        _addr.0, len, prot,
    );

    use super::Errno;
    if _addr.0 == 0 {
        return 0 - Errno::EINVAL as isize;
    }

    0
}

pub(super) fn sys_munmap(addr: VA, _length: usize) -> isize {
    get_current_thread()
        .process
        .inner
        .lock()
        .address_space
        .remove_area(addr);
    0
}
