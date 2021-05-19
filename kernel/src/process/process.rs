use alloc::{collections::BTreeSet, string::String, sync::Arc, vec, vec::Vec};

use lazy_static::lazy_static;
use spin::Mutex;
use xmas_elf::ElfFile;

use crate::{
    arch::PTEImpl,
    board::{interface::Config, ConfigImpl},
    fs::{FileDescriptor, STDIN, STDOUT},
    mm::*,
};

lazy_static! {
    /// 内核进程，所有内核线程都属于该进程。
    /// 通过此进程来进行内核栈的分配
    pub static ref KERNEL_PROCESS: Arc<Process> = {
        info!("初始化内核进程");
        Arc::new(Process {
            pid: 0,
            inner: Mutex::new(ProcessInner {
                cwd: String::from("/"),
                memory_set: MemorySet {
                    page_table: crate::mm::page_table::kernel_page_table(),
                    areas: BTreeSet::new(),
                },
                fd_table: vec![Some(STDIN.clone()), Some(STDOUT.clone())],
            }),
        })
    };
}

pub type Pid = usize;

pub struct Process {
    pub pid: Pid,
    /// 可变的部分。如果要更高的细粒度，去掉 ProcessInner 的 Mutex，给里面的
    /// memory_set 等等分别加上
    pub inner: Mutex<ProcessInner>,
}

pub struct ProcessInner {
    /// 当前工作目录
    pub cwd: String,
    /// 进程中的线程公用页表 / 内存映射
    pub memory_set: MemorySet,
    /// 文件描述符（文件指针，指向一个文件表项 File）
    pub fd_table: Vec<Option<Arc<FileDescriptor>>>,
}

impl Process {
    /// 通过 ELF 文件创建用户进程
    pub fn from_elf(file: &ElfFile, pid: usize) -> Arc<Self> {
        Arc::new(Self {
            pid,
            inner: Mutex::new(ProcessInner {
                cwd: String::from("/"),
                memory_set: MemorySet::from_elf(file),
                fd_table: vec![Some(STDIN.clone()), Some(STDOUT.clone())],
            }),
        })
    }

    /// TODO 用户栈最好是能够和程序段贴紧一些，可以减少分配页表的开销
    /// TODO 加一个 field，保存被 unmap 过的 user_stack_top
    /// TODO 从 elf 创建进程的时候就对齐成 PAGE_SIZE
    /// 的话，这里就不需要向上取整了 TODO 按需分配
    pub fn alloc_user_stack(&self) -> VA {
        let mut inner = self.inner.lock();
        let last = inner.memory_set.areas.last().unwrap().va_range.end.0;
        let user_stack_top = VA(((last + ConfigImpl::PAGE_SIZE - 1)
            & !(ConfigImpl::PAGE_SIZE - 1))
            + ConfigImpl::PAGE_SIZE
            + ConfigImpl::USER_STACK_SIZE);
        inner.memory_set.insert_framed_area(
            user_stack_top - ConfigImpl::USER_STACK_SIZE..user_stack_top,
            PTEImpl::READABLE | PTEImpl::WRITABLE | PTEImpl::USER,
        );

        user_stack_top
    }

    #[inline]
    /// **UNSAFE**
    pub(super) unsafe fn dealloc_user_stack(&self, user_stack_top: VA) {
        self.inner.lock().memory_set.remove_area(user_stack_top);
    }
}
