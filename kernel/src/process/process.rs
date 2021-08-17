use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use core::{mem::size_of, sync::atomic::AtomicU64};

use interface::PageTable;
use lazy_static::lazy_static;
use xmas_elf::{program::Type, ElfFile};

use super::interface::TaskContext;
use crate::{
    arch::{interface::TrapFrame, PTEImpl, TaskContextImpl, TrapFrameImpl},
    board::{interface::Config, ConfigImpl},
    fs::{FileDescriptor, STDIN, STDOUT},
    get_current_thread,
    mm::*,
    sync::SpinLock,
    thread::TRAP_FRAME_OFFSET,
    ElfFileExt,
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

/// auxiliary table.
/// <https://github.com/torvalds/linux/blob/masterinclude/uapi/linux/auxvec.h>
/// <https://github.com/torvalds/linux/blob/master/arch/ia64/include/uapi/asm/auxvec.h>
#[allow(non_camel_case_types, dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(usize)]
enum AUXV {
    /// end of vector
    AT_NULL = 0,
    /// entry should be ignored
    AT_IGNORE = 1,
    /// file descriptor of program
    AT_EXECFD = 2,
    /// program headers for program
    AT_PHDR = 3,
    /// size of program header entry
    AT_PHENT = 4,
    /// number of program headers
    AT_PHNUM = 5,
    /// system page size
    AT_PAGESZ = 6,
    /// base address of interpreter
    AT_BASE = 7,
    /// flags
    AT_FLAGS = 8,
    /// entry point of program
    AT_ENTRY = 9,
    /// program is not ELF
    AT_NOTELF = 10,
    /// real uid
    AT_UID = 11,
    /// effective uid
    AT_EUID = 12,
    /// real gid
    AT_GID = 13,
    /// effective gid
    AT_EGID = 14,
    /// string identifying CPU for optimizations
    AT_PLATFORM = 15,
    /// arch dependent hints at CPU capabilities
    AT_HWCAP = 16,
    /// frequency at which times() increments
    AT_CLKTCK = 17,
    /* AT_* values 18 through 22 are reserved */
    ///secure mode boolean
    AT_SECURE = 23,
    /// string identifying real platform, may differ from AT_PLATFORM.
    AT_BASE_PLATFORM = 24,
    /// address of 16 random bytes
    AT_RANDOM = 25,
    /// extension of AT_HWCAP
    AT_HWCAP2 = 26,

    /// filename of program
    AT_EXECFN = 31,
}

impl Process {
    /// 通过 ELF 文件创建用户进程
    pub fn from_elf(file: &mut ElfFileExt, pid: usize) -> Arc<Self> {
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

    /// - 地址空间替换（程序、用户栈、mmap）
    /// - 将带 close-on-exec 的 fd 关闭
    pub fn execve(&self, file: &mut ElfFileExt) {
        // 1. 新的地址空间
        let mut inner = self.inner.lock();
        inner.address_space = AddressSpace::from_elf(file);
        // XXX
        inner.address_space.page_table.activate();

        // TODO 2. 如果 fd 带 close-on-exec flag，就将其关闭
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

    /// 分配、映射并初始化线程的用户栈
    pub fn alloc_user_stack<'a, 'b>(
        &'a self,
        arguments: &[&str],
        environments: &[&str],
        elf: &ElfFile,
        kernel_stack_end: VA,
    ) -> (VA, &'b TaskContextImpl) {
        let mut inner = self.inner.lock();
        let user_stack_top = inner
            .address_space
            .alloc_user_area(ConfigImpl::USER_STACK_SIZE);
        inner.address_space.insert_framed_area(
            user_stack_top - ConfigImpl::USER_STACK_SIZE..user_stack_top,
            PTEImpl::READABLE | PTEImpl::WRITABLE | PTEImpl::USER,
            None,
        );
        let user_sp_offset = VA::from(
            inner
                .address_space
                .page_table
                .translate_va(user_stack_top - 8)
                .unwrap()
                + 8,
        ) - user_stack_top;
        let mut user_sp = user_stack_top - 8;
        core::mem::drop(inner);

        // environment ASCIIZ str
        // XXX 这里如果用 environments.len() + 1，会导致 alloc 一个连续的 1M 内存区域出错
        // TODO 过大的区域别用 slab 分配器，而是使用不连续的页面
        let mut envp: Vec<usize> = Vec::with_capacity(environments.len());
        for i in 0..environments.len() {
            user_sp -= environments[i].len() + 1;
            envp.push(user_sp.into());
            let mut p = user_sp;
            // 复制字符串
            for &c in environments[i].as_bytes() {
                (p + user_sp_offset).write(c);
                p += 1;
            }
            (p + user_sp_offset).write(b'\0'); // 字符串末尾
        }
        envp.push(0);

        // argument ASCIIZ strings
        let mut argv: Vec<usize> = Vec::with_capacity(arguments.len() + 1);
        for i in 0..arguments.len() {
            user_sp -= arguments[i].len() + 1;
            argv.push(user_sp.into());
            let mut p = user_sp;
            // 复制字符串
            for &c in arguments[i].as_bytes() {
                (p + user_sp_offset).write(c);
                p += 1;
            }
            (p + user_sp_offset).write(b'\0'); // 字符串末尾
        }
        argv.push(0);

        // auvx
        // https://lwn.net/Articles/519085/
        // https://lwn.net/Articles/631631/
        // http://articles.manugarg.com/aboutelfauxiliaryvectors.html
        // https://www.cnblogs.com/likaiming/p/11193697.html
        let mut ph_head_addr = elf.header.pt2.ph_offset(); // program headers 在文件中的偏移
        for ph in elf.program_iter() {
            if ph.get_type() == Ok(Type::Load) && ph.offset() <= ph_head_addr {
                ph_head_addr += ph.virtual_addr() - ph.offset(); // 得到 program headers 的虚拟地址
                break;
            }
        }
        let auvx = [
            (AUXV::AT_PHDR, ph_head_addr as usize),
            (AUXV::AT_PHENT, elf.header.pt2.ph_entry_size() as usize),
            (AUXV::AT_PHNUM, elf.header.pt2.ph_count() as usize),
            (AUXV::AT_PAGESZ, 4096),
            (AUXV::AT_BASE, 0),
            (AUXV::AT_ENTRY, elf.header.pt2.entry_point() as usize),
            (AUXV::AT_NULL, 0),
        ];

        // padding
        user_sp -= user_sp.0 % 16;

        // auxv[] (Elf64_auxv_t)
        user_sp -= auvx.len() * size_of::<(usize, usize)>();
        let auxv_base = user_sp;
        for i in 0..auvx.len() {
            // println!("auvx[{}] =({:?}, {:#x})", i, auvx[i].0, auvx[i].1);
            (auxv_base + user_sp_offset + size_of::<(usize, usize)>() * i).write(auvx[i]);
        }

        // envp[] (pointer)
        user_sp -= envp.len() * size_of::<usize>();
        let envp_base = user_sp;
        for i in 0..envp.len() {
            (envp_base + user_sp_offset + size_of::<usize>() * i).write(envp[i]);
        }

        // argv[] (pointer)
        user_sp -= argv.len() * size_of::<usize>();
        let argv_base = user_sp;
        for i in 0..argv.len() {
            (argv_base + user_sp_offset + size_of::<usize>() * i).write(argv[i]);
        }

        // argc = number of args
        user_sp -= size_of::<usize>();
        (user_sp + user_sp_offset).write(arguments.len());

        // println!(
        //     "user_stack_top, user_sp, argv_base, envp_base, auxv_base {} {} {} {} {}",
        //     user_stack_top, user_sp, argv_base, envp_base, auxv_base
        // );

        // TrapFrame
        let cx = (kernel_stack_end - TRAP_FRAME_OFFSET).get_mut::<TrapFrameImpl>();
        cx.init(
            user_sp.0,
            elf.header.pt2.entry_point() as usize,
            Some(&[arguments.len(), argv_base.0, envp_base.0, auxv_base.0]),
            true,
        );
        // TaskContextImpl
        let task_cx = VA(cx as *const TrapFrameImpl as usize - size_of::<TaskContextImpl>())
            .get_mut::<TaskContextImpl>();
        task_cx.set_ra(crate::arch::ret_to_restore as usize);
        // println!("task_cx {:#p}", task_cx);

        (user_stack_top, task_cx)
    }

    #[inline]
    /// **UNSAFE**
    pub(super) unsafe fn dealloc_user_stack(&self, user_stack_top: VA) {
        self.inner.lock().address_space.remove_area(user_stack_top);
    }
}

impl ProcessInner {
    pub const MAX_FD: usize = 128;

    pub fn fd_alloc(&mut self) -> isize {
        let len = self.fd_table.len();
        for i in 3..len {
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

    pub fn fd_alloc_from(&mut self, min: usize) -> isize {
        let len = self.fd_table.len();
        if min >= Self::MAX_FD {
            return -1;
        }
        if min >= len {
            self.fd_table.resize(min + 1, None);
            return min as isize;
        }
        for i in min..len {
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

    pub fn get_fd(&self, idx: usize) -> Option<&mut Option<alloc::sync::Arc<FileDescriptor>>> {
        unsafe { &mut *(self as *const _ as *mut Self) }
            .fd_table
            .get_mut(idx)
    }
}
