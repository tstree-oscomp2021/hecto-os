use crate::{get_current_thread, mm::VA};

/// Linux 中 brk(0) 只返回 0，此处与 Linux 表现不同，返回数据段末
pub(super) fn sys_brk(addr: VA) -> isize {
    get_current_thread()
        .process
        .inner
        .lock()
        .memory_set
        .brk(addr)
}
