use alloc::{sync::Arc, vec::Vec};
use core::{mem::size_of, sync::atomic::Ordering};

use core_io::Read;
use xmas_elf::ElfFile;

use super::*;
use crate::{
    arch::{
        PTEImpl, RegisterImpl, TaskContextImpl, TrapFrameImpl, __switch,
        cpu::{self, get_cpu_id},
        interface::{PageTable, Register, TaskContext, TrapFrame},
    },
    board::{interface::Config, sbss_with_stack, ConfigImpl},
    fs::*,
    mm::*,
    schedule::get_sched_cx,
    sync::SpinLock,
};

pub struct TidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl TidAllocator {
    pub const fn new() -> Self {
        TidAllocator {
            current: 2, // XXX 0 和 1 被调度线程使用
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
}

/// 线程中需要同步访问的部分
pub struct ThreadInner {
    /// 线程状态
    pub status: ThreadStatus,
    /// 线程进入/离开内核的时刻
    pub cycles: u64,
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

/// 线程指针大小
const THREAD_PTR_OFFSET: usize = size_of::<usize>();
/// TrapFrame 在内核栈的偏移量
const TRAP_FRAME_OFFSET: usize = THREAD_PTR_OFFSET + size_of::<TrapFrameImpl>();

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
    unsafe { &mut *(thread_ptr as *mut Thread) }
}
/// 获取当前线程的 TrapFrame 的可变引用
pub fn get_current_trapframe() -> &'static mut TrapFrameImpl {
    (get_cur_kernel_stack_top() - TRAP_FRAME_OFFSET).get_mut()
}

impl Thread {
    /// 初始化当前 CPU 的调度线程
    pub fn init_sched_thread(entry: usize) -> Arc<Thread> {
        // 1. 先映射内核栈（即 boot_stack）
        let tid = get_cpu_id();
        let kernel_stack_top = get_kernel_stack_range(tid).end;
        let vpn = kernel_stack_top.floor() - 1; // 内核栈顶所在虚拟页面
        let ppn: PPN = PPN::from(VA(sbss_with_stack as usize).floor()) + tid; // 物理页面
        KERNEL_PROCESS
            .inner
            .lock()
            .address_space
            .page_table
            .map_one(vpn, ppn, PTEImpl::READABLE | PTEImpl::WRITABLE);

        // 2. 设置 TaskContext
        let task_cx = (kernel_stack_top - THREAD_PTR_OFFSET - size_of::<TaskContextImpl>())
            .get_mut::<TaskContextImpl>();
        task_cx.set_ra(entry);

        let new_thread = Arc::new(Self {
            tid: Tid(tid),
            process: KERNEL_PROCESS.clone(),
            user_stack_top: VA(0), // 内核线程的用户栈顶为 0，表示没有用户栈
            task_cx,
            inner: SpinLock::new(ThreadInner {
                status: ThreadStatus::Running,
                cycles: 0,
            }),
        });
        *(kernel_stack_top - THREAD_PTR_OFFSET).get_mut::<usize>() =
            Arc::<Thread>::as_ptr(&new_thread) as usize;

        new_thread
    }

