use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{mem::size_of, sync::atomic::Ordering};

use core_io::Read;
use fatfs::Inode;
use riscv::register::{sstatus, sstatus::SPP};
use xmas_elf::ElfFile;

use super::{flag::CloneFlags, *};
use crate::{
    arch::{
        RegisterImpl, TaskContextImpl, TrapFrameImpl, __switch, cpu,
        interface::{PageTable, Register, TaskContext, TrapFrame},
    },
    board::{interface::Config, ConfigImpl},
    fs::*,
    mm::*,
    schedule::SCHEDULE_THREAD,
    sync::SpinLock,
};

pub struct TidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl TidAllocator {
    pub const fn new() -> Self {
        TidAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> Tid {
        if let Some(tid) = self.recycled.pop() {
            Tid(tid)
        } else {
            self.current += 1;
            Tid(self.current - 1)
        }
    }

    pub fn dealloc(&mut self, tid: usize) {
        assert!(tid < self.current);
        assert!(
            self.recycled.iter().find(|ptid| **ptid == tid).is_none(),
            "tid {} has been deallocated!",
            tid
        );
        self.recycled.push(tid);
    }
}

/// 用于分配 tid
pub(super) static TID_ALLOCATOR: SpinLock<TidAllocator> = SpinLock::new(TidAllocator::new());

#[derive(Debug)]
pub struct Tid(usize);
impl Drop for Tid {
    fn drop(&mut self) {
        TID_ALLOCATOR.lock().dealloc(self.0);
    }
}

pub struct Thread {
    /// 线程 ID
    pub tid: Tid,
    /// 所属的进程
    pub process: Arc<Process>,
    /// 用户栈顶
    pub user_stack_top: VA,
    /// 当线程处于 Ready 状态时，task_cx 指向保存在内核栈中的 TaskContextImpl；
    pub task_cx: &'static TaskContextImpl,
    /// 用 `SpinLock` 包装一些可变的变量
    pub inner: SpinLock<ThreadInner>,
    #[cfg(debug_assertions)]
    pub magic: usize,
}

/// 线程中需要同步访问的部分
pub struct ThreadInner {
    /// 线程状态
    pub status: ThreadStatus,
    /// 线程进入/离开内核的时刻
    pub cycles: u64,
    /// see clone(2) set_tid_address(2)
    pub set_child_tid: usize,
    /// 如果不为 0，则该线程退出时会将该指针指向的值置 0，并唤醒该指针上的
    /// futex。 该指针可被 set_tid_address(2) system call 修改。
    /// see also clone(2), futex(2), gettid(2)
    pub clear_child_tid: usize,
}

#[allow(unused)]
#[derive(Clone, Copy)]
pub enum ThreadStatus {
    Ready,
    Running,
    Waiting,
    Zombie,
}

pub fn get_kernel_stack_range(tid: usize) -> VARange {
    let kernel_stack_top = ConfigImpl::KERNEL_STACK_TOP - tid * ConfigImpl::KERNEL_STACK_ALIGN_SIZE;
    VA(kernel_stack_top - ConfigImpl::KERNEL_STACK_SIZE)..VA(kernel_stack_top)
}

#[cfg(test)]
mod tests {
    use test_macros::kernel_test;

    use super::*;
    #[kernel_test]
    fn test_get_kernel_stack_range() {
        println!(
            "test_get_kernel_stack_range 0 = {}",
            VARangeOrd(get_kernel_stack_range(0))
        );
        println!(
            "test_get_kernel_stack_range 1 = {}",
            VARangeOrd(get_kernel_stack_range(1))
        );
        println!(
            "test_get_kernel_stack_range 2 = {}",
            VARangeOrd(get_kernel_stack_range(2))
        );
        println!(
            "test_get_kernel_stack_range 3 = {}",
            VARangeOrd(get_kernel_stack_range(3))
        );
    }
}

/// 线程指针大小
const THREAD_PTR_OFFSET: usize = size_of::<usize>();
/// TrapFrame 在内核栈的偏移量
pub(super) const TRAP_FRAME_OFFSET: usize = THREAD_PTR_OFFSET + size_of::<TrapFrameImpl>();

/// 获取当前线程的内核栈顶
pub fn get_cur_kernel_stack_top() -> VA {
    // XXX 可能的问题：sp 刚好在栈底，得到 guard page 里的内容，发生 page fault
    VA(round_up!(
        RegisterImpl::sp(),
        ConfigImpl::KERNEL_STACK_ALIGN_SIZE
    ))
}
/// 获取当前线程的可变引用
pub fn get_current_thread() -> &'static mut Thread {
    let thread_ptr = *(get_cur_kernel_stack_top() - THREAD_PTR_OFFSET).get_mut::<usize>();
    unsafe {
        debug_assert_eq!((&*(thread_ptr as *mut Thread)).magic, Thread::MAGIC);
        &mut *(thread_ptr as *mut Thread)
    }
}
/// 获取当前线程的 TrapFrame 的可变引用
pub fn get_current_trapframe() -> &'static mut TrapFrameImpl {
    (get_cur_kernel_stack_top() - TRAP_FRAME_OFFSET).get_mut()
}

