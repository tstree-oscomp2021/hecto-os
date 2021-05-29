use alloc::sync::Arc;
use core::slice::from_raw_parts_mut;

use bitflags::bitflags;

use crate::{
    arch::PTEImpl,
    get_current_thread,
    io::{Read, Seek, SeekFrom},
    mm::VA,
};

bitflags! {
    #[derive(Default)]
    pub struct PROT: usize {
        const NONE =             0;
        /// 可读位
        const READABLE =    1 << 0;
        /// 可写位
        const WRITABLE =    1 << 1;
        /// 可执行位
        const EXECUTABLE =  1 << 2;
    }
}

/// Linux 中 brk(0) 只返回 0，此处与 Linux 表现不同，返回数据段末
pub(super) fn sys_brk(addr: VA) -> isize {
    get_current_thread()
        .process
        .inner
        .lock()
        .memory_set
        .brk(addr)
}

/// 将文件或设备映射到内存中
/// mmap通过pagefault来实现？每次pagefault就读取文件内容放在页面里
pub(super) fn sys_mmap(
    _addr: VA,     // 映射起始位置
    length: usize, // 映射区域的长度
    prot: PROT,    // 映射的内存保护方式（即 map_perm）
    _flags: usize, // 映射是否与其他进程共享的标志
    fd: usize,     // 文件描述符
    offset: usize, // 文件偏移量
) -> isize {
    let mut map_perm = PTEImpl::USER;
    map_perm.set(PTEImpl::READABLE, prot.contains(PROT::READABLE));
    map_perm.set(PTEImpl::WRITABLE, prot.contains(PROT::WRITABLE));
    map_perm.set(PTEImpl::EXECUTABLE, prot.contains(PROT::EXECUTABLE));

    let mut process_inner = get_current_thread().process.inner.lock();
    let va_end = process_inner.memory_set.alloc_user_area(length);
    process_inner
        .memory_set
        .insert_framed_area(va_end - length..va_end, map_perm, None);

    if let Some(fd) = process_inner.fd_table.get_mut(fd).unwrap() {
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

pub(super) fn sys_munmap(addr: VA, _length: usize) -> isize {
    get_current_thread()
        .process
        .inner
        .lock()
        .memory_set
        .remove_area(addr);
    0
}