    /// 创建内核线程
    pub fn new_kernel(entry: usize, _args: Option<&[usize]>) -> Arc<Thread> {
        let tid = TID_ALLOCATOR.lock().alloc();

        // 分配内核栈
        let kernel_stack_range = get_kernel_stack_range(tid.0);
        KERNEL_PROCESS
            .inner
            .lock()
            .address_space
            .insert_framed_area(
                kernel_stack_range.clone(),
                PTEImpl::READABLE | PTEImpl::WRITABLE,
                None,
            );
        // TrapFrame
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
            }),
        });
        *(kernel_stack_range.end - THREAD_PTR_OFFSET).get_mut::<usize>() =
            Arc::<Thread>::as_ptr(&new_thread) as usize;

        new_thread
    }

    /// 创建用户进程
    pub fn new_thread(file_name: &str, args: Option<&[usize]>) -> Arc<Thread> {
        println!("new_thread {}", file_name);
        let tid = TID_ALLOCATOR.lock().alloc();

        // 读取 elf 文件内容
        let mut app = FILE_SYSTEM_TABLE[0]
            .1
            .as_ref()
            .unwrap()
            .open_file(file_name)
            .unwrap();
        let mut data: Vec<u8> = Vec::new();
        app.read_to_end(&mut data).unwrap();
        let elf = ElfFile::new(data.as_slice()).unwrap();
        // 创建进程
        let process = Process::from_elf(&elf, tid.0);
        // 分配用户栈
        let user_stack_top = process.alloc_user_stack();
        // 分配内核栈
        let kernel_stack_range = get_kernel_stack_range(tid.0);
        // println!("内核栈 {:#x?}", kernel_stack_range);
        KERNEL_PROCESS
            .inner
            .lock()
            .address_space
            .insert_framed_area(
                kernel_stack_range.clone(),
                PTEImpl::READABLE | PTEImpl::WRITABLE,
                None,
            );
        // TrapFrame
        let cx = (kernel_stack_range.end - TRAP_FRAME_OFFSET).get_mut::<TrapFrameImpl>();
        cx.init(
            user_stack_top.0 - size_of::<usize>(),
            elf.header.pt2.entry_point() as usize,
            args,
            true,
        );
        // TaskContextImpl
        let task_cx = VA(cx as *const TrapFrameImpl as usize - size_of::<TaskContextImpl>())
            .get_mut::<TaskContextImpl>();
        task_cx.set_ra(crate::arch::ret_to_restore as usize);
        // println!("task_cx {:#p}", task_cx);

        let new_thread = Arc::new(Self {
            tid,
            process,
            user_stack_top,
            task_cx,
            inner: SpinLock::new(ThreadInner {
                status: ThreadStatus::Ready,
                cycles: 0,
            }),
        });

        *(kernel_stack_range.end - THREAD_PTR_OFFSET).get_mut::<usize>() =
            Arc::<Thread>::as_ptr(&new_thread) as usize;

        new_thread
    }

    /// fork 用户进程
    pub fn fork(&self) -> Arc<Thread> {
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
            .insert_framed_area(
                kernel_stack_range.clone(),
                PTEImpl::READABLE | PTEImpl::WRITABLE,
                None,
            );
        // TrapFrame
        let trap_frame = (kernel_stack_range.end - TRAP_FRAME_OFFSET).get_mut::<TrapFrameImpl>();
        *trap_frame = *get_current_trapframe();
        trap_frame.set_return_value(0);
        // TaskContext
        let task_cx =
            VA(trap_frame as *const TrapFrameImpl as usize - size_of::<TaskContextImpl>())
                .get_mut::<TaskContextImpl>();
        task_cx.set_ra(crate::arch::__restore as usize);

        let new_thread = Arc::new(Self {
            tid,
            process,
            user_stack_top: get_current_thread().user_stack_top,
            task_cx,
            inner: SpinLock::new(ThreadInner {
                status: ThreadStatus::Ready,
                cycles: 0,
            }),
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

        other.activate();
        unsafe {
            __switch(core::mem::transmute(&mut self.task_cx), other.task_cx);
        }

        self.inner.critical_section(|inner| {
            // 线程进入内核控制路径的时刻
            inner.cycles = cpu::get_cycles();
        });
    }

    pub fn yield_to_sched(&mut self) {
        self.inner.critical_section(|inner| {
            let cur_cycles = cpu::get_cycles();
            self.process
                .times
                .tms_stime
                .fetch_add(cur_cycles - inner.cycles, Ordering::SeqCst);
            // 线程从内核控制路径离开时的时刻
            inner.cycles = cur_cycles;
        });
        unsafe {
            __switch(core::mem::transmute(&mut self.task_cx), *get_sched_cx());
        }
        self.inner.critical_section(|inner| {
            // 线程进入内核控制路径的时刻
            inner.cycles = cpu::get_cycles();
        });
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
