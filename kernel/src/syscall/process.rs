use algorithm::Scheduler;
use riscv::register::sstatus::{self, SPP};

use super::*;
use crate::{
    process::interface::TaskContext, schedule::SCHEDULE_THREAD, trap::interface::TrapFrame,
};

/// 线程退出
/// 如果是进程中的最后一个线程，进程也退出，向父进程发送消息
pub(super) fn sys_exit(status: i32) -> ! {
    unsafe {
        // 1. unmap 当前线程的用户栈
        let mut cur_thread = Arc::from_raw(get_current_thread());
        cur_thread.inner.lock().status = ThreadStatus::Zombie;
        SCHEDULER.critical_section(|s| s.remove_thread(&cur_thread));
        Arc::get_mut_unchecked(&mut cur_thread).dealloc_user_stack();
        // 2. 通知父进程
        let process_inner = cur_thread.process.inner.lock();
        if let Some(parent) = process_inner.parent.upgrade() {
            parent.times.tms_cutime.fetch_add(
                cur_thread.process.times.tms_utime.load(Ordering::Acquire),
                Ordering::SeqCst,
            );
            parent.times.tms_cstime.fetch_add(
                cur_thread.process.times.tms_stime.load(Ordering::Acquire),
                Ordering::SeqCst,
            );

            parent.inner.lock().child_exited.push((
                cur_thread.process.pid,
                Arc::downgrade(&cur_thread.process).clone(),
                status & 0xFF,
            ));
            if let Some(wake_parent) = parent.inner.lock().wake_callbacks.pop() {
                wake_parent();
            }
        };
        drop(process_inner);
        // 因为这是个从裸指针构造的 Arc。但 __switch 不会返回，所以没必要防止析构
        // core::mem::forget(cur_thread);
        // 3. 切换到调度线程
        sys_sched_yield();
    }
    unreachable!()
}

pub(super) fn sys_clone(
    flags: CloneFlags,
    stack: *mut usize,
    _parent_tid: *mut usize,
    _tls: usize,
    child_tid: *mut usize,
) -> isize {
    let new_thread = get_current_thread().fork(flags, child_tid);
    let trap_frame = new_thread.get_trapframe();
    if stack as usize != 0 {
        trap_frame
            .set_sp(stack as usize)
            .set_entry_point(unsafe { *stack.offset(0) });
    }

    SCHEDULER.critical_section(|s| s.add_thread(new_thread.clone()));
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
pub(super) fn sys_wait4(pid: isize, wstatus: *mut i32, _options: isize, _rusage: *mut ()) -> isize {
    debug!("父进程 sys_wait4");
    loop {
        let cur_thread = get_current_thread();
        let mut process_inner = cur_thread.process.inner.lock();
        let result = if pid == -1 {
            if let Some((cpid, child, exit_status)) = process_inner.child_exited.pop() {
                process_inner.child.retain(|c| !c.ptr_eq(&child));
                Some((cpid as isize, exit_status))
            } else {
                None
            }
        } else {
            if let Some((_, child, exit_status)) = process_inner
                .child_exited
                .drain_filter(|c| c.0 == pid as usize)
                .collect::<Vec<_>>()
                .pop()
            {
                process_inner.child.retain(|c| !c.ptr_eq(&child));
                Some((pid, exit_status))
            } else {
                None
            }
        };
        drop(process_inner);

        // 对用户区内存的读写可能造成 StorePageFault 而发生中断，所以需要在临界区外进行
        if let Some((cpid, exit_status)) = result {
            unsafe {
                if wstatus as usize != 0 {
                    *wstatus = exit_status << 8;
                }
            }
            return cpid;
        }

        let parent_thread: &mut Thread =
            unsafe { core::mem::transmute(cur_thread as *const Thread as usize) };
        cur_thread
            .process
            .inner
            .lock()
            .wake_callbacks
            .push(Box::new(move || {
                debug!("唤醒父进程");
                parent_thread.inner.lock().status = ThreadStatus::Ready;
            }));

        debug!("父进程睡眠");
        cur_thread.inner.lock().status = ThreadStatus::Waiting;
        cur_thread.yield_to_sched();
    }
}

unsafe fn convert_cstr_array(mut cstr_p: *const *const u8) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    while !(*cstr_p).is_null() {
        let s =
            { core::str::from_utf8_unchecked(CStr::from_ptr(*cstr_p as *const c_char).to_bytes()) };
        result.push(s.to_owned());
        cstr_p = cstr_p.add(1);
    }

    result
}

pub(super) fn sys_execve(
    pathname: *const u8,
    argv: *const *const u8,
    envp: *const *const u8,
) -> isize {
    let pathname = super::fs::normalize_path(AT_FDCWD, pathname);
    let argv = unsafe { convert_cstr_array(argv) };
    let envp = unsafe { convert_cstr_array(envp) };

    let cur_thread = get_current_thread();
    cur_thread.execve(
        &pathname,
        &(argv.iter().map(|s| s.as_str()).collect::<Vec<&str>>()),
        &(envp.iter().map(|s| s.as_str()).collect::<Vec<&str>>()),
    );
    // cur_thread.activate();

    println!("cur_thread ra = {:#x}", cur_thread.task_cx.get_ra());

    unsafe {
        let mut tmp_task_cx: *const TaskContextImpl = core::mem::transmute(1usize);
        debug_assert_eq!(sstatus::read().spp(), SPP::User);
        __switch(&mut tmp_task_cx, cur_thread.task_cx);
    }

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

pub(super) fn sys_sched_yield() -> isize {
    // get_current_thread().yield_to_sched();
    get_current_thread().switch_to(unsafe { &*SCHEDULE_THREAD });
    0
}

pub(super) fn sys_set_tid_address(tidptr: *const u32) -> isize {
    get_current_thread().inner.lock().clear_child_tid = tidptr as usize;
    get_current_thread().get_tid() as isize
}

// pub(super) fn sys_rt_sigprocmask() {}
