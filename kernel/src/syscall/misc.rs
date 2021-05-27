macro_rules! str2c {
    ($s:expr) => {{
        concat!($s, "\0")
    }};
}

static SYSNAME: &'static str = str2c!("Hecto-OS");
static NODENAME: &'static str = str2c!("None");
static RELEASE: &'static str = str2c!(env!("CARGO_PKG_VERSION"));
static VERSION: &'static str = env!("CARGO_PKG_VERSION_MAJOR");
static MACHINE: &'static str = str2c!("None");
static DOMAINNAME: &'static str = str2c!("None");

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