/// 对 [`xmas_elf::ElfFile`] 的扩展。
/// 创建时会读取 header 和 program headers 数据，不包含 section headers。
pub struct ElfFileExt<'a> {
    pub elf: ElfFile<'a>,
    pub file: Box<dyn Inode + Send + Sync>,
    pub frame: FrameTracker,
}

impl<'a> ElfFileExt<'a> {
    const BLK_SIZE: usize = 512;

    pub fn new(file_name: &str) -> Self {
        let frame = frame_alloc().unwrap();
        let data = VPN::from(frame.ppn).get_array::<u8>();
        let mut file = Box::new(
            FILE_SYSTEM_TABLE[0]
                .1
                .as_ref()
                .unwrap()
                .open_file(file_name)
                .unwrap(),
        );
        file.read_exact(&mut data[..Self::BLK_SIZE]).unwrap();
        let elf = ElfFile::new(data).unwrap();
        let pt2 = elf.header.pt2;
        let ph_entry_end =
            pt2.ph_offset() as usize + (pt2.ph_count() * pt2.ph_entry_size()) as usize;
        if ph_entry_end > Self::BLK_SIZE {
            file.read_exact(
                &mut VPN::from(frame.ppn).get_array::<u8>()[Self::BLK_SIZE..ph_entry_end],
            )
            .unwrap();
        }

        Self { elf, file, frame }
    }
}

impl Thread {
    #[cfg(debug_assertions)]
    pub const MAGIC: usize = 0x5834_3845_2383_3485;

    /// 创建内核线程
    pub fn new_kernel(entry: usize) -> Arc<Thread> {
        let tid = TID_ALLOCATOR.lock().alloc();

        // 分配内核栈
        let kernel_stack_range = get_kernel_stack_range(tid.0);
        KERNEL_PROCESS
            .inner
            .lock()
            .address_space
            .insert_kernel_stack_area(kernel_stack_range.clone());
        // TaskContext
        let task_cx = (kernel_stack_range.end - TRAP_FRAME_OFFSET).get_mut::<TaskContextImpl>();
        task_cx.set_ra(entry);

        let new_thread = Arc::new(Self {
            tid,
            process: KERNEL_PROCESS.clone(),
            user_stack_top: VA(0), // 内核线程的用户栈顶为 0，表示没有用户栈
            task_cx,
            inner: SpinLock::new(ThreadInner {
                status: ThreadStatus::Ready,
                cycles: 0,
                set_child_tid: 0,
                clear_child_tid: 0,
            }),
            #[cfg(debug_assertions)]
            magic: Self::MAGIC,
        });
        *(kernel_stack_range.end - THREAD_PTR_OFFSET).get_mut::<usize>() =
            Arc::<Thread>::as_ptr(&new_thread) as usize;

        new_thread
    }

    /// 创建用户进程
    pub fn new_thread(file_name: &str, arguments: &[&str]) -> Arc<Thread> {
        print!("new_thread({}) args:", file_name);
        for &arg in arguments {
            print!(" {}", arg);
        }
        println!();

        let tid = TID_ALLOCATOR.lock().alloc();

        // 读取 elf 文件内容
        let mut elf_file = ElfFileExt::new(file_name);
        // 创建进程
        let process = Process::from_elf(&mut elf_file, tid.0);
        // 分配内核栈
        let kernel_stack_range = get_kernel_stack_range(tid.0);
        // println!("kernel_stack_range {}", VARangeOrd(kernel_stack_range.clone()));
        KERNEL_PROCESS
            .inner
            .lock()
            .address_space
            .insert_kernel_stack_area(kernel_stack_range.clone());
        // 分配用户栈
        let (user_stack_top, task_cx) = process.alloc_user_stack(
            arguments,
            &[
                "SHELL=/bin/bash",
                "PWD=/",
                "HOME=/",
                "LANG=C.UTF-8",
                "USER=root",
                "PATH=/",
                "OLDPWD=/root",
            ],
            &elf_file.elf,
            kernel_stack_range.end,
        );

        let new_thread = Arc::new(Self {
            tid,
            process,
            user_stack_top,
            task_cx,
            inner: SpinLock::new(ThreadInner {
                status: ThreadStatus::Ready,
                cycles: 0,
                set_child_tid: 0,
                clear_child_tid: 0,
            }),
            #[cfg(debug_assertions)]
            magic: Self::MAGIC,
        });

        *(kernel_stack_range.end - THREAD_PTR_OFFSET).get_mut::<usize>() =
            Arc::<Thread>::as_ptr(&new_thread) as usize;

        new_thread
    }

    /// TODO 还需要 kill 掉其他的线程
    pub fn execve(&mut self, pathname: &str, argv: &[&str], envp: &[&str]) {
        print!("execve({}) args:", pathname);
        for &arg in argv {
            print!(" {}", arg);
        }
        println!();

        let mut elf_file = ElfFileExt::new(pathname);
        self.process.execve(&mut elf_file);
        // 1. 新的用户栈
        let (user_stack_top, task_cx) = self.process.alloc_user_stack(
            argv,
            envp,
            &elf_file.elf,
            get_kernel_stack_range(self.tid.0).end,
        );
        self.user_stack_top = user_stack_top;
        self.task_cx = task_cx;
    }

