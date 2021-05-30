use alloc::boxed::Box;
use core::{sync::atomic::Ordering, time::Duration};

use crate::{
    arch::cpu::{self, INTERVAL},
    get_current_thread,
    timer::TIMER,
    ThreadStatus,
};

macro_rules! str2c {
    ($s:expr) => {{
        concat!($s, "\0")
    }};
}

pub static SYSNAME: &'static str = str2c!("Hecto-OS");
pub static NODENAME: &'static str = str2c!("None");
pub static RELEASE: &'static str = str2c!(env!("CARGO_PKG_VERSION"));
pub static VERSION: &'static str = str2c!(env!("CARGO_PKG_VERSION_MAJOR"));
pub static MACHINE: &'static str = str2c!("None");
pub static DOMAINNAME: &'static str = str2c!("None");

/// UNIX Timesharing System Name
#[derive(Copy, Clone)]
#[repr(C)]
pub struct UTSName {
    sysname: [u8; 65],    /* Operating system name (e.g., "Linux") */
    nodename: [u8; 65],   /* Name within "some implementation-defined network" */
    release: [u8; 65],    /* Operating system release(e.g., "2.6.28") */
    version: [u8; 65],    /* Operating system version */
    machine: [u8; 65],    /* Hardware identifier */
    domainname: [u8; 65], /* NIS or YP domain name */
}

pub(super) fn sys_uname(buf: *mut UTSName) -> isize {
    // TODO 判断 str 长度是否超过了 65
    unsafe {
        let buf = &mut *buf;
        buf.sysname[..SYSNAME.len()].copy_from_slice(SYSNAME.as_bytes());
        buf.nodename[..NODENAME.len()].copy_from_slice(NODENAME.as_bytes());
        buf.release[..RELEASE.len()].copy_from_slice(RELEASE.as_bytes());
        buf.version[..VERSION.len()].copy_from_slice(VERSION.as_bytes());
        buf.machine[..MACHINE.len()].copy_from_slice(MACHINE.as_bytes());
        buf.domainname[..DOMAINNAME.len()].copy_from_slice(DOMAINNAME.as_bytes());
    }
    0
}

/// 时间
pub struct TimeVal {
    tv_sec: u64,  /* seconds */
    tv_usec: u64, /* microseconds */
}

/// 时区
pub struct TimeZone {
    tz_minuteswest: i32, /* minutes west of Greenwich */
    tz_dsttime: i32,     /* type of DST correction */
}

pub(super) fn sys_gettimeofday(tv: *mut TimeVal, _tz: *mut TimeZone) -> isize {
    let (tv_sec, tv_usec) = cpu::get_time();
    unsafe { *tv = TimeVal { tv_sec, tv_usec } }

    0
}

pub(super) fn sys_times(buf: *mut usize) -> isize {
    let times = &get_current_thread().process.times;
    unsafe {
        *buf.offset(0) = (times.tms_utime.load(Ordering::Acquire) / INTERVAL) as usize;
        *buf.offset(1) = (times.tms_stime.load(Ordering::Acquire) / INTERVAL) as usize;
        *buf.offset(2) = (times.tms_cutime.load(Ordering::Acquire) / INTERVAL) as usize;
        *buf.offset(3) = (times.tms_cstime.load(Ordering::Acquire) / INTERVAL) as usize;
    }

    0
}

pub(super) fn sys_nanosleep(req: *const Duration, _rem: *mut Duration) -> isize {
    let cur_thread = get_current_thread();

    TIMER.critical_section(|t| unsafe {
        t.register(
            *req + cpu::get_duration(),
            Box::new(move || {
                cur_thread.inner.lock().status = ThreadStatus::Ready;
            }),
        );
    });
    let cur_thread = get_current_thread();

    cur_thread.inner.lock().status = ThreadStatus::Waiting;
    cur_thread.yield_to_sched();

    0
}
