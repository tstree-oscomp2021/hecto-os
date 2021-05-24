use alloc::{boxed::Box, vec::Vec};

use algorithm::Scheduler;
use core_io::Read;
use xmas_elf::ElfFile;

use super::*;
use crate::{
    arch::{TaskContextImpl, __switch},
    fs::ROOT_DIR,
    process::*,
    processor::get_sched_cx,
    trap::interface::TrapFrame,
    MemorySet,
};

/// 线程退出
/// 如果是进程中的最后一个线程，进程也退出，向父进程发送消息
pub(super) fn sys_exit(_status: isize) -> ! {
    unsafe {
        // 1. unmap 当前线程的用户栈
        let mut cur_thread = Arc::from_raw(get_current_thread());
        cur_thread.inner.lock().status = ThreadStatus::Zombie;
        SCHEDULER.lock(|s| s.remove_thread(&cur_thread));
        Arc::get_mut_unchecked(&mut cur_thread).dealloc_user_stack();
        // 2. 通知父进程
        let process_inner = cur_thread.process.inner.lock();
        if let Some(parent) = process_inner.parent.upgrade() {
            parent.inner.lock().child_exited.push((
                cur_thread.process.pid,
                Arc::downgrade(&cur_thread.process).clone(),
            ));
            if let Some(wake_parent) = parent.inner.lock().wake_callbacks.pop() {
                wake_parent();
            }
        };
        drop(process_inner);
        // 因为这是个从裸指针构造的 Arc。但 __switch 不会返回，所以没必要防止析构
        // core::mem::forget(cur_thread);
        // 3. 切换到调度线程
        let mut cur_task_cx: *const TaskContextImpl = core::mem::transmute(1usize);
        __switch(&mut cur_task_cx, *get_sched_cx());
    }
    unreachable!()
}

pub(super) fn sys_clone(
    _flags: u64,
    stack: *mut usize,
    _parent_tid: *mut usize,
    _tls: usize,
    _child_tid: *mut usize,
) -> isize {
    let new_thread = get_current_thread().fork();
    let trap_frame = new_thread.get_trapframe();
    if stack as usize != 0 {
        trap_frame.set_sp(stack as usize);
        trap_frame.set_entry_point(unsafe { *stack.offset(0) });
    }

    SCHEDULER.lock(|s| s.add_thread(new_thread.clone()));
    // 让新的线程先一步调度
    get_current_thread().inner.lock().status = ThreadStatus::Ready;
    get_current_thread().switch_to(&new_thread);

    new_thread.get_tid() as isize
}

/// 设置进程的 EventHandler（一个闭包，用来唤醒线程），
/// 从调度器中移除当前线程放进闭包， 然后 __switch 到其他线程，
/// 子进程通过调用父进程的这个闭包来将父进程的这个线程加进调度器
/// 不一定非要从调度器中移除，设置 status 即可，调度器调度时判断 status
/// 在 *wstatus 里存储状态信息
pub(super) fn sys_wait4(
    pid: isize,
    _wstatus: *mut isize,
    _options: isize,
    _rusage: *mut (),
) -> isize {
    if pid != -1 {
        todo!();
    }
    debug!("父进程 sys_wait4");
    loop {
        let cur_thread = get_current_thread();
        let mut process_inner = cur_thread.process.inner.lock();
        if let Some((pid, child)) = process_inner.child_exited.pop() {
            process_inner.child.retain(|c| !c.ptr_eq(&child));
            return pid as isize;
        }

        let parent_thread: &mut Thread =
            unsafe { core::mem::transmute(cur_thread as *const Thread as usize) };
        process_inner.wake_callbacks.push(Box::new(move || {
            debug!("唤醒父进程");
            parent_thread.inner.lock().status = ThreadStatus::Ready;
        }));
        drop(process_inner);

        debug!("父进程睡眠");
        cur_thread.inner.lock().status = ThreadStatus::Waiting;
        cur_thread.yield_to_sched();
    }
}

pub(super) fn sys_execve(
    pathname: *const u8,
    _argv: *const *const u8,
    _envp: *const *const u8,
) -> isize {
    let cur_thread = get_current_thread();
    // 读取 elf 文件内容
    let mut app = ROOT_DIR
        .open_file(super::fs::normalize_path(super::fs::AT_FDCWD, pathname).as_str())
        .unwrap();
    let mut data: Vec<u8> = Vec::new();
    app.read_to_end(&mut data).unwrap();
    let elf = ElfFile::new(data.as_slice()).unwrap();
    cur_thread.process.inner.lock().memory_set = MemorySet::from_elf(&elf);
    // 重新分配用户栈
    // unsafe {
    //     cur_thread.dealloc_user_stack();
    // }
    cur_thread.user_stack_top = cur_thread.process.alloc_user_stack();
    // 设置 TrapFrame
    let trap_frame = get_current_trapframe();
    trap_frame.set_sp(cur_thread.user_stack_top.0 - 8);
    trap_frame.set_entry_point(elf.header.pt2.entry_point() as usize);
    // 激活新页表
    cur_thread.activate();

    0
}

pub(super) fn sys_getpid() -> isize {
    get_current_thread().process.pid as isize
}

pub(super) fn sys_getppid() -> isize {
    if let Some(parent) = get_current_thread().process.inner.lock().parent.upgrade() {
        let ppid = parent.pid as isize;
        // XXX 目前，父进程可能是 pid 为 0 的内核线程
        if ppid == 0 {
            1
        } else {
            ppid
        }
    } else {
        -1
    }
}
