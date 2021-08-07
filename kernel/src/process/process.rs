use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use core::sync::atomic::AtomicU64;

use lazy_static::lazy_static;
use xmas_elf::ElfFile;

use crate::{
    arch::PTEImpl,
    board::{interface::Config, ConfigImpl},
    fs::{FileDescriptor, STDIN, STDOUT},
    get_current_thread,
    mm::*,
    sync::SpinLock,
};

lazy_static! {
    /// 内核进程，所有内核线程都属于该进程。
    /// 通过此进程来进行内核栈的分配
    pub static ref KERNEL_PROCESS: Arc<Process> = {
        println!("init kernel process");
        Arc::new(Process {
            pid: 0,
            times: Default::default(),
            inner: SpinLock::new(ProcessInner {
                cwd: String::from("/"),
                address_space: AddressSpace {
                    page_table: crate::mm::page_table::kernel_page_table(),
                    areas: BTreeMap::<VARangeOrd, MapArea>::new(),
                    data_segment_end: VA(1),
                    data_segment_max: VA(1),
                },
                fd_table: vec![Some(STDIN.clone()), Some(STDOUT.clone()), Some(STDOUT.clone())],
                parent: Weak::new(),
                child: Vec::new(),
                child_exited: Vec::new(),
                wake_callbacks: Vec::new(),
            }),
        })
    };
}

pub type Pid = usize;

pub struct Process {
    pub pid: Pid,
    pub inner: SpinLock<ProcessInner>,
    pub times: Times,
}

#[derive(Default)]
pub struct Times {
    pub tms_utime: AtomicU64,  /* user time */
    pub tms_stime: AtomicU64,  /* system time */
    pub tms_cutime: AtomicU64, /* user time of children */
    pub tms_cstime: AtomicU64, /* system time of children */
}

pub struct ProcessInner {
    /// 当前工作目录
    pub cwd: String,
    /// 进程的地址空间
    pub address_space: AddressSpace,
    /// 文件描述符
    pub fd_table: Vec<Option<Arc<FileDescriptor>>>,
    /// 父进程
    pub parent: Weak<Process>,
    /// 子进程
    pub child: Vec<Weak<Process>>,
    /// 已经退出了的子进程 (进程ID, 弱引用，exit_status)，其中 exit_status
    /// 只有低 8 bit 有效
    pub child_exited: Vec<(Pid, Weak<Process>, i32)>,
    /// 回调
    pub wake_callbacks: Vec<Box<dyn Fn() + Send>>,
}

impl Process {
    /// 通过 ELF 文件创建用户进程
    pub fn from_elf(file: &ElfFile, pid: usize) -> Arc<Self> {
        Arc::new(Self {
            pid,
            times: Default::default(),
            inner: SpinLock::new(ProcessInner {
                cwd: String::from("/"),
                address_space: AddressSpace::from_elf(file),
                fd_table: vec![
                    Some(STDIN.clone()),
                    Some(STDOUT.clone()),
                    Some(STDOUT.clone()),
                ],
                parent: Arc::downgrade(&get_current_thread().process),
                child: Vec::new(),
                child_exited: Vec::new(),
                wake_callbacks: Vec::new(),
            }),
        })
    }

    /// fork 进程
    pub fn fork(&self, pid: usize) -> Arc<Self> {
        let mut process_inner = self.inner.lock();
        Arc::new(Self {
            pid,
            times: Default::default(),
            inner: SpinLock::new(ProcessInner {
                cwd: process_inner.cwd.clone(),
                address_space: process_inner.address_space.fork(),
                fd_table: process_inner.fd_table.clone(),
                parent: Arc::downgrade(&get_current_thread().process),
                child: Vec::new(),
                child_exited: Vec::new(),
                wake_callbacks: Vec::new(),
            }),
        })
    }

    /// 分配并映射线程的用户栈
    pub fn alloc_user_stack(&self) -> VA {
        let mut inner = self.inner.lock();
        let user_stack_top = inner
            .address_space
            .alloc_user_area(ConfigImpl::USER_STACK_SIZE);
        inner.address_space.insert_framed_area(
            user_stack_top - ConfigImpl::USER_STACK_SIZE..user_stack_top,
            PTEImpl::READABLE | PTEImpl::WRITABLE | PTEImpl::USER,
            None,
        );

        user_stack_top
    }

    #[inline]
    /// **UNSAFE**
    pub(super) unsafe fn dealloc_user_stack(&self, user_stack_top: VA) {
        self.inner.lock().address_space.remove_area(user_stack_top);
    }
}

impl ProcessInner {
    pub const MAX_FD: usize = 101;

    pub fn fd_alloc(&mut self) -> isize {
        let len = self.fd_table.len();
        for i in 2..self.fd_table.len() {
            if self.fd_table[i].is_none() {
                return i as isize;
            }
        }
        if len == Self::MAX_FD {
            return -1;
        }
        self.fd_table.push(None);
        len as isize
    }
}