    /// fork 用户进程
    pub fn fork(&self, flags: CloneFlags, child_tid: *mut usize) -> Arc<Thread> {
        let tid = TID_ALLOCATOR.lock().alloc();
        let process = self.process.fork(tid.0);
        get_current_thread()
            .process
            .inner
            .lock()
            .child
            .push(Arc::downgrade(&process));
        // 内核栈
        let kernel_stack_range = get_kernel_stack_range(tid.0);
        KERNEL_PROCESS
            .inner
            .lock()
            .address_space
            .insert_kernel_stack_area(kernel_stack_range.clone());
        // TrapFrame
        let trap_frame = (kernel_stack_range.end - TRAP_FRAME_OFFSET).get_mut::<TrapFrameImpl>();
        *trap_frame = *get_current_trapframe();
        trap_frame.set_return_value(0);
        // TaskContext
        let task_cx =
            VA(trap_frame as *const TrapFrameImpl as usize - size_of::<TaskContextImpl>())
                .get_mut::<TaskContextImpl>();
        task_cx.set_ra(crate::arch::__restore as usize);

        // println!("fork flags = {:?}", flags);
        let mut set_child_tid = 0usize;
        let mut clear_child_tid = 0usize;
        if flags.contains(CloneFlags::CLONE_CHILD_CLEARTID) {
            clear_child_tid = child_tid as usize;
        }
        if flags.contains(CloneFlags::CLONE_CHILD_SETTID) {
            set_child_tid = child_tid as usize;
            // XXX 1. 因为是高地址访存错误应该不会发生
            // 2. 子进程和父进程此时还是共享物理页面，这样会影响到父进程？
            let va: VA = process
                .inner
                .lock()
                .address_space
                .page_table
                .translate_va(VA(child_tid as usize))
                .unwrap()
                .into();
            va.write(tid.0);
        }
        if flags.contains(CloneFlags::CLONE_THREAD) {
            unimplemented!()
        }

        let new_thread = Arc::new(Self {
            tid,
            process,
            user_stack_top: get_current_thread().user_stack_top,
            task_cx,
            inner: SpinLock::new(ThreadInner {
                status: ThreadStatus::Ready,
                cycles: 0,
                set_child_tid,
                clear_child_tid,
            }),
            #[cfg(debug_assertions)]
            magic: Self::MAGIC,
        });

        *(kernel_stack_range.end - THREAD_PTR_OFFSET).get_mut::<usize>() =
            Arc::<Thread>::as_ptr(&new_thread) as usize;

        new_thread
    }

    /// 切换到另一个线程
    pub fn switch_to(&mut self, other: &Thread) {
        self.inner.critical_section(|inner| {
            let cur_cycles = cpu::get_cycles();
            self.process
                .times
                .tms_stime
                .fetch_add(cur_cycles - inner.cycles, Ordering::SeqCst);
            // 线程从内核控制路径离开时的时刻
            inner.cycles = cur_cycles;
        });

        info!("thread {:?} is ready", other.tid);
        other.activate();
        unsafe {
            debug_assert_eq!(sstatus::read().spp(), SPP::User);
            __switch(core::mem::transmute(&mut self.task_cx), other.task_cx);
        }

        self.inner.critical_section(|inner| {
            // 线程进入内核控制路径的时刻
            inner.cycles = cpu::get_cycles();
        });
    }

    pub fn yield_to_sched(&mut self) {
        self.switch_to(unsafe { &*SCHEDULE_THREAD });
    }

    /// 激活线程页表
    pub fn activate(&self) {
        self.process
            .inner
            .lock()
            .address_space
            .page_table
            .activate();
    }

    /// 获取线程的 TrapFrame
    pub fn get_trapframe(&self) -> &mut TrapFrameImpl {
        let kernel_stack_top =
            VA(ConfigImpl::KERNEL_STACK_TOP - self.tid.0 * ConfigImpl::KERNEL_STACK_ALIGN_SIZE);
        (kernel_stack_top - TRAP_FRAME_OFFSET).get_mut()
    }

    pub fn get_tid(&self) -> usize {
        self.tid.0
    }

    /// 使用此 unsafe 函数时，需满足以下几点：
    /// 1. 只有用户线程才能调用此函数
    /// 2. 确保该用户线程不会回到用户态
    #[inline]
    pub unsafe fn dealloc_user_stack(&mut self) {
        self.process.dealloc_user_stack(self.user_stack_top);
    }
}

impl Eq for Thread {}
impl PartialEq for Thread {
    fn eq(&self, other: &Self) -> bool {
        self.tid.0 == other.tid.0
    }
}

/// 回收内核栈
impl Drop for Thread {
    fn drop(&mut self) {
        debug!("线程 {:?} drop", self.tid);
        // TODO 暂时不移除，留给下一个线程用？
        KERNEL_PROCESS
            .inner
            .lock()
            .address_space
            .remove_area(get_kernel_stack_range(self.tid.0).end);
    }
}
